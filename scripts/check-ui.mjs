import {spawn} from 'node:child_process'
import {mkdtemp,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import readline from 'node:readline'
import assert from 'node:assert/strict'
import {Cdp,until,delay} from './cdp.mjs'
if(process.platform!=='win32')throw new Error('Use Windows Node and an existing Edge installation')
const root=await mkdtemp(path.resolve('target/check-ui-'))
const proc=spawn(path.resolve('target/windows/debug/tether.exe'),['serve','--data-dir',root,'--oriel-stdio'],{env:{...process.env,TETHER_UI_DIR:path.resolve('dist')},stdio:['pipe','pipe','inherit']})
const iterator=readline.createInterface({input:proc.stdout})[Symbol.asyncIterator]();const desc=JSON.parse((await iterator.next()).value)
const edge=spawn(process.env.TETHER_EDGE??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',['--headless=new','--disable-gpu','--no-first-run','--remote-debugging-port=0',`--user-data-dir=${path.join(root,'profile')}`,'about:blank'],{stdio:'ignore'})
let cdp
async function click(text){await cdp.evaluate(`(()=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(text)});if(!b)throw new Error('missing button '+${JSON.stringify(text)});b.click()})()`)}
async function fill(label,value){await cdp.evaluate(`(()=>{const l=[...document.querySelectorAll('dialog label')].find(l=>l.textContent.startsWith(${JSON.stringify(label)}));const i=l?.querySelector('input,textarea,select');if(!i)throw new Error('missing input');i.value=${JSON.stringify(value)};i.dispatchEvent(new Event('input',{bubbles:true}));i.dispatchEvent(new Event('change',{bubbles:true}))})()`)}
try{
const port=await until(async()=>Number((await readFile(path.join(root,'profile/DevToolsActivePort'),'utf8')).split('\n')[0]),'Edge CDP')
const target=await until(async()=>(await(await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t=>t.type==='page'))
cdp=await new Cdp().connect(target.webSocketDebuggerUrl);await cdp.command('Page.enable');await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:800,deviceScaleFactor:1,mobile:false});await cdp.command('Page.navigate',{url:desc.pageUrl});await until(()=>cdp.evaluate(`document.body.textContent.includes('添加第一台 SSH 主机')`))
await click('添加主机');await fill('名称','隔离主机');await fill('SSH 别名或目标','fixture');await click('保存');await until(()=>cdp.evaluate(`!document.querySelector('dialog[open]')&&document.body.textContent.includes('隔离主机')`),'host save');console.log('PASS create host through UI')
await click('新增转发');await fill('名称','管理页面');await click('保存');await until(()=>cdp.evaluate(`document.body.textContent.includes('管理页面')&&!document.querySelector('dialog[open]')`),'rule save');console.log('PASS create rule through UI')
await click('计划');await click('新建计划');await fill('名称','常用服务');await cdp.evaluate(`document.querySelector('dialog fieldset input').click()`);await click('保存');await until(()=>cdp.evaluate(`document.body.textContent.includes('常用服务')&&!document.querySelector('dialog[open]')`));console.log('PASS create plan through UI')
await click('转发');const shot=await cdp.command('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,'light.png'),Buffer.from(shot.data,'base64'))
const configResponse=await fetch(`${desc.origin}/api`,{method:'POST',headers:{Authorization:`Bearer ${desc.token}`,'Content-Type':'application/json'},body:JSON.stringify({action:'status'})});const {value:s}=await configResponse.json();s.config.rules=Array.from({length:40},(_,i)=>({...s.config.rules[0],id:`long-${i}`,name:`长列表服务 ${i} · 数据库与管理后台`,port:18000+i,notes:'这是一段长备注，用于检查窄窗口下的换行与操作可到达性。'}));s.config.plans=[];s.config.theme='dark';await fetch(`${desc.origin}/api`,{method:'POST',headers:{Authorization:`Bearer ${desc.token}`,'Content-Type':'application/json'},body:JSON.stringify({action:'save',config:s.config,revision:s.revision})});await delay(1300)
await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false});await delay(200);assert.equal(await cdp.evaluate(`document.documentElement.scrollWidth<=innerWidth`),true);assert.equal(await cdp.evaluate(`document.documentElement.dataset.theme`),'dark');assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),40)
await cdp.evaluate(`document.querySelector('nav button').focus()`);assert.ok(await cdp.evaluate(`document.activeElement.tagName==='BUTTON'`));const narrow=await cdp.command('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,'dark-narrow.png'),Buffer.from(narrow.data,'base64'));console.log('PASS dark theme, 40 rules, 480px layout and keyboard focus')
assert.equal(await cdp.evaluate(`Boolean(document.querySelector('[role=alert]'))`),false)
console.log(JSON.stringify({root,screenshots:['light.png','dark-narrow.png']},null,2))
}finally{cdp?.close();edge.kill();proc.stdin.end();await until(()=>proc.exitCode!==null,'Tether shutdown',10000).catch(()=>proc.kill())}
