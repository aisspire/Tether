<script setup lang="ts">
import UiIcon from './UiIcon.vue'
import { type Prompt } from './api'
defineProps<{prompt:Prompt;count:number;inputId:string;busy:boolean;error?:string}>()
const answer=defineModel<string>({required:true})
defineEmits<{submit:[];cancel:[]}>()
</script>
<template><section class="authentication" :aria-label="`${prompt.host} 等待认证`"><div class="authentication-heading"><span class="auth-mark"><UiIcon name="key" :size="18" /></span><div><h2>{{prompt.host}} <span>等待认证</span></h2><small v-if="count>1">另有 {{count-1}} 项等待处理</small></div></div><pre>{{prompt.text}}</pre><form @submit.prevent="$emit('submit')"><label :for="inputId">回答<input :id="inputId" v-model="answer" :type="prompt.secret?'password':'text'" autocomplete="off"></label><button class="primary" :disabled="busy">提交</button><button type="button" :disabled="busy" @click="$emit('cancel')">取消连接</button></form><p v-if="error" class="error" role="alert">{{error}}</p></section></template>
