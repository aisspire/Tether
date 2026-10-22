import { type Rule, type Config, type Host, newRule, endpoint } from './api'
export const clone = <T>(value:T):T => JSON.parse(JSON.stringify(value))
export const optionKey = (s:string) => s.split(/[=\s]/)[0]!.toLowerCase()
export const parseOptions = (s:string) => s.split('\n').map(x=>x.trim()).filter(Boolean)
export function ruleName(r:Rule) { return r.name.trim() || endpoint(r) }
export function normalizeRule(r:Rule):Rule { return {...r,name:ruleName(r)} }
const wildcard = (s:string) => ['', '*', '0.0.0.0', '::'].includes(s)
export function listeningConflict(a:Rule,b:Rule) {
 return a.port===b.port && ((a.direction==='remote')===(b.direction==='remote')) && (a.direction!=='remote'||a.hostId===b.hostId) && (a.bind===b.bind||wildcard(a.bind)||wildcard(b.bind)||(['localhost','127.0.0.1','::1'].includes(a.bind)&&['localhost','127.0.0.1','::1'].includes(b.bind)))
}
export function ruleErrors(rules:Rule[]):Record<string,string[]> {
 const errors:Record<string,string[]>={}
 for (const r of rules) {
  const e:string[]=[]
  if(!Number.isInteger(r.port)||r.port<1||r.port>65535)e.push('监听端口须为 1–65535 的整数')
  if(r.direction!=='dynamic'&&(!Number.isInteger(r.targetPort)||r.targetPort<1||r.targetPort>65535))e.push('目标端口须为 1–65535 的整数')
  if(/[\s\[\]"'\\/]/.test(r.bind)||r.direction!=='dynamic'&&(!r.target||/[\s\[\]"'\\/]/.test(r.target)))e.push('地址无效，IPv6 无需方括号')
  if(rules.some(x=>x.id!==r.id&&listeningConflict(x,r)))e.push('监听地址和端口与本主机其他映射冲突')
  if(e.length)errors[r.id]=e
 }
 return errors
}
// Tab-separated rows preserve IPv6 and names with spaces; JSON supports a lossless bulk import.
export function importRules(text:string,hostId:string):Rule[] {
 if(text.trim().startsWith('[')) {
  const rows:unknown=JSON.parse(text)
  if(!Array.isArray(rows))throw Error('请粘贴映射数组')
  return rows.map((row,i)=>{
   if(!row||typeof row!=='object')throw Error(`第 ${i+1} 行不是映射对象`)
   const x=row as Partial<Rule>
   if(x.direction&&!['local','remote','dynamic'].includes(x.direction))throw Error(`第 ${i+1} 行方向无效`)
   const r={...newRule(hostId),...x,id:crypto.randomUUID(),hostId}
   if(typeof r.bind!=='string'||typeof r.target!=='string'||typeof r.name!=='string'||typeof r.notes!=='string')throw Error(`第 ${i+1} 行字段类型无效`)
   return normalizeRule(r)
  })
 }
 return text.split('\n').filter(x=>x.trim()).map((line,i)=>{
  const columns=line.split('\t').map(x=>x.trim())
  if(columns.length<3||columns.length>7)throw Error(`第 ${i+1} 行需要 3–7 列，请使用制表符分隔`)
  const [name,direction,port,target='127.0.0.1',targetPort='8080',bind='127.0.0.1',notes='']=columns
  const d=({本地:'local',远程:'remote',动态:'dynamic',SOCKS:'dynamic'} as Record<string,string>)[direction!]||direction
  if(!['local','remote','dynamic'].includes(d!))throw Error(`第 ${i+1} 行方向须为 local、remote 或 dynamic`)
  return normalizeRule({...newRule(hostId),name:name!,direction:d as Rule['direction'],port:Number(port),target,targetPort:Number(targetPort),bind,notes})
 })
}
export function pruneRules(config:Config,ids:string[]) {
 const removed=new Set(ids)
 config.rules=config.rules.filter(r=>!removed.has(r.id))
 config.plans.forEach(p=>p.ruleIds=p.ruleIds.filter(id=>!removed.has(id)))
}
export function hostChanged(a:Host,b:Host) {return (['destination','ssh','configFile','options','reconnect','hidden','timeoutSecs'] as const).some(k=>JSON.stringify(a[k])!==JSON.stringify(b[k]))}
export function browserAddress(r:Rule) {return `${r.bind.includes(':')?`[${r.bind}]`:r.bind||'*'}:${r.port}`}
