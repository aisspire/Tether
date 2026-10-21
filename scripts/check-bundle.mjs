import {spawn,execFileSync} from 'node:child_process'
import {mkdtemp,readFile} from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
import readline from 'node:readline'
import assert from 'node:assert/strict'
const exe=path.resolve(process.argv[2]??'target/bundle/tether.exe'),data=await mkdtemp(path.resolve('target/check-bundle-'))
assert.equal(JSON.parse(execFileSync(exe,['check'],{encoding:'utf8'})).protocol,1)
const env={...process.env};delete env.TETHER_UI_DIR
const p=spawn(exe,['serve','--data-dir',data,'--oriel-stdio'],{cwd:os.tmpdir(),env,stdio:['pipe','pipe','inherit']})
try{
const lines=readline.createInterface({input:p.stdout})[Symbol.asyncIterator]();const d=JSON.parse((await lines.next()).value)
const html=await(await fetch(d.origin)).text();const js=html.match(/src="([^"]+\.js)"/)[1];assert.ok((await fetch(d.origin+js)).ok)
const out=JSON.parse(execFileSync(exe,['request','--data-dir',data,'--json','{"action":"status"}'],{encoding:'utf8'}));assert.equal(out.value.instance,d.instance)
const r=await fetch(d.origin+'/api',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${d.token}`},body:'{"action":"shutdown"}'});assert.equal((await r.json()).ok,true)
await new Promise((resolve,reject)=>{if(p.exitCode!==null)return resolve();const timer=setTimeout(()=>reject(new Error('shutdown still waits for open stdin')),7000);p.once('exit',()=>{clearTimeout(timer);resolve()})})
await assert.rejects(readFile(path.join(data,'instance.json')))
console.log(JSON.stringify({passed:['portable assets outside repository cwd','native SSH check','CLI authenticated status','shutdown exits with stdin still open'],exe,data},null,2))
}finally{p.stdin.end();if(p.exitCode===null)p.kill()}
