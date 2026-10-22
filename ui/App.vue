<script setup lang="ts">
import {onMounted,ref} from 'vue'
import Workspace from './Workspace.vue'
import {api,createSession,type Environment,type EnvironmentSession,type State} from './api'
interface Inventory {local:Environment;distributions:{name:string;running:boolean;version:number}[]|null;environments:Environment[];selected:string|null;error:string|null}
const inventory=ref<Inventory|null>(null),session=ref<EnvironmentSession|null>(null),environment=ref<Environment|null>(null),initial=ref<State|null>(null),workspace=ref<InstanceType<typeof Workspace>>(),error=ref(''),switching=ref(false),generation=ref(0)
async function enumerate(){inventory.value=await api<Inventory>('environments.list');return inventory.value}
async function select(distro?:string){
 if(switching.value)return
 if(workspace.value&&!await workspace.value.canLeave())return
 switching.value=true;error.value=''
 try{const result=await api<{environment:Environment;state:State}>('environments.select',distro?{distro}:{});session.value?.dispose();environment.value=result.environment;initial.value=result.state;session.value=createSession(result.environment.id);generation.value++;await enumerate()}
 catch(e){error.value=e instanceof Error?e.message:String(e)}finally{switching.value=false}
}
onMounted(async()=>{try{const inv=await enumerate();const saved=inv.environments.find(e=>e.id===inv.selected);await select(saved?.distro)}catch(e){error.value=String(e)}})
</script>
<template>
 <Workspace v-if="session&&initial&&environment" ref="workspace" :key="generation" :session="session" :initial="initial" :environment="environment" :distributions="inventory?.distributions||[]" :switching="switching" :environment-error="error" @switch="select" @enumerate="enumerate" />
 <main v-else><p role="status">正在连接 Tether…</p><p v-if="error" class="message error" role="alert">{{error}} <button @click="select()">打开本机环境</button></p></main>
</template>
