<script setup lang="ts">
import { computed } from 'vue'
import type { Rule } from './api'

const props=defineProps<{rule:Rule}>()
const mapping=computed(()=>{
 const r=props.rule,listen=`${r.bind||'*'}:${r.port}`,target=`${r.target}:${r.targetPort}`
 if(r.direction==='remote')return {left:target,right:listen,arrow:'→',description:`远程转发：本机目标 ${target}，远端监听 ${listen}`}
 if(r.direction==='dynamic')return {left:listen,right:'…',arrow:'↔',description:`动态 SOCKS：本机监听 ${listen}，目标由客户端指定`}
 return {left:listen,right:target,arrow:'←',description:`本地转发：本机监听 ${listen}，远端目标 ${target}`}
})
const description=computed(()=>mapping.value.description+(!props.rule.bind?'；监听地址为空，* 表示通配地址，并非 127.0.0.1':props.rule.bind==='*'?'；* 是配置中显式指定的通配地址':''))
</script>

<template>
 <code class="rule-endpoint" :aria-label="description" :title="description"><span class="endpoint-left" aria-hidden="true">{{mapping.left}}</span><span class="endpoint-arrow" aria-hidden="true">{{mapping.arrow}}</span><span class="endpoint-right" aria-hidden="true">{{mapping.right}}</span></code>
</template>
