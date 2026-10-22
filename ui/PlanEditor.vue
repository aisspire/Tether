<script setup lang="ts">
import { computed,ref } from 'vue'
import { type Plan, type Host, type Rule, endpoint } from './api'
const plan=defineModel<Plan>({required:true})
const props=defineProps<{hosts:Host[];rules:Rule[]}>()
const search=ref('')
const visible=computed(()=>props.hosts.map(h=>({host:h,rules:props.rules.filter(r=>r.hostId===h.id&&`${h.name} ${h.destination} ${r.name} ${endpoint(r)} ${r.notes}`.toLowerCase().includes(search.value.toLowerCase()))})).filter(x=>x.rules.length))
const count=(rules:Rule[])=>rules.filter(r=>plan.value.ruleIds.includes(r.id)).length
function toggle(rules:Rule[],on:boolean){const ids=new Set(plan.value.ruleIds);rules.forEach(r=>on?ids.add(r.id):ids.delete(r.id));plan.value.ruleIds=[...ids]}
</script>
<template>
 <label>计划名称<input v-model.trim="plan.name" required></label><label>备注<textarea v-model="plan.notes" rows="2"></textarea></label>
 <label>搜索主机或映射<input v-model="search" placeholder="主机、名称或端口"></label>
 <div class="toolbar"><span>{{plan.ruleIds.length}} 条已选映射</span><button type="button" @click="toggle(visible.flatMap(x=>x.rules),true)">全选{{search?'筛选结果':''}}</button><button type="button" :disabled="!plan.ruleIds.length" @click="plan.ruleIds=[]">清空选择</button></div>
 <p v-if="!visible.length" class="empty compact">{{rules.length?'没有匹配的映射':'请先在主机中添加端口映射'}}</p>
 <fieldset v-for="entry in visible" :key="entry.host.id" class="plan-host"><legend>{{entry.host.name}}</legend><label class="check"><input type="checkbox" :checked="count(entry.rules)===entry.rules.length" :indeterminate="count(entry.rules)>0&&count(entry.rules)<entry.rules.length" @change="toggle(entry.rules,($event.target as HTMLInputElement).checked)">全选{{search?'筛选结果':'此主机'}} · {{count(entry.rules)}} / {{entry.rules.length}}</label><label v-for="r in entry.rules" :key="r.id" class="check plan-rule"><input v-model="plan.ruleIds" type="checkbox" :value="r.id"><span><strong>{{r.name}}</strong><code>{{endpoint(r)}}</code></span></label></fieldset>
 <p class="muted">新增映射不会自动加入此计划，可在这里主动选择。</p><label class="check"><input v-model="plan.autoStart" type="checkbox">每次后台启动时执行此计划</label>
</template>
