<script setup lang="ts">
import { computed, ref } from 'vue'
import UiIcon from './UiIcon.vue'
import SortHandle from './SortHandle.vue'
import { useSortable, moveItem } from './useSortable'
import { newRule, endpoint, type Rule } from './api'
import { clone, importRules, ruleErrors } from './editor'
const props=defineProps<{hostId:string;locked?:boolean}>()
defineEmits<{'copy-id':[id:string]}>()
const model=defineModel<Rule[]>({required:true})
const sortRoot=ref<HTMLElement>()
useSortable(sortRoot,move=>{const rows=[...model.value];if(moveItem(rows,move))model.value=rows},()=>!props.locked)
const selected=ref<string[]>([]),paste=ref(''),importError=ref(''),undo=ref<Rule[]|null>(null)
const errors=computed(()=>ruleErrors(model.value))
const incoming=computed(()=>{try{return {rows:paste.value.trim()?importRules(paste.value,props.hostId):[],error:''}}catch(e){return {rows:[],error:String(e instanceof Error?e.message:e)}}})
const importErrors=computed(()=>ruleErrors([...model.value,...incoming.value.rows]))
const invalidImport=computed(()=>incoming.value.rows.some(r=>importErrors.value[r.id]?.length))
function add(){const r=newRule(props.hostId);while(model.value.some(x=>x.port===r.port)&&r.port<65535)r.port++;model.value=[...model.value,r]}
function duplicate(r:Rule){model.value=[...model.value,{...clone(r),id:crypto.randomUUID(),name:r.name?`${r.name} 副本`:''}]}
function remove(ids:string[]){undo.value=clone(model.value);model.value=model.value.filter(r=>!ids.includes(r.id));selected.value=[]}
function applyImport(){if(incoming.value.error||invalidImport.value||!incoming.value.rows.length){importError.value='请先修正预览中的错误';return}model.value=[...model.value,...incoming.value.rows];paste.value='';importError.value=''}
defineExpose({add})
</script>
<template>
 <div ref="sortRoot" class="rules-editor">
 <div v-if="!locked" class="inline-toolbar"><button type="button" @click="add"><UiIcon name="plus" :size="16" /><span>添加映射</span></button><button type="button" :disabled="!selected.length" @click="remove(selected)"><UiIcon name="trash" :size="16" /><span>删除所选</span><span v-if="selected.length" class="count">{{selected.length}}</span></button><button v-if="undo" type="button" class="text-button" @click="model=undo;undo=null"><UiIcon name="refresh" :size="14" /><span>撤销删除</span></button><span class="toolbar-spacer"></span><span class="count-label">{{model.length}} 条映射</span></div>
 <p v-if="!model.length" class="empty compact">还没有端口映射，点击“添加映射”或批量粘贴。</p>
 <article v-for="(r,index) in model" :key="r.id" :data-rule-id="r.id" class="mapping-editor" :data-sort-id="locked?undefined:r.id" data-sort-group="draft-rules" :aria-label="`映射 ${index+1}`">
  <div class="mapping-title"><label class="check"><input v-if="!locked" v-model="selected" type="checkbox" :value="r.id" :aria-label="`选择第 ${index+1} 条映射`"><span>映射 {{index+1}}</span></label><div v-if="!locked" class="actions"><SortHandle :label="`映射 ${index+1}`" /><button type="button" class="icon-button" title="复制行" aria-label="复制行" @click="duplicate(r)"><UiIcon name="copy" :size="15" /><span class="sr-only">复制行</span></button><button type="button" class="icon-button danger" title="删除行" aria-label="删除行" @click="remove([r.id])"><UiIcon name="trash" :size="15" /><span class="sr-only">删除行</span></button></div></div>
  <div class="fields"><label>映射名称<input v-model="r.name" :placeholder="endpoint(r)"></label><label>方向<select v-model="r.direction" :disabled="locked"><option value="local">本地转发</option><option value="remote">远程转发</option><option value="dynamic">动态 SOCKS</option></select></label></div>
  <div class="mapping-fields"><label>监听地址<input v-model.trim="r.bind" :disabled="locked" :aria-invalid="!!errors[r.id]" placeholder="127.0.0.1"></label><label>监听端口<input v-model.number="r.port" :disabled="locked" type="number" min="1" max="65535" required :aria-invalid="!!errors[r.id]"></label><template v-if="r.direction!=='dynamic'"><label>目标地址<input v-model.trim="r.target" :disabled="locked" required :aria-invalid="!!errors[r.id]"></label><label>目标端口<input v-model.number="r.targetPort" :disabled="locked" type="number" min="1" max="65535" required :aria-invalid="!!errors[r.id]"></label></template></div>
  <p class="mapping-hint">{{r.direction==='local'?'监听本机端口，目标由远端 SSH 主机访问':r.direction==='remote'?'监听远端端口，目标由本机访问':'在本机提供 SOCKS 代理入口'}}；名称留空时自动生成。</p>
  <details class="mapping-extra"><summary><span>备注与规则 ID</span><UiIcon name="chevron-down" :size="14" class="summary-chevron" /></summary><label>映射备注<textarea v-model="r.notes" rows="2"></textarea></label><button type="button" class="text-button" @click="$emit('copy-id',r.id)"><UiIcon name="copy" :size="14" />复制规则 ID</button></details>
  <ul v-if="errors[r.id]" class="error inline" role="alert"><li v-for="message in errors[r.id]" :key="message">{{message}}</li></ul>
 </article>
 <details v-if="!locked" class="panel import-panel"><summary><UiIcon name="file" :size="16" /><span>批量粘贴映射</span><UiIcon name="chevron-down" :size="15" class="summary-chevron" /></summary><p class="muted">从表格粘贴：名称、方向、监听端口、目标地址、目标端口、监听地址、备注，以制表符分隔。方向为 local / remote / dynamic；也支持 JSON 数组。</p><label>待导入内容<textarea v-model="paste" rows="4" spellcheck="false" placeholder="Web&#9;local&#9;18080&#9;127.0.0.1&#9;8080"></textarea></label>
  <p v-if="incoming.error||importError" class="error" role="alert">{{incoming.error||importError}}</p>
  <ul v-if="incoming.rows.length" class="import-preview"><li v-for="(r,i) in incoming.rows" :key="i">{{r.name}} · {{endpoint(r)}}<p v-if="importErrors[r.id]" class="error">{{importErrors[r.id]!.join('；')}}</p></li></ul>
  <button type="button" :disabled="!incoming.rows.length||invalidImport" @click="applyImport"><UiIcon name="plus" :size="16" /><span>添加预览中的 {{incoming.rows.length}} 条映射</span></button>
 </details>
 </div>
</template>
