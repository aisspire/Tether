// Portable checks for gesture safety and stable-ID ordering; rendering is covered
// separately by check-ui.mjs in Windows Edge.
import assert from 'node:assert/strict'
import {readFile} from 'node:fs/promises'
import {test} from 'node:test'
import ts from 'typescript'
import {createRenderer} from 'vue'

async function sourceModule(file){
 const source=await readFile(new URL(file,import.meta.url),'utf8')
 const {outputText}=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}})
 return import('data:text/javascript;base64,'+Buffer.from(outputText.replaceAll("from 'vue'",`from ${JSON.stringify(import.meta.resolve('vue'))}`)).toString('base64'))
}
const {vDialogDismiss}=await sourceModule('../ui/dialogDismiss.ts')
const {moveItem,useSortable}=await sourceModule('../ui/useSortable.ts')
const {copyGeneratedText}=await sourceModule('../ui/clipboard.ts')

async function clipboardFixture(clipboard,Item,fn){
 const descriptors=new Map(['navigator','ClipboardItem'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]))
 Object.defineProperty(globalThis,'navigator',{configurable:true,value:{clipboard}})
 Object.defineProperty(globalThis,'ClipboardItem',{configurable:true,value:Item})
 try{await fn()}finally{for(const [key,descriptor] of descriptors)if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key]}
}
test('command copy reserves clipboard access before asynchronous generation finishes',async()=>{
 let release,wrote=false,content
 class Item{constructor(data){this.data=data}}
 await clipboardFixture({write:async items=>{wrote=true;content=await(await items[0].data['text/plain']).text()}},Item,async()=>{
  const operation=copyGeneratedText(()=>new Promise(resolve=>release=resolve))
  assert.equal(wrote,true,'write starts during the original click')
  release('ssh -N -L single-forward host');await operation
  assert.equal(content,'ssh -N -L single-forward host')
 })
})
test('command generation and clipboard denial remain distinguishable and never report success',async()=>{
 class Item{constructor(data){this.data=data}}
 await clipboardFixture({write:async items=>{await items[0].data['text/plain']}},Item,async()=>{
  await assert.rejects(copyGeneratedText(async()=>{throw Error('SSH 参数无法导出')}),/SSH 参数无法导出/)
 })
 await clipboardFixture({write:async()=>{throw Error('剪贴板权限被拒绝')}},Item,async()=>{
  await assert.rejects(copyGeneratedText(async()=>'ssh command'),/剪贴板权限被拒绝/)
  await assert.rejects(copyGeneratedText(async()=>{throw Error('命令生成也失败')}),/命令生成也失败/)
 })
 await clipboardFixture({writeText:async text=>assert.equal(text,'fallback command')},undefined,async()=>{
  await copyGeneratedText(async()=>'fallback command')
 })
})

function fixture(){
 const dialog=new EventTarget();let dismissed=0
 dialog.getBoundingClientRect=()=>({left:100,top:100,right:500,bottom:500})
 vDialogDismiss.mounted(dialog,{value:()=>dismissed++})
 function send(type,x,y,options={}){const event=new Event(type);Object.assign(event,{clientX:x,clientY:y,pointerId:1,isPrimary:true,button:0,...options});dialog.dispatchEvent(event)}
 function gesture(start,end,options={}){send('pointerdown',...start,options);send('pointerup',...end,options);send('click',...end,options)}
 return {dialog,send,gesture,count:()=>dismissed}
}
test('only a complete primary click on the backdrop dismisses',()=>{
 const f=fixture()
 f.gesture([200,200],[20,20]);assert.equal(f.count(),0,'inside to outside must preserve the editor')
 f.gesture([20,20],[200,200]);assert.equal(f.count(),0,'outside to inside must preserve the editor')
 f.gesture([110,110],[110,110]);assert.equal(f.count(),0,'dialog padding is inside')
 f.gesture([20,20],[20,20],{button:2});assert.equal(f.count(),0,'secondary button must not dismiss')
 f.gesture([20,20],[20,20],{isPrimary:false});assert.equal(f.count(),0,'secondary pointer must not dismiss')
 f.gesture([20,20],[20,20]);assert.equal(f.count(),1)
 f.send('click',20,20);assert.equal(f.count(),1,'an unpaired click must not dismiss')
})
test('cancelled, mismatched and stale gestures cannot dismiss a reopened dialog',()=>{
 const f=fixture()
 f.send('pointerdown',20,20);f.send('pointercancel',20,20);f.send('pointerup',20,20);f.send('click',20,20);assert.equal(f.count(),0)
 f.send('pointerdown',20,20);f.send('pointerup',20,20,{pointerId:2});f.send('click',20,20);assert.equal(f.count(),0)
 f.send('pointerdown',20,20);f.send('close',20,20);f.send('pointerup',20,20);f.send('click',20,20);assert.equal(f.count(),0)
 vDialogDismiss.beforeUnmount(f.dialog);f.gesture([20,20],[20,20]);assert.equal(f.count(),0,'unmount removes listeners')
})
test('sorting moves the existing object before or after a stable target',()=>{
 const original=['a','b','c','d'].map(id=>({id,ruleIds:[`${id}-rule`]}))
 const rows=[...original],move=(source,target,after)=>moveItem(rows,{group:'plans',source,target,after})
 assert.equal(move('d','a',false),true);assert.deepEqual(rows.map(x=>x.id),['d','a','b','c'])
 assert.equal(move('d','c',true),true);assert.deepEqual(rows.map(x=>x.id),['a','b','c','d'])
 assert.equal(move('a','c',false),true);assert.deepEqual(rows.map(x=>x.id),['b','a','c','d'])
 assert.equal(move('a','c',false),false,'adjacent no-op')
 assert.equal(move('a','a',true),false)
 assert.equal(move('missing','a',true),false)
 for(const row of rows)assert.equal(row,original.find(x=>x.id===row.id),'IDs, references and contents remain unchanged')
})

