<script setup lang="ts">
import {computed,nextTick,onUnmounted,ref,watch} from 'vue'
import {useSession,type Config} from './api'
const props=defineProps<{config:Config;ask:(title:string,lines:string[])=>Promise<string>}>()
const emit=defineEmits<{refresh:[];notice:[message:string]}>()
const session=useSession(),api=session.request
interface FileResult {path:string;text:string;revision:string;readOnly:boolean;kind:string;cancelled?:boolean}
const path=ref(''),loaded=ref(''),text=ref(''),baseline=ref(''),revision=ref(''),readOnly=ref(false),busy=ref(false),error=ref(''),history=ref(false),query=ref(''),cursor=ref(-1),editor=ref<HTMLTextAreaElement>(),knownHost=ref(''),observation=ref('')
interface Directory {path:string;parent:string|null;entries:{name:string;path:string;directory:boolean}[];truncated:boolean}
const browser=ref<HTMLDialogElement>(),directory=ref<Directory|null>(null),directoryPath=ref(''),browserError=ref('')
let browseSequence=0
async function browse(p=''){const sequence=++browseSequence;browserError.value='';try{const result=await api<Directory>('files.browse',{path:p});if(sequence!==browseSequence)return;directory.value=result;directoryPath.value=result.path}catch(e){if(sequence===browseSequence)browserError.value=e instanceof Error?e.message:String(e)}}
async function chooseEntry(entry:Directory['entries'][number]){if(entry.directory){await browse(entry.path);return}if(!await protect())return;browser.value?.close();await open(entry.path,true)}
const isDirty=computed(()=>text.value!==baseline.value)
defineExpose({isDirty})
const matches=computed(()=>{const q=query.value.toLocaleLowerCase();if(!q)return [];const source=text.value.toLocaleLowerCase(),found:number[]=[];let i=0;while((i=source.indexOf(q,i))!==-1){found.push(i);i+=Math.max(1,q.length)}return found})
watch(query,()=>{cursor.value=-1})
async function run(fn:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';try{await fn()}catch(e){error.value=e instanceof Error?e.message:String(e)}finally{busy.value=false;emit('refresh')}}
async function protect(){return !isDirty.value||await props.ask('放弃当前文件的修改？',['未保存的内容将被放弃。'])==='continue'}
function accept(file:FileResult){loaded.value=file.path;path.value=file.path;text.value=file.text;baseline.value=file.text;revision.value=file.revision;readOnly.value=file.readOnly;query.value='';cursor.value=-1;observation.value='';history.value=false}
async function open(p=path.value,protectedAlready=false){if(!p.trim()||!protectedAlready&&!await protect())return;await run(async()=>accept(await api<FileResult>('files.open',{path:p.trim()})))}
async function choose(){if(session.environment!=='windows'){await browse(loaded.value?loaded.value.slice(0,loaded.value.lastIndexOf('/')):'');browser.value?.showModal();return}if(!await protect())return;await run(async()=>{const f=await api<FileResult>('files.choose');if(!f.cancelled)accept(f)})}
async function forget(p:string){await run(async()=>{await api('files.forget',{path:p});emit('notice','已移除文件历史')})}
async function save(){await run(async()=>{const r=await api<{revision:string;backup:string}>('files.write',{path:loaded.value,text:text.value,revision:revision.value});revision.value=r.revision;baseline.value=text.value;emit('notice',`已保存；备份 ${r.backup}`)})}
async function locate(delta:number){if(!matches.value.length)return;cursor.value=(cursor.value+delta+matches.value.length)%matches.value.length;await nextTick();const start=matches.value[cursor.value]!;editor.value?.focus();editor.value?.setSelectionRange(start,start+query.value.length);if(editor.value){const line=text.value.slice(0,start).split('\n').length-1;const lineHeight=parseFloat(getComputedStyle(editor.value).lineHeight)||21;editor.value.scrollTop=Math.max(0,line*lineHeight-editor.value.clientHeight/2)}}
async function known(action:string){if(action==='knownHosts.remove'){if(!await protect())return;if(await props.ask('删除当前文件中的主机指纹？',[`${knownHost.value} · ${loaded.value}`])!=='continue')return}await run(async()=>{const r=await api<{result:string}>(action,{path:loaded.value,host:knownHost.value,revision:revision.value,confirm:action==='knownHosts.remove'});if(action==='knownHosts.remove')accept(await api<FileResult>('files.open',{path:loaded.value}));observation.value=r.result||'没有匹配记录'})}
onUnmounted(()=>{text.value='';baseline.value='';query.value=''})
</script>
<template>
 <section class="panel files-panel" aria-labelledby="files-title">
  <h2 id="files-title">SSH 文件</h2>
  <form class="file-open" @submit.prevent="open()"><label>文件绝对路径<input v-model="path" autocomplete="off" spellcheck="false" placeholder="输入当前环境的文件路径后按 Enter"></label><button type="button" :disabled="busy" aria-label="选择文件" title="选择文件" @click="choose"><svg viewBox="0 0 24 24" width="19" height="19" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M3 6h7l2 3h9v11H3z"/><path d="M3 6V4h7l2 2h7v3"/></svg></button><button type="button" :aria-expanded="history" @click="history=!history">历史 ▾</button><button :disabled="busy||!path.trim()">打开</button></form>
  <div v-if="history" class="file-history" aria-label="文件历史"><p v-if="!config.sshFiles.length" class="muted">还没有成功打开的文件。</p><div v-for="p in config.sshFiles" :key="p"><button class="history-path" :disabled="busy" @click="open(p)">{{p}}</button><button :disabled="busy" :aria-label="`移除历史 ${p}`" @click="forget(p)">移除</button></div></div>
  <p v-if="busy" class="muted" role="status">正在处理文件…</p><p v-if="error" class="message error" role="alert">{{error}}</p>
  <template v-if="loaded">
   <p class="muted break">{{loaded}} <span v-if="readOnly"> · 私钥只读</span><span v-else-if="isDirty"> · 有未保存的修改</span></p>
   <div class="file-search"><label>搜索文件全文<input v-model="query" autocomplete="off" spellcheck="false" @keydown.enter.prevent="locate($event.shiftKey?-1:1)"></label><output aria-live="polite">{{cursor>=0&&matches.length?`${cursor+1} / `:''}}{{matches.length}} 处匹配</output><button :disabled="!matches.length" aria-label="上一个搜索结果" @click="locate(-1)">上一个</button><button :disabled="!matches.length" aria-label="下一个搜索结果" @click="locate(1)">下一个</button></div>
   <label>文件内容<textarea ref="editor" v-model="text" rows="18" :readonly="readOnly" autocomplete="off" autocapitalize="off" spellcheck="false"></textarea></label>
   <div class="actions"><button :disabled="busy" @click="open(loaded)">重新载入</button><button v-if="!readOnly" class="primary" :disabled="busy||!isDirty" @click="save">保存文件与备份</button></div>
   <details v-if="!readOnly" class="known-host-tools" open><summary>主机指纹查找与删除</summary><label>主机或 [主机]:端口<input v-model="knownHost" autocomplete="off"></label><div class="actions"><button :disabled="busy||!knownHost.trim()" @click="known('knownHosts.find')">查找指纹</button><button class="danger" :disabled="busy||!knownHost.trim()" @click="known('knownHosts.remove')">删除此主机记录</button></div><pre v-if="observation">{{observation}}</pre></details>
  </template>
  <p v-else class="empty compact">打开 config、known_hosts 或其他 SSH 文件。私钥仅供只读查看与搜索。</p>
  <dialog @close="browseSequence++" ref="browser" class="file-browser" aria-labelledby="file-browser-title"><div class="dialog-heading"><h2 id="file-browser-title">选择当前环境的文件</h2><button @click="browser?.close()">取消</button></div><form class="file-open" @submit.prevent="browse(directoryPath)"><label>目录路径<input v-model="directoryPath" autocomplete="off" spellcheck="false"></label><button>打开目录</button><button type="button" :disabled="!directory?.parent" @click="browse(directory!.parent!)">上一级</button></form><p v-if="browserError" class="message error" role="alert">{{browserError}}</p><p v-if="directory?.truncated" class="muted">此目录条目较多，仅显示前 2000 项；可直接输入文件路径。</p><div class="file-browser-list"><button v-for="entry in directory?.entries" :key="entry.path" @click="chooseEntry(entry)"><span>{{entry.directory?'文件夹':'文件'}}</span>{{entry.name}}</button><p v-if="directory&&!directory.entries.length" class="empty compact">此目录为空。</p></div></dialog>
 </section>
</template>
