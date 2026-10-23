<script setup lang="ts">
import OptionsEditor from './OptionsEditor.vue'
import RulesEditor from './RulesEditor.vue'
import UiIcon from './UiIcon.vue'
import { type Host, type Rule } from './api'
const host=defineModel<Host>({required:true})
const rules=defineModel<Rule[]>('rules',{required:true})
defineProps<{tab:string}>()
defineEmits<{tab:[value:string];preview:[];'copy-id':[id:string]}>()
</script>
<template>
 <div class="editor-tabs" role="group" aria-label="主机编辑内容"><button type="button" :class="{active:tab==='connection'}" :aria-pressed="tab==='connection'" @click="$emit('tab','connection')"><UiIcon name="sliders" :size="16" /><span>连接设置</span></button><button type="button" :class="{active:tab==='mappings'}" :aria-pressed="tab==='mappings'" @click="$emit('tab','mappings')"><UiIcon name="link" :size="16" /><span>端口映射 · {{rules.length}}</span></button></div>
 <section v-show="tab==='connection'" class="editor-section">
  <div class="field-grid">
   <label>主机名称<input v-model.trim="host.name" required autocomplete="off"></label>
   <label>SSH 目标 / 别名<input v-model.trim="host.destination" required placeholder="dev-server 或 admin@server" autocomplete="off"><small>使用别名时继承原有 SSH 配置。</small></label>
  </div>
  <label>备注<textarea v-model="host.notes" rows="2" placeholder="可选，用于区分用途或环境"></textarea></label>
  <OptionsEditor v-model="host.options" />
  <details class="panel detail-panel"><summary><UiIcon name="settings" :size="16" /><span>本机与连接行为</span><UiIcon name="chevron-down" :size="15" class="summary-chevron" /></summary><div class="field-grid"><label>SSH 配置路径<input v-model="host.configFile" placeholder="留空使用 OpenSSH 默认配置"></label><label>SSH 程序<input v-model.trim="host.ssh" required></label><label>认证和就绪超时（秒）<input v-model.number="host.timeoutSecs" required type="number" min="1" max="120"></label></div><div class="check-row"><label class="check"><input v-model="host.reconnect" type="checkbox">断线后重连</label><label class="check"><input v-model="host.hidden" type="checkbox">隐藏本地控制台</label></div><button type="button" class="text-button" @click="$emit('copy-id',host.id)"><UiIcon name="copy" :size="14" />复制主机 ID</button></details>
  <button type="button" class="preview-action" @click="$emit('preview')"><UiIcon name="sliders" :size="16" /><span>检查生效参数</span></button>
 </section>
 <section v-show="tab==='mappings'" class="editor-section"><RulesEditor v-model="rules" :host-id="host.id" @copy-id="$emit('copy-id',$event)" /></section>
</template>