// A minimal event/animation host checks the scrolling lifecycle without claiming
// browser layout coverage. Both page and dialog geometry are supplied explicitly.
test('stationary drag scrolls either edge and cancellation stops all frames',()=>{
 const names=['Element','document','window','innerWidth','innerHeight','requestAnimationFrame','cancelAnimationFrame']
 const originals=new Map(names.map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]))
 class Node extends EventTarget {
  dataset={};rect={left:0,right:900,top:0,bottom:700,width:900,height:700};scrollTop=300;parentElement=null;dialog=null
  getBoundingClientRect(){return this.rect}
  getClientRects(){return [this.rect]}
  closest(selector){if(selector==='dialog')return this.dialog;if(selector==='[data-sort-handle]')return this.handle?this:null;if(selector==='[data-sort-id]')return this.dataset.sortId?this:this.parentElement?.closest(selector)??null;return null}
  removeAttribute(name){delete this.dataset[name.replace(/^data-/, '').replace(/-([a-z])/g,(_,c)=>c.toUpperCase())]}
  querySelector(selector){return selector.includes('dialog-heading')?this.header:selector==='footer'?this.footer:null}
  querySelectorAll(){return [source,target]}
  scrollBy({top}){this.scrollTop+=top}
 }
 const root=new Node(),source=new Node(),target=new Node(),handle=new Node(),page=new Node(),doc=new Node(),win=new Node()
 source.dataset={sortId:'a',sortGroup:'hosts'};target.dataset={sortId:'b',sortGroup:'hosts'};handle.handle=true;handle.parentElement=source
 doc.scrollingElement=page;doc.elementFromPoint=()=>target
 const frames=new Map();let frameId=0,time=0,moves=[]
 Object.assign(globalThis,{Element:Node,document:doc,window:win,innerWidth:900,innerHeight:700,requestAnimationFrame:fn=>{frames.set(++frameId,fn);return frameId},cancelAnimationFrame:id=>frames.delete(id)})
 const step=()=>{const pending=[...frames.values()];frames.clear();time+=16;pending.forEach(fn=>fn(time))}
 const send=(node,type,targetNode,x,y)=>{const event=new Event(type,{cancelable:true});Object.defineProperty(event,'target',{value:targetNode});Object.assign(event,{clientX:x,clientY:y});node.dispatchEvent(event)}
 const renderer=createRenderer({createComment:()=>({}),insert(){},remove(){},parentNode(){},nextSibling(){}})
 const app=renderer.createApp({setup(){useSortable({value:root},move=>moves.push(move));return ()=>null}})
 try{
  app.mount({})
  send(root,'dragstart',handle,200,300);send(root,'dragover',target,200,690);step();step();assert.ok(page.scrollTop>300,'page scrolls down while pointer stays still')
  const down=page.scrollTop;send(root,'dragover',target,200,10);step();step();assert.ok(page.scrollTop<down,'page scrolls up')
  doc.dispatchEvent(new Event('dragend'));assert.equal(frames.size,0);assert.equal(source.dataset.sorting,undefined);assert.equal(target.dataset.sortDrop,undefined);assert.deepEqual(moves,[])
  const dialog=new Node();dialog.rect={left:100,right:800,top:100,bottom:600,width:700,height:500}
  dialog.header=new Node();dialog.header.rect={bottom:180};dialog.footer=new Node();dialog.footer.rect={top:540};source.dialog=dialog
  send(root,'dragstart',handle,200,300);send(root,'dragover',target,200,530);const oldPage=page.scrollTop;step();assert.ok(dialog.scrollTop>300);assert.equal(page.scrollTop,oldPage,'dialog drag does not scroll the page')
  const downDialog=dialog.scrollTop;send(root,'dragover',target,200,190);step();assert.ok(dialog.scrollTop<downDialog)
  send(root,'dragover',target,50,190);const stopped=dialog.scrollTop;step();assert.equal(dialog.scrollTop,stopped,'pointer outside dialog stops scrolling')
  send(root,'drop',target,200,300);assert.equal(frames.size,0);assert.deepEqual(moves,[{group:'hosts',source:'a',target:'b',after:false}])
  send(root,'dragstart',handle,200,300);app.unmount();assert.equal(frames.size,0)
  send(root,'dragstart',handle,200,300);assert.equal(frames.size,0,'unmounted sorter has no listeners')
 }finally{
  app.unmount()
  for(const [name,descriptor] of originals)if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name]
 }
})
