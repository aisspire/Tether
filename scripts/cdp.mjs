export const delay=ms=>new Promise(r=>setTimeout(r,ms))
export async function until(fn,description='condition',timeout=25000){const end=Date.now()+timeout;let last;while(Date.now()<end){try{const v=await fn();if(v)return v}catch(e){last=e}await delay(100)}throw new Error(`Timeout: ${description} ${last?.message??''}`)}
export class Cdp {
 next=0;pending=new Map()
 async connect(url){this.socket=new WebSocket(url);this.socket.addEventListener('message',event=>{const m=JSON.parse(String(event.data));const p=this.pending.get(m.id);if(p){clearTimeout(p.timer);this.pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result)}});await new Promise((r,j)=>{this.socket.addEventListener('open',r,{once:true});this.socket.addEventListener('error',j,{once:true})});return this}
 command(method,params={}){const id=++this.next;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{this.pending.delete(id);reject(new Error(`CDP timeout ${method}`))},30000);this.pending.set(id,{resolve,reject,timer});this.socket.send(JSON.stringify({id,method,params}))})}
 async evaluate(expression){const r=await this.command('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true,userGesture:true});if(r.exceptionDetails)throw new Error(r.exceptionDetails.exception?.description??r.exceptionDetails.text);return r.result.value}
 close(){this.socket.close()}
}
