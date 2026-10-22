<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { sshOptions, type SshOption } from './ssh-options'
import { optionKey, parseOptions } from './editor'
const model=defineModel<string[]>({required:true})
const search=ref(''), picked=ref('Port')
const common=['Port','IdentityFile','ProxyJump','ServerAliveInterval','AddressFamily']
const catalog=computed(()=>sshOptions.filter(x=>`${x.name} ${x.label} ${x.restricted||''}`.toLowerCase().includes(search.value.toLowerCase())))
watch(catalog,rows=>{if(!rows.some(x=>x.name===picked.value))picked.value=rows[0]?.name||''})
const definition=(name:string)=>sshOptions.find(x=>x.name.toLowerCase()===name.toLowerCase())
const value=(name:string)=>model.value.find(x=>optionKey(x)===name.toLowerCase())?.split('=').slice(1).join('=')||''
function set(name:string,v:string){const key=name.toLowerCase(), index=model.value.findIndex(x=>optionKey(x)===key),rows=[...model.value];if(index<0){if(v)rows.push(`${name}=${v}`)}else if(v)rows[index]=`${name}=${v}`;else rows.splice(index,1);model.value=rows}
function add(){const d=definition(picked.value);if(!d||d.restricted)return;if(!d.repeatable&&model.value.some(x=>optionKey(x)===d.name.toLowerCase()))return;model.value=[...model.value,`${d.name}=${d.choices?.[0]||''}`]}
function edit(i:number,v:string){const rows=[...model.value];rows[i]=`${model.value[i]!.split('=')[0]}=${v}`;model.value=rows}
function remove(i:number){model.value=model.value.filter((_,n)=>n!==i)}
const raw=computed({get:()=>model.value.join('\n'),set:s=>model.value=parseOptions(s)})
function inputType(d?:SshOption){return d?.numeric?'number':'text'}
</script>
<template>
 <div class="fields common-options"><label v-for="name in common" :key="name">{{ definition(name)?.label }}<select v-if="definition(name)?.choices" :value="value(name)" @change="set(name,($event.target as HTMLSelectElement).value)"><option value="">继承配置 / 默认</option><option v-for="v in definition(name)!.choices" :key="v">{{v}}</option></select><input v-else :type="inputType(definition(name))" :value="value(name)" placeholder="继承配置 / 默认" @input="set(name,($event.target as HTMLInputElement).value)"><small>{{ value(name)?'手动覆盖':'继承 SSH 配置或本机默认' }}<button v-if="value(name)" type="button" class="text-button" @click="set(name,'')">恢复继承</button></small></label></div>
 <details class="option-catalog"><summary>全部 SSH 参数 · {{sshOptions.length}} 项</summary>
  <p class="muted">目录基于 OpenSSH 10.0，实际支持以本机客户端解析结果为准。</p>
  <label>搜索参数<input v-model="search" placeholder="名称或用途，例如认证、压缩"></label>
  <p v-if="!catalog.length" class="muted">没有匹配的参数。</p><label>参数目录<select v-model="picked" size="7" aria-label="SSH 参数目录"><option v-for="d in catalog" :key="d.name" :value="d.name">{{d.name}} · {{d.label}}{{d.restricted?' · 专用设置':''}}</option></select></label>
  <p v-if="definition(picked)?.restricted" class="muted">{{definition(picked)?.restricted}}</p><p v-else-if="definition(picked)?.repeatable" class="muted">此参数允许添加多项，保留填写顺序。</p>
  <div class="actions"><button type="button" :disabled="!definition(picked)||!!definition(picked)?.restricted" @click="add">添加覆盖项</button><a :href="`https://man.openbsd.org/ssh_config#${picked}`" target="_blank" rel="noopener noreferrer">参数说明</a></div>
 </details>
 <div v-if="model.length" class="option-rows"><div v-for="(line,i) in model" :key="i" class="option-row"><label>{{definition(optionKey(line))?.label||line.split('=')[0]}} <small>{{line.split('=')[0]}} · 手动覆盖</small><select v-if="definition(optionKey(line))?.choices" :value="line.split('=').slice(1).join('=')" @change="edit(i,($event.target as HTMLSelectElement).value)"><option v-if="!definition(optionKey(line))!.choices!.includes(line.split('=').slice(1).join('='))" :value="line.split('=').slice(1).join('=')">{{line.split('=').slice(1).join('=')}}</option><option v-for="v in definition(optionKey(line))!.choices" :key="v">{{v}}</option></select><input v-else required :type="inputType(definition(optionKey(line)))" :value="line.split('=').slice(1).join('=')" @input="edit(i,($event.target as HTMLInputElement).value)"></label><button type="button" :aria-label="`移除 ${line.split('=')[0]} 覆盖`" @click="remove(i)">恢复继承</button></div></div>
 <details><summary>高级有序 -o 原文</summary><label>每行 Name=Value<textarea v-model="raw" rows="5" spellcheck="false"></textarea></label><p class="muted">与上方共用同一份配置。单值通常首次值优先，多值参数保留顺序。</p></details>
</template>
