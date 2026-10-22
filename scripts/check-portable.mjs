import {spawn} from 'node:child_process'
import {mkdtemp,mkdir,copyFile,writeFile,readFile,chmod} from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import readline from 'node:readline'
import assert from 'node:assert/strict'
const win=process.platform==='win32',source=path.resolve(process.env.TETHER_EXE??(win?'target/windows/debug/tether.exe':'target/debug/tether'))
const root=await mkdtemp(win?path.resolve('target/check-portable-'):path.join(os.tmpdir(),'tether-portable-'))
const deployment=path.join(root,'portable with spaces'),profile=path.join(root,'user-profile'),old=path.join(profile,'Tether','config.json');await mkdir(deployment);await mkdir(path.dirname(old),{recursive:true})
const exe=path.join(deployment,win?'tether.exe':'tether');await copyFile(source,exe);if(!win)await chmod(exe,0o755)
const original=JSON.stringify({version:1,hosts:[{id:'stable-host',name:'portable fixture',destination:'fixture'}],rules:[],plans:[],theme:'system'})
await writeFile(old,original)
const env={...process.env,LOCALAPPDATA:profile,XDG_RUNTIME_DIR:path.join(root,'system-runtime')};delete env.TETHER_DATA_DIR
async function start(){const child=spawn(exe,['serve','--oriel-stdio'],{cwd:os.tmpdir(),env,stdio:['pipe','pipe','pipe']});let error='';child.stderr.on('data',b=>error+=b);const lines=readline.createInterface({input:child.stdout})[Symbol.asyncIterator]();const first=await lines.next();if(first.done)throw Error(error);return {child,d:JSON.parse(first.value)}}
async function stop({child,d}){await fetch(d.origin+'/api',{method:'POST',headers:{Authorization:`Bearer ${d.token}`,'Content-Type':'application/json'},body:JSON.stringify({action:'shutdown'})});await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('shutdown timeout')),10000);child.once('exit',()=>{clearTimeout(timer);resolve()});if(child.exitCode!==null){clearTimeout(timer);resolve()}})}
let active
try{
 active=await start();const data=path.join(deployment,'data'),config=path.join(data,win?'windows':'linux','config.json');const bytes=await readFile(config,'utf8');if(win)assert.equal(bytes,original);assert.equal(await readFile(old,'utf8'),original);await assert.rejects(readFile(path.join(data,'instance.json')));await assert.rejects(readFile(path.join(path.dirname(config),'instance.json')));await stop(active);active=null
 await writeFile(old,'broken legacy after migration');await writeFile(path.join(data,'keep-me'),'portable user data');await copyFile(source,exe)
 active=await start();assert.equal(await readFile(config,'utf8'),bytes);assert.equal(await readFile(path.join(data,'keep-me'),'utf8'),'portable user data');await stop(active);active=null
 if(!win){await chmod(data,0o500);await assert.rejects(start(),/不可写|Permission denied/);await chmod(data,0o700)}
 console.log(JSON.stringify({passed:win?3:4,root,checks:['portable config and separate user runtime','original config preserved; stable IDs migrated on Windows','binary replacement preserves data and never repeats migration',...(!win?['read-only data reports failure without changing source']:[])]},null,2))
}finally{if(active)await stop(active).catch(()=>active.child.kill())}
