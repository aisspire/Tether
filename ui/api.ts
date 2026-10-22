import { inject, type InjectionKey } from "vue"
export interface Host { id:string; name:string; destination:string; notes:string; category:string; ssh:string; configFile:string|null; options:string[]; reconnect:boolean; hidden:boolean; timeoutSecs:number }
export interface Rule { id:string; name:string; hostId:string; direction:'local'|'remote'|'dynamic'; bind:string; port:number; target:string; targetPort:number; notes:string }
export interface Plan { id:string; name:string; ruleIds:string[]; notes:string; autoStart:boolean }
export interface Config { agentSocket?:string; version:number; hosts:Host[]; rules:Rule[]; plans:Plan[]; keyFiles:string[]; sshFiles:string[]; theme:'system'|'light'|'dark' }
export interface Group { id:string; hostId:string; state:string; rules:Rule[]; consumers:string[]; pid:number|null; error:string|null; desired:boolean; logs:string[]; args:string[]; snapshot:{host:Host; inheritedForwards:string[]; effective:Record<string,string[]>} }
export interface Prompt { id:string; groupId:string; host:string; text:string; secret:boolean }
export interface PortCheck {ruleId:string;state:string;source:string|null;process:string|null;pid:number|null;reason:string;checkedAt:number;reachable:boolean|null}
export interface State { environment:string;environmentLabel:string;portChecks:PortCheck[]; config:Config; revision:string; configError:string|null; configPath:string; groups:Group[]; prompts:Prompt[]; latency:Record<string,number|null>; externalMatches:{ruleId:string;pid:number;verified:boolean}[]; externalCheckError:string|null }
const fragment = new URLSearchParams(location.hash.slice(1))
let token = fragment.get('token') ?? sessionStorage.getItem('tether-token') ?? ''
if (token) sessionStorage.setItem('tether-token',token)
if (fragment.has('token')) history.replaceState(null,'',location.pathname)
export async function api<T=unknown>(action:string, args:Record<string,unknown>={}):Promise<T> {
 const response=await fetch('/api',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify({action,...args})})
 const result=await response.json()
 if(!response.ok || !result.ok) throw new Error(result.error ?? '操作失败')
 return result.value as T
}
export function newHost():Host {return {id:crypto.randomUUID(),name:'',destination:'',notes:'',category:'',ssh:'ssh',configFile:null,options:[],reconnect:true,hidden:true,timeoutSecs:30}}
export function newRule(hostId=''):Rule {return {id:crypto.randomUUID(),name:'',hostId,direction:'local',bind:'127.0.0.1',port:18080,target:'127.0.0.1',targetPort:8080,notes:''}}
export function endpoint(r:Rule) {return `${r.direction==='remote'?'远程':r.direction==='dynamic'?'SOCKS':'本地'} ${r.bind||'*'}:${r.port}${r.direction==='dynamic'?'':` → ${r.target}:${r.targetPort}`}`}
export const states:Record<string,string>={starting:'启动中',authentication:'等待认证',running:'运行中',reconnecting:'重连等待',failed:'失败',stopping:'停止中',stopped:'已停止',pending:'参数待应用',checking:'不可用 · 正在确认来源',unavailable:'不可用',unverified:'外部远端未确认',conflict:'监听冲突',timeout:'超时'}

export interface StartResult { ready:boolean; results:{ruleId:string; groupId?:string; state:string; reused?:boolean; error?:string}[] }
export interface Preview {host:Host; effective:Record<string,string[]>; inheritedForwards:string[]; program?:string; args?:string[]}

export interface Environment {id:string;label:string;distro?:string;user?:string}
export interface EnvironmentSession {environment:string;request:<T=unknown>(action:string,args?:Record<string,unknown>)=>Promise<T>;dispose:()=>void}
export const SessionKey:InjectionKey<EnvironmentSession>=Symbol('Tether environment')
export function createSession(environment:string):EnvironmentSession {
 let alive=true
 return {environment,dispose:()=>{alive=false},request:async<T>(action:string,args:Record<string,unknown>={})=>{
  if(!alive)throw new Error('环境会话已切换')
  const value=await api<T>(action,{...args,environment})
  if(!alive)throw new Error('环境会话已切换')
  return value
 }}
}
export function useSession(){const session=inject(SessionKey);if(!session)throw new Error('缺少环境会话');return session}
