// Temporary WSL-only SSH/Agent fixtures, driven by check-environments. Never reads ~/.ssh.
import ssh2 from 'ssh2'
import crypto from 'node:crypto'
import net from 'node:net'
import {spawn,execFileSync} from 'node:child_process'
import {mkdtemp,writeFile} from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import readline from 'node:readline'
const root=await mkdtemp(path.join(os.tmpdir(),'tether-wsl-fixture-'))
const key=path.join(root,'identity');execFileSync('ssh-keygen',['-q','-t','ed25519','-N','','-f',key])
const clients=new Set()
const hostKey=crypto.generateKeyPairSync('rsa',{modulusLength:2048,privateKeyEncoding:{type:'pkcs1',format:'pem'},publicKeyEncoding:{type:'spki',format:'pem'}}).privateKey
const sshd=new ssh2.Server({hostKeys:[hostKey]},client=>{clients.add(client);client.on('error',()=>{}).on('close',()=>clients.delete(client));client.on('authentication',ctx=>ctx.username==='fixture'&&ctx.method==='publickey'?ctx.accept():ctx.reject(['publickey']));client.on('ready',()=>client.on('tcpip',(accept,reject,info)=>{const dest=net.connect(info.destPort,'127.0.0.1',()=>{const ch=accept();ch.pipe(dest).pipe(ch)});dest.on('error',()=>reject())}))})
const listen=s=>new Promise(r=>s.listen(0,'127.0.0.1',()=>r(s.address().port)))
const sshPort=await listen(sshd),echo=net.createServer(s=>{s.on('error',()=>{});s.pipe(s)}),echoPort=await listen(echo)
const free=net.createServer();const managedPort=await listen(free);await new Promise(r=>free.close(r))
const config=path.join(root,'config');await writeFile(config,`Host fixture\n HostName 127.0.0.1\n Port ${sshPort}\n User fixture\n IdentityFile ${key}\n IdentitiesOnly yes\n UserKnownHostsFile ${root}/known_hosts\n StrictHostKeyChecking accept-new\n`)
const externalSlot=net.createServer(),externalPort=await listen(externalSlot);await new Promise(r=>externalSlot.close(r));const external=spawn('ssh',['-NT','-o','ExitOnForwardFailure=yes','-F',config,'-L',`127.0.0.1:${externalPort}:127.0.0.1:${echoPort}`,'fixture'],{stdio:'ignore'})
const socket=path.join(root,'agent.sock'),agent=spawn('ssh-agent',['-D','-a',socket],{stdio:'ignore'})
const probe=port=>new Promise(resolve=>{const s=net.connect(port,'127.0.0.1',()=>s.write('fixture'));s.setTimeout(1000,()=>{s.destroy();resolve(false)});s.once('data',b=>{s.destroy();resolve(b.toString()==='fixture')});s.on('error',()=>resolve(false));s.once('close',()=>resolve(false))})
for(let i=0;i<100&&!await probe(externalPort);i++)await new Promise(r=>setTimeout(r,50))
console.log(JSON.stringify({root,key,config,managedPort,externalPort,echoPort,socket,externalPid:external.pid}))
try{for await(const line of readline.createInterface({input:process.stdin})){const v=JSON.parse(line);console.log(JSON.stringify({id:v.id,managed:await probe(managedPort),external:await probe(externalPort)}))}}
finally{external.kill();agent.kill();for(const c of clients)c.end();sshd.close();echo.close()}
