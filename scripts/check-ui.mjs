// Real Edge and the isolated Tether backend. No user SSH files or remote hosts are used.
import {spawn,execFileSync} from 'node:child_process'
import ssh2 from 'ssh2'
import net from 'node:net'
import crypto from 'node:crypto'
import {mkdtemp,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import readline from 'node:readline'
import assert from 'node:assert/strict'
import {Cdp,until,delay} from './cdp.mjs'
if(process.platform!=='win32')throw new Error('Use Windows Node and an existing Edge installation')
const root=await mkdtemp(path.resolve('target/check-ui-'))
const clients=new Set();let terminalAuth=false
const privateKey=crypto.generateKeyPairSync('rsa',{modulusLength:2048,privateKeyEncoding:{type:'pkcs1',format:'pem'},publicKeyEncoding:{type:'spki',format:'pem'}}).privateKey
const echo=net.createServer(socket=>{socket.on('error',()=>{});socket.pipe(socket)})
await new Promise(r=>echo.listen(0,'127.0.0.1',r))
const sshd=new ssh2.Server({hostKeys:[privateKey]},client=>{clients.add(client);client.on('error',()=>{}).on('close',()=>clients.delete(client));client.on('authentication',ctx=>ctx.username==='fixture'&&(ctx.method==='password'&&ctx.password==='fixture-secret'||terminalAuth&&ctx.method==='publickey')?ctx.accept():ctx.reject(['publickey','password']));client.on('ready',()=>client.on('tcpip',(accept,reject,info)=>{const remote=net.connect(info.destPort,'127.0.0.1',()=>{const channel=accept();channel.pipe(remote).pipe(channel)});remote.on('error',()=>reject())}))})
await new Promise(r=>sshd.listen(0,'127.0.0.1',r))
const sshConfig=path.join(root,'ssh-config')
await writeFile(sshConfig,`Host fixture\n HostName 127.0.0.1\n User fixture\n Port ${sshd.address().port}\n PubkeyAuthentication no\n PreferredAuthentications password\n StrictHostKeyChecking accept-new\n UserKnownHostsFile "${path.join(root,'known-hosts').replaceAll('\\','/')}"\n`)
const freePort=async()=>{const server=net.createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;await new Promise(r=>server.close(r));return port}
const proc=spawn(path.resolve('target/windows/debug/tether.exe'),['serve','--data-dir',root,'--oriel-stdio'],{env:{...process.env,TETHER_UI_DIR:path.resolve('dist')},stdio:['pipe','pipe','inherit']})
const iterator=readline.createInterface({input:proc.stdout})[Symbol.asyncIterator]();const desc=JSON.parse((await iterator.next()).value)
const edge=spawn(process.env.TETHER_EDGE??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',['--headless=new','--disable-gpu','--no-first-run','--remote-debugging-port=0',`--user-data-dir=${path.join(root,'profile')}`,'about:blank'],{stdio:'ignore'})
let cdp,passed=0;const exceptions=[]
async function check(name,fn){await fn();passed++;console.log(`PASS ${name}`)}
async function api(action,args={}){const r=await fetch(`${desc.origin}/api`,{method:'POST',headers:{Authorization:`Bearer ${desc.token}`,'Content-Type':'application/json'},body:JSON.stringify({action,...args})});const data=await r.json();if(!data.ok)throw Error(data.error);return data.value}
const visible=`el=>el.getClientRects().length>0`
async function click(text,scope='body',prefix=false){await cdp.evaluate(`(()=>{const b=[...document.querySelectorAll(${JSON.stringify(scope+' button')})].filter(${visible}).find(b=>${prefix?'b.textContent.trim().startsWith':'b.textContent.trim()==='}(${JSON.stringify(text)}));if(!b)throw new Error('missing button '+${JSON.stringify(text)});b.click()})()`);await delay(60)}
async function fill(label,value,scope='.editor-dialog',index=0){await cdp.evaluate(`(()=>{const labels=[...document.querySelectorAll(${JSON.stringify(scope+' label')})].filter(${visible}).filter(l=>l.textContent.trim().startsWith(${JSON.stringify(label)}));const i=labels[${index}]?.querySelector('input,textarea,select');if(!i)throw new Error('missing input '+${JSON.stringify(label)});i.value=${JSON.stringify(value)};i.dispatchEvent(new Event('input',{bubbles:true}));i.dispatchEvent(new Event('change',{bubbles:true}))})()`)}
async function hostAction(text){await cdp.evaluate(`(()=>{const menu=document.querySelector('.host-menu');if(!menu?.open)menu?.querySelector('summary')?.click()})()`);await delay(40);await click(text,'.host-menu');await cdp.evaluate(`document.querySelector('.host-menu')?.removeAttribute('open')`)}
async function summary(text,scope='.editor-dialog'){await cdp.evaluate(`(()=>{const s=[...document.querySelectorAll(${JSON.stringify(scope+' summary')})].filter(${visible}).find(s=>s.textContent.trim().startsWith(${JSON.stringify(text)}));if(!s)throw Error('missing summary '+${JSON.stringify(text)});s.click()})()`);await delay(50)}
const closed=()=>until(()=>cdp.evaluate(`!document.querySelector('.editor-dialog[open]')`),'editor save')
const escape=()=>cdp.command('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27})
async function pointerClick(selector){
 const {x,y}=await cdp.evaluate(`(()=>{const r=document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()`)
 await cdp.command('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1})
 await cdp.command('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1})
}
async function pointerGesture(start,end=start){
 await cdp.command('Input.dispatchMouseEvent',{type:'mouseMoved',...start})
 await cdp.command('Input.dispatchMouseEvent',{type:'mousePressed',...start,button:'left',clickCount:1})
 await cdp.command('Input.dispatchMouseEvent',{type:'mouseMoved',...end,buttons:1})
 await cdp.command('Input.dispatchMouseEvent',{type:'mouseReleased',...end,button:'left',clickCount:1})
 await delay(80)
}
async function dialogPoints(selector){return cdp.evaluate(`(()=>{const d=document.querySelector(${JSON.stringify(selector)}),r=d.getBoundingClientRect();return {inside:{x:r.left+8,y:r.top+8},outside:{x:Math.max(1,r.left-6),y:Math.max(1,r.top-6)}}})()`)}
async function pinnedClose(selector){
 await cdp.evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollTop=100000`);await delay(80)
 assert.equal(await cdp.evaluate(`(()=>{const d=document.querySelector(${JSON.stringify(selector)}),b=d.querySelector('.dialog-heading > button'),r=b.getBoundingClientRect(),box=d.getBoundingClientRect();return r.top>=box.top&&r.bottom<=Math.min(innerHeight,box.bottom)&&box.right-r.right<40&&b.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))})()`),true,'close stays visible at the upper right after scrolling')
}
async function screenshot(name){await writeFile(path.join(root,name),Buffer.from((await cdp.command('Page.captureScreenshot',{format:'png'})).data,'base64'))}
try{
const port=await until(async()=>Number((await readFile(path.join(root,'profile/DevToolsActivePort'),'utf8')).split('\n')[0]),'Edge CDP')
const target=await until(async()=>(await(await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t=>t.type==='page'))
cdp=await new Cdp().connect(target.webSocketDebuggerUrl);cdp.socket.addEventListener('message',e=>{const m=JSON.parse(String(e.data));if(m.method==='Runtime.exceptionThrown')exceptions.push(m.params.exceptionDetails.exception?.description||m.params.exceptionDetails.text)})
await cdp.command('Page.enable');await cdp.command('Runtime.enable');await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:840,deviceScaleFactor:1,mobile:false});await cdp.command('Page.navigate',{url:desc.pageUrl});await until(()=>cdp.evaluate(`document.body.textContent.includes('添加第一台 SSH 主机')`))
await check('bundled help, Escape and focus restoration',async()=>{await click('使用说明');await until(()=>cdp.evaluate(`document.querySelector('.help-dialog[open]')?.textContent.includes('配置与运行状态')`));await screenshot('help-light.png');await escape();await until(()=>cdp.evaluate(`!document.querySelector('.help-dialog[open]')`));await until(()=>cdp.evaluate(`document.activeElement.textContent.trim()==='使用说明'`),'help focus restoration')})
await check('host owns mappings and common forms avoid duplicate user/category fields',async()=>{assert.deepEqual(await cdp.evaluate(`[...document.querySelectorAll('nav button')].map(x=>x.textContent.trim())`),['主机','计划','密钥','设置']);await click('添加主机');await fill('主机名称','开发机 A');await fill('SSH 目标 / 别名','fixture');await summary('本机与连接行为');await fill('SSH 配置路径',sshConfig);assert.equal(await cdp.evaluate(`[...document.querySelectorAll('.editor-dialog label')].some(l=>['SSH 用户','分组'].includes(l.firstChild.textContent.trim()))`),false);await click('端口映射','.editor-dialog',true);await click('添加映射','.editor-dialog');await fill('映射名称','Web');await click('保存','.editor-dialog');await closed();const s=await api('status');assert.equal(s.config.hosts.length,1);assert.equal(s.config.rules[0].hostId,s.config.hosts[0].id);assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),1)})
await check('dialog backdrop requires both ends outside and padding stays inside',async()=>{
 for(const selector of ['.help-dialog','.environment-dialog','.editor-dialog']){
  if(selector==='.help-dialog')await click('使用说明');else if(selector==='.environment-dialog')await cdp.evaluate(`document.querySelector('.environment button').click()`);else await click('编辑主机')
  const {inside,outside}=await dialogPoints(selector)
  await pointerGesture(inside,outside);assert.equal(await cdp.evaluate(`document.querySelector('${selector}').open`),true)
  assert.equal(await cdp.evaluate(`document.querySelector('.confirm-dialog').open`),false)
  await pointerGesture(outside,inside);assert.equal(await cdp.evaluate(`document.querySelector('${selector}').open`),true)
  await pointerGesture(inside);assert.equal(await cdp.evaluate(`document.querySelector('${selector}').open`),true,'padding must not close')
  if(selector==='.help-dialog')await pinnedClose(selector)
  await pointerGesture(outside);await until(()=>cdp.evaluate(`!document.querySelector('${selector}').open`),'full backdrop click closes')
 }
 await click('编辑主机');await fill('主机名称','保留草稿');const {inside,outside}=await dialogPoints('.editor-dialog')
 await pointerGesture(inside,outside);assert.equal(await cdp.evaluate(`document.querySelector('.confirm-dialog').open`),false)
 await pointerGesture(outside);await until(()=>cdp.evaluate(`document.querySelector('.confirm-dialog').open`))
 const confirm=await dialogPoints('.confirm-dialog');await pointerGesture(confirm.inside,confirm.outside)
 assert.equal(await cdp.evaluate(`document.querySelector('.confirm-dialog').open`),true)
 await pointerGesture(confirm.outside);assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog').open`),true)
 assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog input').value`),'保留草稿')
 await click('关闭编辑','.editor-dialog');await click('放弃修改','.confirm-dialog');await closed()
})
await check('host command replaces per-mapping command and reports copy and generation errors',async()=>{
 await cdp.evaluate(`(()=>{
  window.commandClipboardDescriptor=Object.getOwnPropertyDescriptor(navigator,'clipboard');window.commandOriginalFetch=window.fetch;window.commandCopied='';
  Object.defineProperty(navigator,'clipboard',{configurable:true,value:{write:async items=>{window.commandCopied=await(await items[0].getType('text/plain')).text()},writeText:async text=>{window.commandCopied=text}}});
 })()`)
 try{
  await pointerClick('.rule .row-menu > summary')
  const labels=await cdp.evaluate(`[...document.querySelectorAll('.rule .menu-panel button')].map(b=>b.textContent.trim())`)
  assert.ok(!labels.includes('复制完整链接命令'));assert.ok(labels.includes('复制规则 ID'));await cdp.evaluate(`document.querySelector('.rule .row-menu').open=false`)
  await screenshot('host-command-light.png')
  await click('复制完整链接命令','.host-actions')
  await until(()=>cdp.evaluate(`document.querySelector('.notice')?.textContent.includes('完整链接命令已复制')`),'command copy success')
  const copied=await cdp.evaluate('window.commandCopied');assert.ok(copied.startsWith('ssh -N -T '));assert.equal((copied.match(/\s-L\s/g)||[]).length,1)
  assert.equal(await cdp.evaluate(`document.querySelector('.rule .row-menu').open`),false)
  await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false});await cdp.evaluate(`document.documentElement.dataset.theme='dark'`)
  await screenshot('host-command-dark-narrow.png')
  assert.equal(await cdp.evaluate(`document.documentElement.scrollWidth<=innerWidth`),true)
  await cdp.evaluate(`document.querySelector('.rule .row-menu').open=false;document.documentElement.dataset.theme='light'`)
  await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:840,deviceScaleFactor:1,mobile:false})
  await cdp.evaluate(`window.fetch=async (...args)=>{if(args[1]?.body&&JSON.parse(args[1].body).action==='hostCommand')return new Response(JSON.stringify({ok:false,error:'隔离命令生成失败'}));return window.commandOriginalFetch(...args)}`)
  await click('复制完整链接命令','.host-actions')
  await until(()=>cdp.evaluate(`document.querySelector('main').textContent.includes('隔离命令生成失败')`),'generation error is visible')
  assert.equal(await cdp.evaluate('window.commandCopied'),copied);assert.equal(await cdp.evaluate(`!!document.querySelector('.notice')`),false)
  await cdp.evaluate(`window.fetch=window.commandOriginalFetch;Object.defineProperty(navigator,'clipboard',{configurable:true,value:{write:async items=>{await items[0].getType('text/plain');throw Error('隔离剪贴板权限失败')}}})`)
  await click('复制完整链接命令','.host-actions')
  await until(()=>cdp.evaluate(`document.querySelector('main').textContent.includes('隔离剪贴板权限失败')`),'clipboard denial is visible')
 }finally{
  await cdp.evaluate(`window.fetch=window.commandOriginalFetch;if(window.commandClipboardDescriptor)Object.defineProperty(navigator,'clipboard',window.commandClipboardDescriptor);else delete navigator.clipboard`)
  await cdp.evaluate(`(()=>{const error=[...document.querySelectorAll('.message.error')].find(el=>el.textContent.includes('隔离剪贴板权限失败'));error?.querySelector('button')?.click()})()`)
 }
})
await check('host selection scopes normal launch, copied command and Ctrl terminal without cross-host leakage',async()=>{
 const original=(await api('status')).config,c=structuredClone(original),h=c.hosts[0],r=c.rules[0]
 c.rules.push({...r,id:'host-choice-two',name:'第二条映射',port:await freePort()})
 c.hosts.push({...h,id:'host-choice-other',name:'另一台主机'})
 c.rules.push({...r,id:'host-choice-other-rule',hostId:'host-choice-other',name:'其他主机映射',port:await freePort()})
 await api('save',{config:c,revision:(await api('status')).revision})
 await until(()=>cdp.evaluate(`document.querySelectorAll('.rule input[type=checkbox]').length===3`),'host selection fixture')
 await cdp.evaluate(`(()=>{window.scopeFetch=window.fetch;window.scopeClipboard=Object.getOwnPropertyDescriptor(navigator,'clipboard');window.scopeRequests=[];Object.defineProperty(navigator,'clipboard',{configurable:true,value:{write:async items=>window.scopeCopied=await(await items[0].getType('text/plain')).text(),writeText:async text=>window.scopeCopied=text}});window.fetch=(url,options)=>{const q=options?.body&&JSON.parse(options.body);if(q?.action==='ensure'||q?.action==='hostTerminal'){window.scopeRequests.push(q);return Promise.resolve(new Response(JSON.stringify({ok:true,value:q.action==='ensure'?{ready:true,results:[]}:{opened:true}})))}return window.scopeFetch(url,options)}})()`)
 const host=`.host[data-sort-id="${h.id}"]`,other='.host[data-sort-id="host-choice-other"]'
 const control=type=>cdp.command('Input.dispatchKeyEvent',{type,key:'Control',code:'ControlLeft',windowsVirtualKeyCode:17,modifiers:type==='keyDown'?2:0})
 const choose=selector=>cdp.evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`)
 try{
  await choose(host+' .rule[data-sort-id="host-choice-two"] input')
  await until(()=>cdp.evaluate(`document.querySelector('${host} .host-launch').title.includes('所选 1')`))
  await choose(host+' .host-copy-command');await until(()=>cdp.evaluate(`Boolean(window.scopeCopied)`))
  const copied=await cdp.evaluate('window.scopeCopied');assert.ok(copied.includes(String(c.rules[1].port)+':'));assert.ok(!copied.includes(String(r.port)+':'))
  await choose(host+' .host-launch');await until(()=>cdp.evaluate(`window.scopeRequests.length===1`))
  const normal=await cdp.evaluate('window.scopeRequests[0]');assert.deepEqual(normal.request.ruleIds,['host-choice-two']);assert.equal(normal.request.includeInherited,false)
  await until(()=>cdp.evaluate(`!document.querySelector('${host} .host-launch').disabled`))
  await control('keyDown');assert.equal(await cdp.evaluate(`document.querySelector('${host} .host-launch').classList.contains('external-launch')`),true)
  assert.match(await cdp.evaluate(`document.querySelector('${host} .host-launch').title`),/外部终端.*所选 1/)
  assert.equal(await cdp.evaluate(`getComputedStyle(document.querySelector('${host} .host-launch'),'::after').animationName`),'terminal-pulse')
  await screenshot('host-ctrl-light.png')
  await cdp.command('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]})
  assert.equal(await cdp.evaluate(`getComputedStyle(document.querySelector('${host} .host-launch'),'::after').animationName`),'none')
  await cdp.command('Emulation.setEmulatedMedia',{features:[]})
  await choose(host+' .host-launch');await until(()=>cdp.evaluate(`window.scopeRequests.length===2`));await control('keyUp')
  const terminal=await cdp.evaluate('window.scopeRequests[1]');assert.equal(terminal.action,'hostTerminal');assert.deepEqual(terminal.ruleIds,normal.request.ruleIds);assert.equal(terminal.hostId,h.id)
  await until(()=>cdp.evaluate(`!document.querySelector('${host} .host-launch').disabled`))
  assert.equal(await cdp.evaluate(`document.querySelector('${host} .host-launch').classList.contains('external-launch')`),false)
  await control('keyDown');await cdp.evaluate(`window.dispatchEvent(new Event('blur'))`)
  assert.equal(await cdp.evaluate(`document.querySelector('${host} .host-launch').classList.contains('external-launch')`),false);await control('keyUp')
  await choose(host+' .rule[data-sort-id="host-choice-two"] input');await choose(other+' .rule input')
  await until(()=>cdp.evaluate(`document.querySelector('${host} .host-launch').title.includes('全部 2')`))
  await cdp.evaluate('window.scopeCopied=""');await choose(host+' .host-copy-command');await until(()=>cdp.evaluate(`Boolean(window.scopeCopied)`))
  assert.equal(((await cdp.evaluate('window.scopeCopied')).match(/\s-L\s/g)||[]).length,2)
  await choose(host+' .host-launch');await until(()=>cdp.evaluate(`window.scopeRequests.length===3`))
  assert.deepEqual((await cdp.evaluate('window.scopeRequests[2]')).request.ruleIds,[r.id,'host-choice-two'])
  await until(()=>cdp.evaluate(`!document.querySelector('${host} .host-launch').disabled`))
  await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false});await cdp.evaluate(`document.documentElement.dataset.theme='dark'`);await control('keyDown');await screenshot('host-ctrl-dark-narrow.png');assert.equal(await cdp.evaluate(`document.documentElement.scrollWidth<=innerWidth`),true);await control('keyUp')
 }finally{
  await control('keyUp');await cdp.evaluate(`window.fetch=window.scopeFetch;if(window.scopeClipboard)Object.defineProperty(navigator,'clipboard',window.scopeClipboard);else delete navigator.clipboard;document.documentElement.dataset.theme='light'`)
  await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:840,deviceScaleFactor:1,mobile:false})
  await api('save',{config:original,revision:(await api('status')).revision});await until(()=>cdp.evaluate(`document.querySelectorAll('.registered').length===1`))
 }
})
await check('row menus dismiss outside, switch exclusively, close on Escape and preserve actions',async()=>{
 const host='.host-menu',rule='.rule .row-menu'
 const open=selector=>until(()=>cdp.evaluate(`document.querySelector(${JSON.stringify(selector)}).open`),'row menu open')
 const shut=selector=>until(()=>cdp.evaluate(`!document.querySelector(${JSON.stringify(selector)}).open`),'row menu closed')
 await pointerClick(host+' > summary');await open(host)
 await pointerClick('main > header h1');await shut(host)
 await pointerClick(rule+' > summary');await open(rule)
 await pointerClick('main > header h1');await shut(rule)
 await pointerClick(host+' > summary');await open(host)
 // The host popup covers the rule's summary at this width. Use actual keyboard
 // activation to switch menus instead of clicking an obscured control.
 await cdp.evaluate(`document.querySelector('.rule .row-menu > summary').focus()`)
 await cdp.command('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',text:'\r',unmodifiedText:'\r',windowsVirtualKeyCode:13})
 await cdp.command('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13})
 await open(rule);await shut(host)
 // Keyboard activation has no pointerdown; toggle must still close the previous menu.
 await cdp.evaluate(`document.querySelector('.host-menu > summary').focus()`)
 await cdp.command('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',text:'\r',unmodifiedText:'\r',windowsVirtualKeyCode:13})
 await cdp.command('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13})
 await open(host);await shut(rule)
 await cdp.evaluate(`document.querySelector('.host-menu button').focus()`)
 await escape();await shut(host)
 assert.equal(await cdp.evaluate(`document.activeElement===document.querySelector('.host-menu > summary')`),true)
 await pointerClick(rule+' > summary');await open(rule)
 await escape();await shut(rule)
 assert.equal(await cdp.evaluate(`document.activeElement===document.querySelector('.rule .row-menu > summary')`),true)
 await pointerClick(host+' > summary');await open(host)
 await pointerClick(host+' > summary');await shut(host)
 await pointerClick(rule+' > summary');await open(rule)
 await click('编辑映射',rule);await shut(rule)
 await until(()=>cdp.evaluate(`!!document.querySelector('.editor-dialog[open]')`),'mapping editor opens')
 await click('取消','.editor-dialog');await closed()
 // Ordinary disclosure panels are not menus and must remain open on outside clicks.
 await cdp.evaluate(`document.querySelector('.temporary-options > summary').click()`)
 await pointerClick('main > header h1')
 assert.equal(await cdp.evaluate(`document.querySelector('.temporary-options').open`),true)
 await cdp.evaluate(`document.querySelector('.temporary-options > summary').click()`)
})
await check('TCP latency uses a visible success color and renders the measured result',async()=>{await cdp.evaluate(`document.querySelector('.host-menu > summary').click()`);await delay(60);await screenshot('host-menu-light.png');await cdp.evaluate(`document.querySelector('.host-menu')?.removeAttribute('open')`);await hostAction('测试 TCP 响应');await until(()=>cdp.evaluate(`document.querySelector('.latency')?.classList.contains('measured')`));assert.match(await cdp.evaluate(`document.querySelector('.latency').textContent`),/\d+(\.\d+)? ms/);assert.equal(await cdp.evaluate(`getComputedStyle(document.querySelector('.latency')).color===getComputedStyle(document.querySelector('h2')).color`),false);assert.equal(await cdp.evaluate(`document.querySelector('.external')===null`),true);await click('编辑主机');await click('端口映射','.editor-dialog',true);assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog').textContent.includes('网页地址')`),false);await click('取消','.editor-dialog');await closed()})
await check('keys load automatically as file rows with public-only and missing states',async()=>{const privatePath=path.join(root,'ui-identity');execFileSync('ssh-keygen',['-q','-t','ed25519','-N','','-f',privatePath]);const s=await api('status');s.config.keyFiles=[privatePath,privatePath+'.pub',path.join(root,'missing-key')];await api('save',{config:s.config,revision:s.revision});await delay(1200);await click('密钥','nav');await until(()=>cdp.evaluate(`document.querySelectorAll('.key-row').length===3&&document.querySelector('.key-fingerprint')?.textContent.includes('SHA256:')`));assert.equal(await cdp.evaluate(`document.querySelectorAll('.key-list .key-kind')[1].textContent`),'仅公钥');assert.equal(await cdp.evaluate(`document.querySelectorAll('.key-list .key-kind')[2].textContent`),'文件缺失');assert.equal(await cdp.evaluate(`document.querySelectorAll('.key-list .key-row')[1].querySelectorAll('button')[1].disabled`),true);assert.equal(await cdp.evaluate(`document.querySelector('.key-list').textContent.includes('"privateFile"')`),false);await screenshot('keys-light.png');await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false});await cdp.evaluate(`document.documentElement.dataset.theme='dark'`);assert.equal(await cdp.evaluate(`document.documentElement.scrollWidth<=innerWidth`),true);await screenshot('keys-dark-narrow.png');await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:840,deviceScaleFactor:1,mobile:false});await cdp.evaluate(`document.documentElement.dataset.theme='light'`);await click('主机','nav')})
await check('key menus preserve inputs and nested details, and dismiss outside at narrow widths',async()=>{
 await click('密钥','nav')
 await until(()=>cdp.evaluate(`!!document.querySelector('.key-row .menu-public-key')`),'key menu content')
 await cdp.evaluate(`document.querySelector('.key-row .row-menu > summary').click()`)
 await fill('公钥备注','菜单内草稿','.key-row .menu-panel')
 await pointerClick('.key-row .menu-panel input')
 assert.equal(await cdp.evaluate(`document.querySelector('.key-row .row-menu').open`),true)
 await cdp.evaluate(`document.querySelector('.key-row .menu-public-key > summary').click()`)
 assert.equal(await cdp.evaluate(`document.querySelector('.key-row .row-menu').open&&document.querySelector('.key-row .menu-public-key').open`),true)
 assert.equal(await cdp.evaluate(`document.querySelector('.key-row .menu-panel input').value`),'菜单内草稿')
 await escape()
 await until(()=>cdp.evaluate(`!document.querySelector('.key-row .row-menu').open`),'key Escape')
 assert.equal(await cdp.evaluate(`document.activeElement===document.querySelector('.key-row .row-menu > summary')`),true)
 await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false})
 await cdp.evaluate(`document.querySelector('.key-row .row-menu > summary').click()`)
 await pointerClick('main > header h1')
 assert.equal(await cdp.evaluate(`!document.querySelector('.key-row .row-menu').open`),true)
 await cdp.command('Emulation.setDeviceMetricsOverride',{width:1180,height:840,deviceScaleFactor:1,mobile:false})
 await click('主机','nav')
})
await check('settings open and register files, locate full text, protect dirty buffers, and keep private keys read-only',async()=>{
 const file=path.join(root,'plain-file');await writeFile(file,'first match\nsecond match\nlast\n');await click('设置','nav');await fill('文件绝对路径',file,'.files-panel');await click('打开','.files-panel');await until(()=>cdp.evaluate(`document.querySelector('.files-panel textarea')?.value.includes('second match')`));
 assert.ok((await api('status')).config.sshFiles.includes(file));assert.ok(!(await api('status')).config.keyFiles.includes(file));await fill('搜索文件全文','match','.files-panel');assert.equal(await cdp.evaluate(`document.querySelector('.file-search output').textContent.includes('2 处匹配')`),true);await click('下一个','.file-search');assert.equal(await cdp.evaluate(`document.querySelector('.files-panel textarea').selectionStart`),6);await click('下一个','.file-search');assert.equal(await cdp.evaluate(`document.querySelector('.files-panel textarea').selectionStart`),19);await screenshot('settings-light.png');
 await fill('文件内容','edited match','.files-panel');await click('历史');await click('移除','.file-history');await until(async()=>!(await api('status')).config.sshFiles.includes(file));assert.equal(await readFile(file,'utf8'),'first match\nsecond match\nlast\n');
 await click('主机','nav');await until(()=>cdp.evaluate(`!!document.querySelector('.confirm-dialog[open]')`));await click('继续编辑','.confirm-dialog');assert.equal(await cdp.evaluate(`document.querySelector('.files-panel textarea').value`),'edited match');
 await fill('文件绝对路径',path.join(root,'ui-identity'),'.files-panel');await click('打开','.files-panel');await click('继续','.confirm-dialog');await until(()=>cdp.evaluate(`document.querySelector('.files-panel textarea')?.readOnly`));await fill('搜索文件全文','PRIVATE KEY','.files-panel');assert.equal(await cdp.evaluate(`document.querySelector('.file-search output').textContent.includes('2 处匹配')`),true);assert.equal(await cdp.evaluate(`[...document.querySelectorAll('.files-panel button')].some(b=>b.textContent.includes('保存文件'))`),false);
 await click('主机','nav');
})
await check('matching terminal SSH updates its existing mapping without stop controls',async()=>{const s=await api('status');const port=await freePort();s.config.hosts.push({...s.config.hosts[0],id:'terminal-ui',name:'终端主机'});s.config.rules.push({...s.config.rules[0],id:'terminal-ui-rule',hostId:'terminal-ui',name:'终端映射',port,targetPort:echo.address().port});await api('save',{config:s.config,revision:s.revision});terminalAuth=true;
 const external=spawn('ssh',['-NT','-F',sshConfig,'-o','PubkeyAuthentication=yes','-o','PreferredAuthentications=publickey','-i',path.join(root,'ui-identity'),'-L',`127.0.0.1:${port}:127.0.0.1:${echo.address().port}`,'fixture'],{stdio:'ignore'});
 try{await until(async()=>{try{const socket=net.connect(port,'127.0.0.1');return await new Promise(r=>{socket.once('connect',()=>{socket.destroy();r(true)});socket.once('error',()=>r(false))})}catch{return false}},'terminal listener',20000);await api('test',{hostId:'terminal-ui'});await until(()=>cdp.evaluate(`[...document.querySelectorAll('.registered')].find(r=>r.textContent.includes('终端映射'))?.querySelector('.status').textContent.includes('Windows')`));assert.equal(await cdp.evaluate(`[...document.querySelectorAll('.registered')].find(r=>r.textContent.includes('终端映射')).querySelector('button[aria-label="启动 / 复用"]').disabled`),true);assert.equal(await cdp.evaluate(`[...document.querySelectorAll('.host')].find(h=>h.querySelector('h2')?.textContent.includes('终端主机')).querySelector('.host-heading .danger').disabled`),true);await screenshot('terminal-light.png');external.kill();await until(()=>external.exitCode!==null||external.signalCode!==null,'external exit');await api('test',{hostId:'terminal-ui'});await until(()=>cdp.evaluate(`[...document.querySelectorAll('.registered')].find(r=>r.textContent.includes('终端映射'))?.querySelector('.status').textContent==='未启动'`),'terminal state cleared',20000)}finally{external.kill();terminalAuth=false}
 const end=await api('status');end.config.hosts=end.config.hosts.filter(h=>h.id!=='terminal-ui');end.config.rules=end.config.rules.filter(r=>r.id!=='terminal-ui-rule');await api('save',{config:end.config,revision:end.revision});await until(()=>cdp.evaluate(`document.querySelectorAll('.host').length===1`));})
