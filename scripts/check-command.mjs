// Exported commands use only isolated keys/config and loopback SSH2 endpoints.
import assert from 'node:assert/strict'
import {spawn,execFileSync} from 'node:child_process'
import {mkdtemp,writeFile,readFile,rename} from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
import net from 'node:net'
import crypto from 'node:crypto'
import readline from 'node:readline'
import ssh2 from 'ssh2'

const win=process.platform==='win32'
const root=await mkdtemp(path.join(os.tmpdir(),"tether-command space's-$value-"))
if(win){const user=execFileSync('whoami',[],{encoding:'utf8'}).trim();execFileSync('icacls',[root,'/inheritance:r','/grant:r',`${user}:(OI)(CI)F`])}
const key=path.join(root,"identity space's-$value")
execFileSync('ssh-keygen',['-q','-t','ed25519','-N','','-f',key])
const privateKey=crypto.generateKeyPairSync('rsa',{modulusLength:2048,privateKeyEncoding:{type:'pkcs1',format:'pem'},publicKeyEncoding:{type:'spki',format:'pem'}}).privateKey
const clients=new Set(),relays=new Set(),commands=new Set()
let connections=0
const sshd=new ssh2.Server({hostKeys:[privateKey]},client=>{
 connections++;clients.add(client);client.on('error',()=>{}).on('close',()=>clients.delete(client))
 client.on('authentication',ctx=>ctx.method==='publickey'&&ctx.username==='fixture'?ctx.accept():ctx.reject(['publickey']))
 client.on('ready',()=>{
  client.on('tcpip',(accept,reject,info)=>{
   if(info.destIP!=='127.0.0.1')return reject()
   const socket=net.connect(info.destPort,'127.0.0.1',()=>{const channel=accept();channel.on('error',()=>{});channel.pipe(socket).pipe(channel)})
   socket.on('error',()=>reject())
  })
  client.on('request',(accept,reject,name,info)=>{
   if(name!=='tcpip-forward')return reject?.()
   const server=net.createServer(socket=>{socket.on('error',()=>{});client.forwardOut(info.bindAddr,info.bindPort,socket.remoteAddress,socket.remotePort,(error,channel)=>{if(error)return socket.destroy();channel.on('error',()=>{});channel.pipe(socket).pipe(channel)})})
   server.on('error',()=>reject?.());server.listen(info.bindPort,'127.0.0.1',()=>accept?.(info.bindPort));relays.add(server)
   client.on('close',()=>{server.close();relays.delete(server)})
  })
 })
})
await new Promise(resolve=>sshd.listen(0,'127.0.0.1',resolve))
const echo=net.createServer(socket=>{socket.on('error',()=>{});socket.pipe(socket)})
await new Promise(resolve=>echo.listen(0,'127.0.0.1',resolve))
const freePort=async()=>{const s=net.createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const n=s.address().port;await new Promise(r=>s.close(r));return n}
const ports=await Promise.all(Array.from({length:8},freePort)),echoPort=echo.address().port
const cfg=path.join(root,'ssh-config'),include=path.join(root,'included-config')
const knownHosts=path.join(await mkdtemp(path.join(os.tmpdir(),'tether-command-hosts-')),'known-hosts')
const sshPath=value=>value.replaceAll('\\','/')
const publicHostKey=ssh2.utils.parseKey(privateKey).getPublicSSH()
await writeFile(knownHosts,`[127.0.0.1]:${sshd.address().port} ssh-rsa ${publicHostKey.toString('base64')}\n`)
const base=`Host fixture jump1 jump2 target\n HostName 127.0.0.1\n User fixture\n Port ${sshd.address().port}\n IdentityFile "${sshPath(key)}"\n IdentitiesOnly yes\n StrictHostKeyChecking yes\n UserKnownHostsFile "${sshPath(knownHosts)}"\n GlobalKnownHostsFile none\n`
await writeFile(include,base+'Host target\n ProxyJump jump1,jump2\n')
await writeFile(cfg,`Include "${sshPath(include)}"\nHost fixture\n LocalForward 127.0.0.1:${ports[1]} 127.0.0.1:${echoPort}\nMatch host 127.0.0.1\n Compression yes\n`)
const host={id:'h',name:'fixture',destination:'fixture',ssh:'ssh',configFile:cfg,options:[],reconnect:false,hidden:true,timeoutSecs:10}
const rule=(id,port,extra={})=>({id,name:id,hostId:'h',direction:'local',bind:'127.0.0.1',port,target:'127.0.0.1',targetPort:echoPort,notes:'',...extra})
const config={version:1,hosts:[host,{...host,id:'jump-host',destination:'target'}],rules:[rule('local',ports[0]),rule('other',ports[2]),rule('remote',ports[3],{direction:'remote'}),rule('dynamic',ports[4],{direction:'dynamic',target:'',targetPort:0}),rule('jump',ports[5],{hostId:'jump-host'})],plans:[],theme:'system'}
await writeFile(path.join(root,'config.json'),JSON.stringify(config))
const exe=process.env.TETHER_EXE??path.resolve(win?'target/windows/debug/tether.exe':'target/debug/tether')
const backend=spawn(exe,['serve','--data-dir',root,'--oriel-stdio'],{env:{...process.env,XDG_RUNTIME_DIR:path.join(root,'runtime'),TETHER_UI_DIR:path.resolve('dist')},stdio:['pipe','pipe','inherit']})
const iterator=readline.createInterface({input:backend.stdout})[Symbol.asyncIterator]()
const first=await iterator.next();assert.equal(first.done,false,'backend started')
const desc=JSON.parse(first.value)
async function api(action,args={}){
 const result=await(await fetch(desc.origin+'/api',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${desc.token}`},body:JSON.stringify({action,...args})})).json()
 if(!result.ok)throw Error(result.error)
 return result.value
}
async function waitFor(fn){const start=Date.now();while(Date.now()-start<12000){if(await fn())return;await new Promise(r=>setTimeout(r,60))}throw Error('fixture wait timed out')}
async function reachable(port){return new Promise(resolve=>{const s=net.connect(port,'127.0.0.1',()=>{s.destroy();resolve(true)});s.on('error',()=>resolve(false))})}
async function echoTest(port){return new Promise((resolve,reject)=>{let received=false;const s=net.connect(port,'127.0.0.1',()=>s.write('single-command-data'));s.setTimeout(3000,()=>s.destroy(Error('echo timed out')));s.once('data',data=>{assert.equal(data.toString(),'single-command-data');received=true;s.end()});s.once('close',()=>received?resolve():reject(Error('echo closed before data')));s.on('error',reject)})}
function launch(value){
 if(globalThis.process.env.TETHER_COMMAND_DEBUG)console.log(value.command)
 const process=win?spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',value.command],{stdio:['ignore','ignore','pipe']}):spawn('sh',['-c',`exec ${globalThis.process.env.TETHER_COMMAND_DEBUG?value.command.replace(' -F',' -vvv -F'):value.command}`],{stdio:['ignore','ignore','pipe']})
 process.stderr.on('data',data=>{process.commandError=(process.commandError??'')+data.toString()})
 process.stderr.on('error',()=>{}) // Windows may reset pipes when our process tree is stopped.
 commands.add(process);process.on('exit',()=>commands.delete(process));return process
}
async function ready(process,port){await waitFor(()=>{if(process.exitCode!==null||process.signalCode!==null)throw Error(process.commandError||'exported command exited');return reachable(port)})}
async function stop(process){if(process.exitCode===null&&process.signalCode===null){if(win)execFileSync('taskkill',['/PID',String(process.pid),'/T','/F']);else process.kill()}await waitFor(()=>process.exitCode!==null||process.signalCode!==null)}
function forwards(value){return value.args.filter(arg=>['-L','-R','-D'].includes(arg))}
let passed=0
async function check(name,fn){await fn();passed++;console.log('PASS '+name)}
try{
 assert.equal((await api('status')).configError,null)
 await check('unstarted rule exports exactly one forward and generation opens no SSH connection',async()=>{
  const before=connections,value=await api('connectionCommand',{ruleId:'local'})
  assert.deepEqual(forwards(value),['-L']);assert.equal(value.shell,win?'PowerShell':'POSIX')
  assert.ok(!value.args.includes(cfg));assert.ok(!value.command.includes('runtime/'));assert.ok(!value.command.includes(ports[1]+':'))
  assert.equal(connections,before);assert.equal((await api('status')).groups.length,0)
  const process=launch(value)
  try{await ready(process,ports[0]);await echoTest(ports[0]);assert.equal(await reachable(ports[1]),false);assert.equal(await reachable(ports[2]),false)}finally{await stop(process)}
 })
 await check('remote and SOCKS exports carry real data',async()=>{
  for(const [id,flag,port] of [['remote','-R',ports[3]],['dynamic','-D',ports[4]]]){
   const value=await api('connectionCommand',{ruleId:id});assert.deepEqual(forwards(value),[flag])
   const process=launch(value)
   try{
    await ready(process,port)
    if(id==='remote'){await echoTest(port);continue}
    const s=net.connect(port,'127.0.0.1');s.on('error',()=>{});await new Promise(r=>s.once('connect',r));s.write(Buffer.from([5,1,0]));await new Promise(r=>s.once('data',r))
    s.write(Buffer.from([5,1,0,1,127,0,0,1,echoPort>>8,echoPort&255]));const reply=await new Promise(r=>s.once('data',r));assert.equal(reply[1],0)
    s.write('socks-command');const data=await new Promise(r=>s.once('data',r));assert.equal(data.toString(),'socks-command');const closed=new Promise(r=>s.once('close',r));s.end();await closed
   }finally{await stop(process)}
  }
 })
 await check('two-hop export preserves each identity and runs without original config files',async()=>{
  const value=await api('connectionCommand',{ruleId:'jump'});assert.deepEqual(forwards(value),['-L']);assert.ok(value.args.some(arg=>arg.startsWith('proxycommand ')))
  await rename(cfg,cfg+'.away');await rename(include,include+'.away')
  const process=launch(value)
  try{await ready(process,ports[5]);await echoTest(ports[5])}finally{await stop(process);await rename(cfg+'.away',cfg);await rename(include+'.away',include)}
 })
 await check('running shared group exports one rule and saved changes take precedence',async()=>{
  const started=await api('ensure',{request:{ruleIds:['local','other']}});assert.equal(started.ready,true,JSON.stringify(started))
  const before=await api('status'),group=before.groups.find(g=>g.rules.some(r=>r.id==='local'))
  assert.ok(group.rules.length>=2)
  const first=await api('connectionCommand',{ruleId:'local'});assert.deepEqual(forwards(first),['-L']);assert.ok(!first.command.includes(ports[2]+':'))
  before.config.rules.find(r=>r.id==='local').targetPort++
  await api('save',{config:before.config,revision:before.revision})
  const updated=await api('connectionCommand',{ruleId:'local'});assert.ok(updated.args.includes(`127.0.0.1:${ports[0]}:127.0.0.1:${echoPort+1}`))
  assert.equal((await api('status')).groups.find(g=>g.id===group.id).pid,group.pid);await echoTest(ports[0])
  const state=await api('status');state.config.rules=state.config.rules.filter(r=>r.id!=='local');await api('save',{config:state.config,revision:state.revision})
  const temporary=await api('connectionCommand',{ruleId:'local',groupId:group.id});assert.ok(temporary.args.includes(`127.0.0.1:${ports[0]}:127.0.0.1:${echoPort}`))
  const inherited=group.rules.find(r=>r.id.startsWith('ssh:'));assert.ok(inherited)
  const source=await api('connectionCommand',{ruleId:inherited.id,groupId:group.id});assert.deepEqual(forwards(source),['-L']);assert.ok(source.args.includes(`127.0.0.1:${ports[1]}:127.0.0.1:${echoPort}`))
  await api('stop',{scope:'all'})
 })
 await check('temporary mappings use their runtime snapshot and stale references fail',async()=>{
  const started=await api('ensure',{request:{temporary:[rule('temporary',ports[6])],overrides:['Compression=no']}});assert.equal(started.ready,true,JSON.stringify(started))
  const groupId=started.results.find(r=>r.ruleId==='temporary').groupId
  await rename(cfg,cfg+'.away')
  try{
   const value=await api('connectionCommand',{ruleId:'temporary',groupId});assert.deepEqual(forwards(value),['-L'])
   assert.match(execFileSync(value.program,['-G',...value.args],{encoding:'utf8'}),/compression no/)
  }finally{await rename(cfg+'.away',cfg)}
  await api('stop',{scope:'all'});await assert.rejects(api('connectionCommand',{ruleId:'temporary',groupId}),/映射已不存在/)
  await assert.rejects(api('connectionCommand',{ruleId:'unknown'}),/映射已不存在/)
 })
 await check('IPv6 and repeated identities survive native argument parsing',async()=>{
  const state=await api('status');state.config.rules.push(rule('ipv6',ports[7],{bind:'::1',target:'::1'}))
  state.config.hosts[0].options=[`IdentityFile="${sshPath(key)}"`,`IdentityFile="${sshPath(key)}-second"`]
  await api('save',{config:state.config,revision:state.revision})
  const value=await api('connectionCommand',{ruleId:'ipv6'});assert.ok(value.args.includes(`[::1]:${ports[7]}:[::1]:${echoPort}`))
  const parsed=execFileSync(value.program,['-G',...value.args],{encoding:'utf8'});assert.equal(parsed.split('\n').filter(l=>l.startsWith('identityfile ')).length,2)
 })
 await check('jump-dependent path tokens fail explicitly without changing running resources',async()=>{
  const state=await api('status');state.config.hosts.find(h=>h.id==='jump-host').options=['IdentityFile=~/.ssh/key-%j']
  await api('save',{config:state.config,revision:state.revision})
  await assert.rejects(api('connectionCommand',{ruleId:'jump'}),/%j 或 %C/)
  assert.equal((await api('status')).groups.length,0)
 })
 assert.ok((await readFile(cfg,'utf8')).startsWith('Include '),'original config was not rewritten')
 console.log(`${passed} connection command checks passed (${win?'Windows':'Linux auxiliary'}); fixtures: ${root}`)
}finally{
 for(const process of commands)await stop(process)
 backend.stdin.end();await waitFor(()=>backend.exitCode!==null||backend.signalCode!==null).catch(()=>backend.kill())
 for(const client of clients)client.end()
 for(const server of relays)server.close()
 sshd.close();echo.close()
}
