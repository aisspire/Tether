<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useSession, type Config } from './api'
import { clone } from './editor'
const api=useSession().request
interface KeyFile {path:string; publicPath:string; privateFile:boolean; publicFile:boolean; publicKey:string|null; fingerprint:string|null; error:string|null}
interface Inventory {agentStatus:{available:boolean;socket:string|null;candidates:string[];reason?:string};files:KeyFile[]; agent:string|null; agentError:string|null}
const props=defineProps<{config:Config;revision:string}>()
const emit=defineEmits<{refresh:[];notice:[message:string]}>()
const inventory=ref<Inventory|null>(null),pending=ref(''),error=ref(''),path=ref(''),search=ref(''),comments=ref<Record<string,string>>({}),socket=ref('')
const filename=(p:string)=>p.split(/[\\/]/).at(-1)||p
const identity=(key:string)=>key.trim().split(/\s+/).slice(0,2).join(' ')
const agentKeys=computed(()=>inventory.value?.agent?.trim().split('\n').filter(Boolean).map(publicKey=>{const [algorithm,,...comment]=publicKey.trim().split(/\s+/);return {publicKey,algorithm,comment:comment.join(' '),registered:inventory.value!.files.some(k=>k.publicKey&&identity(k.publicKey)===identity(publicKey))}})||[])
const files=computed(()=>(inventory.value?.files||props.config.keyFiles.map(path=>({path,publicPath:'',privateFile:false,publicFile:false,publicKey:null,fingerprint:null,error:null}))).filter(k=>`${k.path} ${k.fingerprint}`.toLowerCase().includes(search.value.toLowerCase())))
const loaded=(k:KeyFile)=>!!k.publicKey&&agentKeys.value.some(a=>identity(a.publicKey)===identity(k.publicKey!))
async function load(){inventory.value=await api<Inventory>('keys.list')}
async function run(id:string,fn:()=>Promise<void>){if(pending.value)return;pending.value=id;error.value='';try{await fn()}catch(e){error.value=e instanceof Error?e.message:String(e)}finally{pending.value='';emit('refresh')}}
async function register(generate=false){await run('register',async()=>{const p=path.value.trim();if(!p)throw Error('请输入密钥文件的绝对路径');if(generate&&p.endsWith('.pub'))throw Error('生成密钥时请输入私钥的保存路径');if(!props.config.keyFiles.includes(p)){const c=clone(props.config);c.keyFiles.push(p);await api('save',{config:c,revision:props.revision})}if(generate)await api('keys.generate',{path:p,algorithm:'ed25519'});path.value='';await load();emit('notice',generate?'密钥已生成':'密钥路径已登记')})}
async function operate(action:string,k:KeyFile){await run(k.path,async()=>{if(action==='keys.public'){const r=await api<{publicKey:string}>(action,{path:k.path});await navigator.clipboard.writeText(r.publicKey)}else await api(action,{path:k.path,...(action==='keys.comment'?{comment:comments.value[k.path]||''}:{})});await load();emit('notice',({'keys.public':'公钥已复制','agent.add':'已加载 Agent','agent.remove':'已移出 Agent','keys.passphrase':'密钥口令已修改','keys.comment':'公钥备注已保存'} as Record<string,string>)[action]!)})}
onMounted(()=>run('refresh',load))
</script>
<template>
 <div class="toolbar"><input v-model="search" aria-label="搜索密钥" placeholder="搜索文件名、路径或指纹"><button :disabled="!!pending" @click="run('refresh',load)">{{pending==='refresh'?'读取中…':'刷新密钥与 Agent'}}</button></div>
 <p v-if="error" class="message error" role="alert">{{error}}<button @click="error=''">关闭</button></p>
 <section v-if="inventory?.agentStatus&&!inventory.agentStatus.available" class="panel"><h2>连接已有 Agent</h2><p class="muted">{{inventory.agentStatus.reason}}</p><label>Agent socket 路径<input v-model="socket" list="agent-sockets" placeholder="/tmp/ssh-…/agent.…"><datalist id="agent-sockets"><option v-for="p in inventory.agentStatus.candidates" :key="p" :value="p" /></datalist></label><button :disabled="!!pending||!socket" @click="run('agent-bind',async()=>{await api('agent.bind',{path:socket});await load()})">绑定 socket</button></section>
 <form class="panel key-register" @submit.prevent="register()"><label>密钥文件的绝对路径<input v-model="path" required placeholder="C:/Users/you/.ssh/id_ed25519"></label><div class="actions"><button :disabled="!!pending">登记路径</button><button type="button" :disabled="!!pending||!path.trim()" @click="register(true)">在新路径生成 Ed25519 密钥</button></div></form>
 <section class="key-list" aria-labelledby="key-list-title" :aria-busy="pending==='refresh'"><div class="key-section-heading"><h2 id="key-list-title">已登记密钥 <small>{{config.keyFiles.length}}</small></h2></div>
  <p v-if="!inventory&&pending==='refresh'" class="empty compact" role="status">正在读取密钥…</p><p v-else-if="!config.keyFiles.length" class="empty compact">还没有登记密钥，在上方填写已有文件路径或生成新密钥。</p><p v-else-if="!files.length" class="empty compact">没有匹配的密钥。<button @click="search=''">清除搜索</button></p>
  <article v-for="k in files" :key="k.path" class="key-row" :aria-label="filename(k.path)">
   <div class="key-summary"><div class="key-title"><h3>{{filename(k.path)}}</h3><span class="key-kind">{{!inventory?'待检查':k.privateFile?(k.publicFile?'私钥 + 公钥':'私钥 · 公钥缺失'):k.publicFile?'仅公钥':'文件缺失'}}</span><span v-if="loaded(k)" class="status running">已加载 Agent</span></div><code class="key-path">{{k.path}}</code><p v-if="k.fingerprint" class="key-fingerprint">{{k.fingerprint.trim()}}</p><p v-if="k.error" class="error inline" role="alert">{{k.error||'无法读取密钥指纹'}}</p></div>
   <div class="actions key-actions"><button :disabled="!!pending||!k.publicFile" @click="operate('keys.public',k)">复制公钥</button><button v-if="!loaded(k)" :disabled="!!pending||!k.privateFile" @click="operate('agent.add',k)">加载 Agent</button><button v-else :disabled="!!pending" @click="operate('agent.remove',k)">移出 Agent</button><details class="row-menu"><summary>更多</summary><div class="actions"><button :disabled="!!pending||!k.privateFile" @click="operate('keys.passphrase',k)">修改口令</button></div><form v-if="k.privateFile" @submit.prevent="operate('keys.comment',k)"><label>公钥备注<input v-model="comments[k.path]" placeholder="输入新的公钥备注"></label><button :disabled="!!pending||comments[k.path]===undefined">保存备注</button></form><details v-if="k.publicKey"><summary>查看公钥</summary><pre>{{k.publicKey}}</pre></details></details></div>
   <p v-if="pending===k.path" class="muted" role="status">正在处理，请按提示完成认证…</p>
  </article>
 </section>
 <section class="panel agent-panel" aria-labelledby="agent-title"><div class="key-section-heading"><h2 id="agent-title">SSH Agent <small>{{agentKeys.length}} 个身份</small></h2></div><p v-if="inventory?.agentError" class="muted break" role="status">{{inventory.agentError||'Agent 未运行或没有加载身份'}}</p><p v-else-if="inventory&&!agentKeys.length" class="muted">Agent 中暂无身份。</p><article v-for="a in agentKeys" :key="identity(a.publicKey)" class="agent-row"><div class="key-title"><strong>{{a.comment||'未命名身份'}}</strong><span class="key-kind">{{a.algorithm}}</span><span class="muted">{{a.registered?'已登记文件':'仅 Agent 身份'}}</span></div><details><summary>查看公钥</summary><pre>{{a.publicKey}}</pre></details></article></section>
</template>