await check('structured SSH catalog and raw options share one source',async()=>{await click('编辑主机');await summary('全部 SSH 参数');await fill('搜索参数','Compression');await fill('参数目录','Compression');await click('添加覆盖项','.editor-dialog');await fill('数据压缩','yes');await summary('高级有序');assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog textarea[spellcheck=false]').value`),'Compression=yes');await click('保存','.editor-dialog');await closed();assert.deepEqual((await api('status')).config.hosts[0].options,['Compression=yes'])})
await check('bulk import previews errors and saves valid rows atomically',async()=>{await click('编辑主机');await click('端口映射','.editor-dialog',true);await summary('批量粘贴');await fill('待导入内容','重复\tlocal\t18080\t127.0.0.1\t8080');assert.equal(await cdp.evaluate(`document.querySelector('.import-preview').textContent.includes('冲突')`),true);assert.equal(await cdp.evaluate(`[...document.querySelectorAll('.import-panel button')].find(b=>b.textContent.includes('添加预览')).disabled`),true);await fill('待导入内容','数据库\tlocal\t15432\t127.0.0.1\t5432\nSOCKS\tdynamic\t11080');await click('添加预览中的','.editor-dialog',true);await click('保存','.editor-dialog');await closed();assert.equal((await api('status')).config.rules.length,3)})
await check('duplicate mapping conflicts stay editable and deletion can be undone',async()=>{await click('编辑主机');await click('端口映射','.editor-dialog',true);await click('复制行','.mapping-editor');await click('保存','.editor-dialog');assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog[open]')!==null`),true);assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog').textContent.includes('请修正映射行')`),true);await cdp.evaluate(`[...document.querySelectorAll('.mapping-editor')].at(-1).querySelector('button.danger').click()`);await click('撤销删除','.editor-dialog');assert.equal(await cdp.evaluate(`document.querySelectorAll('.mapping-editor').length`),4);await cdp.evaluate(`[...document.querySelectorAll('.mapping-editor')].at(-1).querySelector('button.danger').click()`);await click('保存','.editor-dialog');await closed()})
await check('unsaved edits offer continue, discard and save with keyboard',async()=>{await click('编辑主机');await fill('主机名称','未保存');await escape();await until(()=>cdp.evaluate(`!!document.querySelector('.confirm-dialog[open]')`));await click('继续编辑','.confirm-dialog');assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog input').value`),'未保存');await escape();await click('放弃修改','.confirm-dialog');await closed();assert.equal((await api('status')).config.hosts[0].name,'开发机 A');await click('编辑主机');await fill('备注','键盘保存');await cdp.evaluate(`window.dispatchEvent(new KeyboardEvent('keydown',{key:'s',ctrlKey:true,bubbles:true,cancelable:true}))`);await closed();assert.equal((await api('status')).config.hosts[0].notes,'键盘保存')})
await check('optimistic revision conflict preserves draft and external changes',async()=>{await click('编辑主机');await fill('主机名称','不能覆盖外部修改');const s=await api('status');s.config.hosts[0].notes='外部修改';await api('save',{config:s.config,revision:s.revision});await click('保存','.editor-dialog');await until(()=>cdp.evaluate(`document.querySelector('.editor-dialog [role=alert]')?.textContent.includes('配置')`));assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog input').value`),'不能覆盖外部修改');assert.equal((await api('status')).config.hosts[0].notes,'外部修改');await click('取消','.editor-dialog');await click('放弃修改','.confirm-dialog');await closed()})
await check('temporary forwarding authenticates inside editor and stop removes history',async()=>{await hostAction('临时转发');await fill('映射名称','临时认证');const port=await freePort();await fill('监听端口',String(port));await fill('目标端口',String(echo.address().port));await click('启动临时转发','.editor-dialog');await until(()=>cdp.evaluate(`!!document.querySelector('#editor-auth-answer')`),'visible authentication inside modal');await fill('回答','fixture-secret','.editor-dialog .authentication');await click('提交','.editor-dialog .authentication');await closed();await until(async()=>(await api('status')).groups.some(g=>g.state==='running'));assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),4);await summary('当前连接','main');await click('停止连接','main',true);await until(()=>cdp.evaluate(`document.querySelector('.confirm-dialog[open]')?.textContent.includes('临时认证')`));await click('继续','.confirm-dialog');await until(async()=>(await api('status')).groups.length===0);await until(()=>cdp.evaluate(`document.querySelectorAll('.registered').length===3`));assert.equal(await cdp.evaluate(`document.querySelectorAll('.connection-details').length`),0)})
await check('edited running mapping shows pending and explicitly restarts its connection',async()=>{const port=await freePort();await click('编辑主机');await click('端口映射','.editor-dialog',true);await fill('监听端口',String(port));await click('保存','.editor-dialog');await closed();await click('启动 / 复用','.registered');await until(()=>cdp.evaluate(`!!document.querySelector('#auth-answer')`));await fill('回答','fixture-secret','main .authentication');await click('提交','main .authentication');await until(async()=>(await api('status')).groups.some(g=>g.state==='running'));const old=(await api('status')).groups.find(g=>g.state==='running');await click('编辑主机');await click('端口映射','.editor-dialog',true);await fill('目标端口',String(echo.address().port));await click('保存','.editor-dialog');await closed();assert.equal(await cdp.evaluate(`document.querySelector('.registered .status').textContent`),'参数待应用');await summary('当前连接','main');await click('应用参数并重启');await click('继续','.confirm-dialog');await until(()=>cdp.evaluate(`!!document.querySelector('#auth-answer')`));await fill('回答','fixture-secret','main .authentication');await click('提交','main .authentication');await until(async()=>(await api('status')).groups.some(g=>g.state==='running'&&g.id!==old.id&&g.rules.some(r=>r.targetPort===echo.address().port)));await new Promise((resolve,reject)=>{const socket=net.connect(port,'127.0.0.1',()=>socket.write('ui-forward'));socket.setTimeout(5000,()=>socket.destroy(Error('echo timeout')));socket.once('data',data=>{try{assert.equal(data.toString(),'ui-forward');socket.end();resolve()}catch(e){reject(e)}});socket.on('error',reject)});await click('停止全部托管');await click('继续','.confirm-dialog');await until(async()=>(await api('status')).groups.length===0)})
await check('plan groups mappings by host and saves explicit member IDs',async()=>{await click('添加主机');await fill('主机名称','测试机 B');await fill('SSH 目标 / 别名','fixture-b');await click('端口映射','.editor-dialog',true);await click('添加映射','.editor-dialog');await fill('映射名称','API');await fill('监听端口','19000');await click('保存','.editor-dialog');await closed();await click('计划','nav');await click('新建计划');await fill('计划名称','日常开发');assert.equal(await cdp.evaluate(`document.querySelectorAll('.plan-host').length`),2);await cdp.evaluate(`document.querySelector('.plan-host input').click()`);await screenshot('plan-editor-light.png');await click('保存','.editor-dialog');await closed();const s=await api('status');assert.equal(s.config.plans[0].ruleIds.length,3);assert.equal(await cdp.evaluate(`document.querySelector('.host-heading').textContent.includes('1 台主机 · 3 条映射')`),true);await screenshot('plans-light.png')})
await check('new host mappings never auto-enroll in existing plans',async()=>{const old=(await api('status')).config.plans[0].ruleIds;await click('主机','nav');await click('添加映射');await fill('映射名称','后续新增','.mapping-editor:last-of-type');await fill('监听端口','19999', '.mapping-editor:last-of-type');await click('保存','.editor-dialog');await closed();const s=await api('status');assert.equal(s.config.rules.length,5);assert.deepEqual(s.config.plans[0].ruleIds,old);await screenshot('hosts-light.png')})
await check('search finds mapping ports and state filter is explicit',async()=>{await cdp.evaluate(`(()=>{const i=document.querySelector('input[aria-label="搜索主机或映射"]');i.value='15432';i.dispatchEvent(new Event('input',{bubbles:true}));})()`);await delay(50);assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),1);await cdp.evaluate(`(()=>{const i=document.querySelector('input[aria-label="搜索主机或映射"]');i.value='';i.dispatchEvent(new Event('input',{bubbles:true}));const s=document.querySelector('select[aria-label="筛选映射状态"]');s.value='running';s.dispatchEvent(new Event('change',{bubbles:true}));})()`);await delay(50);assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),0);await click('清除筛选')})
await check('host deletion previews plan references and cascades registration only',async()=>{await hostAction('删除主机');await until(()=>cdp.evaluate(`document.querySelector('.confirm-dialog[open]')?.textContent.includes('日常开发')`));await click('删除登记，保留运行连接','.confirm-dialog');await until(async()=>(await api('status')).config.hosts.length===1);const s=await api('status');assert.equal(s.config.rules.length,1);assert.equal(s.config.plans[0].ruleIds.length,0)})
await check('dark 480px long lists and bulk editor have no viewport overflow',async()=>{const s=await api('status');s.config.rules=Array.from({length:40},(_,i)=>({...s.config.rules[0],id:`long-${i}`,name:`服务 ${i} · 数据库与管理后台`,port:18000+i,notes:'长备注用于检查窄窗口的换行与操作可到达性。'}));s.config.plans=[];s.config.theme='dark';await api('save',{config:s.config,revision:s.revision});await delay(1400);await cdp.command('Emulation.setDeviceMetricsOverride',{width:480,height:800,deviceScaleFactor:1,mobile:false});assert.equal(await cdp.evaluate(`document.documentElement.scrollWidth<=innerWidth`),true);assert.equal(await cdp.evaluate(`document.documentElement.dataset.theme`),'dark');assert.equal(await cdp.evaluate(`document.querySelectorAll('.registered').length`),40);await screenshot('dark-narrow.png');await cdp.evaluate(`document.querySelector('.host-menu > summary').click()`);await delay(60);await screenshot('host-menu-dark-narrow.png');await cdp.evaluate(`document.querySelector('.host-menu')?.removeAttribute('open')`);await click('编辑主机');await click('端口映射','.editor-dialog',true);assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog').scrollWidth<=document.querySelector('.editor-dialog').clientWidth`),true);await screenshot('editor-dark-narrow.png');await click('取消','.editor-dialog');await closed();await click('使用说明');assert.equal(await cdp.evaluate(`document.querySelector('.help-dialog').scrollWidth<=document.querySelector('.help-dialog').clientWidth`),true);await screenshot('help-dark-narrow.png');await escape()})
await check('mapping drag scrolls both edges, cancels cleanly and saves stable IDs',async()=>{
 const before=await api('status'),ids=before.config.rules.map(r=>r.id)
 await click('编辑主机');await click('端口映射','.editor-dialog',true);await pinnedClose('.editor-dialog')
 // Dispatch drag events against real DOM geometry; the animation loop must keep
 // scrolling even when the pointer stays still near either edge.
 await cdp.evaluate(`(()=>{const d=document.querySelector('.editor-dialog');d.scrollTop=300;window.sortData=new DataTransfer();const h=d.querySelector('.sort-handle'),r=d.getBoundingClientRect();h.dispatchEvent(new DragEvent('dragstart',{bubbles:true,cancelable:true,dataTransfer:window.sortData,clientX:r.left+50,clientY:r.top+140}));d.querySelector('.rules-editor').dispatchEvent(new DragEvent('dragover',{bubbles:true,cancelable:true,dataTransfer:window.sortData,clientX:r.left+50,clientY:r.top+95}))})()`)
 await until(()=>cdp.evaluate(`document.querySelector('.editor-dialog').scrollTop<200`),'upward edge scroll')
 await cdp.evaluate(`(()=>{const d=document.querySelector('.editor-dialog'),r=d.getBoundingClientRect();d.querySelector('.rules-editor').dispatchEvent(new DragEvent('dragover',{bubbles:true,cancelable:true,dataTransfer:window.sortData,clientX:r.left+50,clientY:r.bottom-50}))})()`)
 await until(()=>cdp.evaluate(`document.querySelector('.editor-dialog').scrollTop>500`),'downward edge scroll')
 await cdp.evaluate(`document.dispatchEvent(new DragEvent('dragend',{bubbles:true}))`)
 const stopped=await cdp.evaluate(`document.querySelector('.editor-dialog').scrollTop`);await delay(150)
 assert.equal(await cdp.evaluate(`document.querySelector('.editor-dialog').scrollTop`),stopped)
 assert.equal(await cdp.evaluate(`!!document.querySelector('[data-sorting],[data-sort-drop]')`),false)
 assert.equal(await cdp.evaluate(`document.querySelector('.confirm-dialog').open`),false)
 assert.deepEqual((await api('status')).config.rules.map(r=>r.id),ids,'cancelled drag does not save')
 await cdp.evaluate(`(()=>{const d=document.querySelector('.editor-dialog');d.scrollTop=0;const rows=d.querySelectorAll('.mapping-editor'),target=rows[2];target.scrollIntoView({block:'start'});const r=target.getBoundingClientRect();rows[0].querySelector('.sort-handle').dispatchEvent(new DragEvent('dragstart',{bubbles:true,cancelable:true,dataTransfer:new DataTransfer(),clientX:r.left+30,clientY:r.top+30}));target.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,clientX:r.left+30,clientY:r.top+20}))})()`)
 const expected=[ids[1],ids[0],...ids.slice(2)]
 assert.deepEqual(await cdp.evaluate(`[...document.querySelectorAll('.mapping-editor')].map(r=>r.dataset.ruleId)`),expected)
 assert.deepEqual((await api('status')).config.rules.map(r=>r.id),ids,'draft order waits for save')
 await click('保存','.editor-dialog');await closed();assert.deepEqual((await api('status')).config.rules.map(r=>r.id),expected)
 const snapshot=await api('status')
 await cdp.evaluate(`document.querySelector('.registered .sort-handle').focus()`)
 await cdp.command('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowDown',code:'ArrowDown',windowsVirtualKeyCode:40,modifiers:1})
 await until(async()=>(await api('status')).config.rules[0].id===ids[0],'keyboard sorting saves')
 const sorted=await api('status');assert.deepEqual([...sorted.config.rules].sort((a,b)=>a.id.localeCompare(b.id)),[...snapshot.config.rules].sort((a,b)=>a.id.localeCompare(b.id)))
 await cdp.command('Page.reload');await until(()=>cdp.evaluate(`document.querySelector('.registered')?.dataset.sortId===${JSON.stringify(ids[0])}`),'order persists after reload')
})
await check('collapsed hosts persist after refresh and focus remains keyboard accessible',async()=>{await cdp.evaluate(`document.querySelector('.host-title .disclosure').click()`);await cdp.evaluate('window.tetherReloadMarker=true');await cdp.command('Page.reload');await until(()=>cdp.evaluate(`!window.tetherReloadMarker&&document.querySelector('.host-title .disclosure')?.getAttribute('aria-expanded')==='false'`));await cdp.evaluate(`document.querySelector('nav button').focus()`);assert.equal(await cdp.evaluate(`document.activeElement.tagName`),'BUTTON')})
await check('environment replacement discards delayed old responses',async()=>{
 await until(()=>cdp.evaluate(`document.querySelector('.environment button')?.textContent.trim()==='Windows'&&!document.querySelector('.environment button').disabled`),'local environment settled after reload')
 await cdp.evaluate(`(()=>{const original=window.fetch.bind(window);let armed=true;window.fetch=async(url,options)=>{if(armed&&JSON.parse(options?.body||'{}').action==='status'){armed=false;const response=await original(url,options);const value=await response.json();value.value.config.hosts[0].name='旧环境延迟返回';await new Promise(r=>setTimeout(r,2200));return new Response(JSON.stringify(value),{status:200,headers:{'Content-Type':'application/json'}})}return original(url,options)}})()`);
 await click('刷新');await click('Windows','.environment');await click('Windows','.environment-options');await delay(3000);assert.equal(await cdp.evaluate(`document.body.textContent.includes('旧环境延迟返回')`),false);
})
await check('WSL file chooser browses native paths, cancels without history and opens a selected file',async()=>{
 const inv=await api('environments.list');if(!inv.distributions?.length)return;const distro=inv.distributions[0].name;const file=path.join(root,'wsl-browser-file');await writeFile(file,'WSL browser content');const native=execFileSync('wsl.exe',['-d',distro,'--exec','wslpath','-a','-u',file],{encoding:'utf8'}).trim();
 await click('Windows','.environment');await click('WSL · '+distro,'.environment-options',true);await until(()=>cdp.evaluate(`document.querySelector('.environment button')?.textContent.includes('WSL')`));await click('设置','nav');await cdp.evaluate(`document.querySelector('button[aria-label="选择文件"]').click()`);await until(()=>cdp.evaluate(`!!document.querySelector('.file-browser[open]')`));await fill('目录路径',native.slice(0,native.lastIndexOf('/')),'.file-browser');await click('打开目录','.file-browser');await until(()=>cdp.evaluate(`document.querySelector('.file-browser-list')?.textContent.includes('wsl-browser-file')`));await screenshot('wsl-file-browser.png');await click('取消','.file-browser');await cdp.evaluate(`document.querySelector('button[aria-label="选择文件"]').click()`);await until(()=>cdp.evaluate(`!!document.querySelector('.file-browser[open]')`));await fill('目录路径',native.slice(0,native.lastIndexOf('/')),'.file-browser');await click('打开目录','.file-browser');await until(()=>cdp.evaluate(`document.querySelector('.file-browser-list')?.textContent.includes('wsl-browser-file')`));await cdp.evaluate(`[...document.querySelectorAll('.file-browser-list button')].find(b=>b.textContent.includes('wsl-browser-file')).click()`);await until(()=>cdp.evaluate(`document.querySelector('.files-panel textarea')?.value==='WSL browser content'`));await screenshot('wsl-file-open.png');
})
assert.deepEqual(exceptions,[])
console.log(JSON.stringify({passed,root,screenshots:['host-command-light.png','host-command-dark-narrow.png','host-ctrl-light.png','host-ctrl-dark-narrow.png','keys-light.png','keys-dark-narrow.png','settings-light.png','terminal-light.png','host-menu-light.png','hosts-light.png','plan-editor-light.png','plans-light.png','dark-narrow.png','host-menu-dark-narrow.png','editor-dark-narrow.png','help-light.png','help-dark-narrow.png']},null,2))
}catch(e){try{console.error(await cdp.evaluate(`document.body.innerText`));await screenshot('failure.png')}catch{}throw e}
finally{cdp?.close();edge.kill();for(const client of clients)client.end();sshd.close();echo.close();proc.stdin.end();await until(()=>proc.exitCode!==null,'Tether shutdown',10000).catch(()=>proc.kill())}
