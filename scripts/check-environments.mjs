// Interface coverage against the same binary on either OS, plus the real Windows -> WSL bridge.
import {spawn,execFileSync} from 'node:child_process'
import {mkdtemp,writeFile,readFile,mkdir,copyFile} from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
import net from 'node:net'
import readline from 'node:readline'
import assert from 'node:assert/strict'
const win=process.platform==='win32',exe=path.resolve(process.env.TETHER_EXE??(win?'target/windows/debug/tether.exe':'target/debug/tether'))
const root=await mkdtemp(win?path.resolve('target/check-environments-'):path.join(os.tmpdir(),'tether-environments-'))
const env={...process.env,...(!win?{XDG_RUNTIME_DIR:path.join(root,'system-runtime')}:{})}
const proc=spawn(exe,['serve','--data-dir',root,'--oriel-stdio'],{env,stdio:['pipe','pipe','inherit']})
const lines=readline.createInterface({input:proc.stdout})[Symbol.asyncIterator]();const first=await lines.next();assert.equal(first.done,false);const descriptor=JSON.parse(first.value)
let passed=0,fixture,fixtureLines,blocker,remoteId;let fixtureSequence=0;const terminalPids=[]
const fixtureProbe=async()=>{fixture.stdin.write(JSON.stringify({id:++fixtureSequence})+"\n");return JSON.parse((await fixtureLines.next()).value)}
async function api(action,args={}){const r=await fetch(descriptor.origin+'/api',{method:'POST',headers:{Authorization:`Bearer ${descriptor.token}`,'Content-Type':'application/json'},body:JSON.stringify({action,...args})});const data=await r.json();if(!data.ok)throw Error(data.error);return data.value}
async function check(name,fn){await fn();console.log('PASS '+name);passed++}
async function picker(environment,p,cancel=false){
 const request=api('files.choose',{environment});
 const driver=spawn('powershell.exe',['-NoProfile','-File',path.resolve('scripts/picker-fixture.ps1')],{env:{...process.env,TETHER_PICK_PARENT:String(proc.pid),TETHER_PICK_PATH:p,TETHER_PICK_CANCEL:cancel?'1':'0'},stdio:['ignore','ignore','pipe']});let error='';driver.stderr.on('data',b=>error+=b);await new Promise((resolve,reject)=>driver.once('exit',code=>code===0?resolve():reject(Error(error))));return Promise.race([request,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error('picker result timeout')),25000);timer.unref()})])
}
async function files(environment,translate=p=>p){
 const file=path.join(root,`text-${environment}.txt`),privateFile=path.join(root,`private-${environment}`),missing=path.join(root,'does-not-exist')
 await writeFile(file,'first\nneedle\nlast\n');await writeFile(privateFile,'-----BEGIN OPENSSH PRIVATE KEY-----\nfixture-private-content\n-----END OPENSSH PRIVATE KEY-----\n')
 const p=translate(file),key=translate(privateFile)
 const before=await api('status',{environment});await assert.rejects(api('files.open',{environment,path:translate(missing)}));assert.deepEqual((await api('status',{environment})).config.sshFiles,before.config.sshFiles)
 const opened=await api('files.open',{environment,path:p});assert.equal(opened.readOnly,false);assert.equal(opened.text,'first\nneedle\nlast\n');let state=await api('status',{environment});assert.ok(state.config.sshFiles.includes(p));assert.deepEqual(state.config.keyFiles,[])
 const saved=await api('files.write',{environment,path:p,text:'changed\n',revision:opened.revision});assert.equal(await readFile(file,'utf8'),'changed\n');assert.ok(saved.backup)
 await writeFile(file,'external change\n');await assert.rejects(api('files.write',{environment,path:p,text:'lost',revision:saved.revision}),/外部修改/);assert.equal(await readFile(file,'utf8'),'external change\n')
 const secret=await api('files.open',{environment,path:key});assert.equal(secret.readOnly,true);await assert.rejects(api('files.write',{environment,path:key,text:'erase',revision:secret.revision}),/私钥/)
 assert.ok(!JSON.stringify(await api('status',{environment})).includes('fixture-private-content'))
 state=await api('status',{environment});state.config.keyFiles=[key];await api('save',{environment,config:state.config,revision:state.revision});await api('files.forget',{environment,path:key});state=await api('status',{environment});assert.ok(!state.config.sshFiles.includes(key));assert.ok(state.config.keyFiles.includes(key));assert.ok((await readFile(privateFile,'utf8')).includes('fixture-private-content'))
}
try{
 await check('environment omission always resolves to the local backend',async()=>{const s=await api('status');assert.equal(s.environment,win?'windows':'linux');await assert.rejects(api('status',{environment:'not-an-environment'}));assert.equal((await api('status')).environment,s.environment)})
 await check('file open, history, external conflicts, backups and private-key read-only',()=>files(win?'windows':'linux'))
 if(win&&process.env.TETHER_PICKER_TESTS)await check('native Windows picker opens a file and cancellation never registers a path',async()=>{const file=path.join(root,'picked-local.txt');await writeFile(file,'picked');const opened=await picker('windows',file);assert.equal(opened.text,'picked');const before=(await api('status')).config.sshFiles;assert.equal((await picker('windows','',true)).cancelled,true);assert.deepEqual((await api('status')).config.sshFiles,before)})
 const inventory=await api('environments.list');assert.ok(!(inventory.distributions||[]).some(d=>d.name.startsWith('docker-desktop')))
 if(win&&(inventory.distributions||[]).length){
  const distro=inventory.distributions[0].name
  const selected=await api('environments.select',{distro}),id=selected.environment.id
  const translate=p=>execFileSync('wsl.exe',['-d',distro,'--exec','wslpath','-a','-u',p],{encoding:'utf8'}).trim()
  await check('WSL actions execute natively and keep file/key configuration independent',async()=>{assert.match(id,/^wsl-/);assert.equal(selected.state.environment,id);assert.deepEqual(selected.state.config.hosts,[]);await files(id,translate);assert.equal((await api('status')).environment,'windows');const local=await api('status'),remote=await api('status',{environment:id});assert.ok(local.config.sshFiles.every(p=>!remote.config.sshFiles.includes(p)));assert.ok(!remote.configPath.includes('C:\\'))})
  await check('WSL file browser lists the native directory without registering cancelled selections',async()=>{const file=path.join(root,'picked-wsl.txt');await writeFile(file,'picked WSL');const native=translate(file),before=(await api('status',{environment:id})).config.sshFiles;const listing=await api('files.browse',{environment:id,path:translate(root)});assert.ok(listing.entries.some(e=>e.path===native&&!e.directory));assert.deepEqual((await api('status',{environment:id})).config.sshFiles,before);assert.equal((await api('files.open',{environment:id,path:native})).text,'picked WSL')})
  await check('switch remembers distribution and preserves backend identity',async()=>{const before=await api('status',{environment:id});await api('environments.select');const again=await api('environments.select',{distro});assert.equal(again.environment.id,id);assert.equal(again.state.instance,before.instance);assert.equal((await api('environments.list')).selected,id);const configs=await readFile(path.join(root,'environments',id,'config.json'),'utf8');assert.ok(configs.includes('sshFiles'));assert.equal((await api('status')).environment,'windows')})
  await check('invalid WSL selection leaves both existing sessions usable',async()=>{await assert.rejects(api('environments.select',{distro:'tether-fixture-missing-distribution'}));assert.equal((await api('status',{environment:id})).environment,id);assert.equal((await api('status')).environment,'windows')})
  if(process.env.TETHER_WSL_NODE){
   blocker=net.createServer(s=>s.end());await new Promise(r=>blocker.listen(0,'127.0.0.1',r));const port=blocker.address().port
   fixture=spawn('wsl.exe',['-d',distro,'--exec',process.env.TETHER_WSL_NODE,translate(path.resolve('scripts/wsl-fixture.mjs')),String(port)],{stdio:['pipe','pipe','inherit']});fixtureLines=readline.createInterface({input:fixture.stdout})[Symbol.asyncIterator]();const f=JSON.parse((await fixtureLines.next()).value);remoteId=id
   const rule=(rid,p)=>({id:rid,name:rid,hostId:'fixture',direction:'local',bind:'127.0.0.1',port:p,target:'127.0.0.1',targetPort:f.echoPort,notes:''})
   const remote=await api('status',{environment:id});remote.config.hosts=[{id:'fixture',name:'WSL fixture',destination:'fixture',configFile:f.config,ssh:'ssh',options:[],timeoutSecs:15,reconnect:true,hidden:true}];remote.config.rules=[rule('managed',f.managedPort),rule('external',f.externalPort),rule('same-number',port)];remote.config.keyFiles=[f.key];await api('save',{environment:id,config:remote.config,revision:remote.revision})
   await check('WSL Agent binds an isolated socket and never stops its external service',async()=>{const bound=await api('agent.bind',{environment:id,path:f.socket});assert.equal(bound.socket,f.socket);await api('agent.add',{environment:id,path:f.key});assert.ok((await api('keys.list',{environment:id})).agent.includes('ssh-ed25519'));await api('agent.remove',{environment:id,path:f.key})})
   await check('Windows occupancy and WSL SSH use actual endpoints even with a same-number mapping',async()=>{const local=await api('status');local.config.hosts=[{...remote.config.hosts[0],destination:'127.0.0.1',configFile:'none',options:[`Port=${port}`]}];local.config.rules=[rule('windows-occupied',port)];await api('save',{config:local.config,revision:local.revision});const w=await api('test',{hostId:'fixture'}),l=await api('test',{environment:id,hostId:'fixture'});assert.equal(w.portChecks[0].state,'unavailable');assert.ok(w.portChecks[0].source==='Windows');assert.equal(l.portChecks.find(c=>c.ruleId==='external').state,'running',JSON.stringify(l.portChecks));assert.equal(l.portChecks.find(c=>c.ruleId==='external').pid,f.externalPid);assert.notEqual(l.portChecks.find(c=>c.ruleId==='same-number').state,'running')})
   await check('switch retains managed WSL forwarding and process identity',async()=>{assert.equal((await api('ensure',{environment:id,request:{ruleIds:['managed']}})).ready,true);const group=(await api('status',{environment:id})).groups[0];await api('environments.select');assert.equal((await api('status',{environment:id})).groups[0].pid,group.pid);assert.equal((await fixtureProbe()).managed,true)})
   await check('external terminal follows WSL distribution/user and starts only the selected native batch',async()=>{
    const state=await api('status',{environment:id});state.config.rules.push(rule('terminal-a',f.terminalPorts[0]),rule('terminal-b',f.terminalPorts[1]));await api('save',{environment:id,config:state.config,revision:state.revision})
    const command=await api('hostCommand',{environment:id,hostId:'fixture',ruleIds:['terminal-a','terminal-b']});assert.equal(command.shell,'POSIX');assert.equal(command.args.filter(a=>a==='-L').length,2);assert.ok(command.command.includes(f.key))
    const terminal=await api('hostTerminal',{environment:id,hostId:'fixture',ruleIds:['terminal-a','terminal-b']});terminalPids.push(terminal.terminalPid);assert.equal(terminal.environment,id)
    let ready=false;for(let i=0;i<80;i++){if((await fixtureProbe()).terminal.every(Boolean)){ready=true;break}await new Promise(r=>setTimeout(r,100))}assert.equal(ready,true,'selected WSL forwards transmit data through the Linux SSH client')
    assert.ok((await api('status',{environment:id})).groups.every(g=>g.rules.every(r=>!r.id.startsWith('terminal-'))))
    await assert.rejects(api('hostTerminal',{environment:id,hostId:'fixture',ruleIds:['terminal-a']}),/占用/)
   })
  }
  await check('concurrent WSL status and independent actions use correlated replies',async()=>{const results=await Promise.all(Array.from({length:20},(_,i)=>api(i%2?'status':'keys.list',{environment:id})));assert.ok(results.every((r,i)=>i%2?r.environment===id:Array.isArray(r.files)))})
 }else{
  await check('missing WSL cancels selection without changing local state',async()=>{await assert.rejects(api('environments.select',{distro:'missing'}));assert.equal((await api('status')).environment,win?'windows':'linux')})
 }
 await api('shutdown');await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('shutdown timeout')),15000);proc.once('exit',()=>{clearTimeout(timer);resolve()});if(proc.exitCode!==null){clearTimeout(timer);resolve()}})
 if(fixture)await check('complete exit cleans managed WSL forwarding and preserves external SSH',async()=>{const result=await fixtureProbe();assert.equal(result.managed,false);assert.equal(result.external,true);if(terminalPids.length)assert.deepEqual(result.terminal,[true,true])})
 console.log(JSON.stringify({passed,root,platform:process.platform},null,2))
}finally{fixture?.stdin.end();blocker?.close();proc.stdin.end();if(proc.exitCode===null)proc.kill();if(fixture){for(let i=0;i<100&&fixture.exitCode===null&&fixture.signalCode===null;i++)await new Promise(r=>setTimeout(r,100))}for(const pid of terminalPids){try{execFileSync('taskkill.exe',['/PID',String(pid),'/T','/F'],{stdio:'ignore'})}catch{}}}
