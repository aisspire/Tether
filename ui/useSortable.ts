import { onMounted, onUnmounted, type Ref } from 'vue'

export interface SortMove { group:string; source:string; target:string; after:boolean }
export function moveItem<T extends {id:string}>(items:T[], move:SortMove):boolean {
 const from=items.findIndex(item=>item.id===move.source),target=items.findIndex(item=>item.id===move.target)
 if(from<0||target<0||from===target)return false
 const to=target+(move.after?1:0)-(from<target?1:0)
 if(from===to)return false
 items.splice(to,0,items.splice(from,1)[0]!)
 return true
}

export function useSortable(root:Ref<HTMLElement|undefined>, onMove:(move:SortMove)=>unknown, canStart:()=>boolean=()=>true) {
 let source:HTMLElement|null=null,target:HTMLElement|null=null,after=false,frame=0,x=0,y=0,lastTime=0,container:HTMLElement|null=null
 const item=(node:EventTarget|null)=>node instanceof Element?node.closest<HTMLElement>('[data-sort-id]'):null
 const sameGroup=(node:HTMLElement)=>source&&node.dataset.sortGroup===source.dataset.sortGroup&&node!==source
 function mark(node:HTMLElement|null){target?.removeAttribute('data-sort-drop');target=node;if(target)target.dataset.sortDrop=after?'after':'before'}
 function stop(){cancelAnimationFrame(frame);frame=0;source?.removeAttribute('data-sorting');source=null;mark(null);container=null;lastTime=0}
 function locate(){
  let node=item(document.elementFromPoint(x,y))
  // A rule is nested in a host; allow dropping hosts over their contents.
  while(node&&source&&node.dataset.sortGroup!==source.dataset.sortGroup)node=item(node.parentElement)
  if(node&&sameGroup(node)){const rect=node.getBoundingClientRect();after=y>rect.top+rect.height/2;mark(node)}else mark(null)
 }
 function tick(time:number){
  if(!source)return
  const elapsed=lastTime?Math.min(time-lastTime,32):16;lastTime=time
  const rect=container?.getBoundingClientRect(),top=rect?Math.max(0,rect.top):0,bottom=rect?Math.min(innerHeight,rect.bottom):innerHeight
  const header=container?.querySelector<HTMLElement>(':scope > .dialog-heading'),footer=container?.querySelector<HTMLElement>('footer')
  const edgeTop=header?Math.max(top,header.getBoundingClientRect().bottom):top
  const edgeBottom=footer?Math.min(bottom,footer.getBoundingClientRect().top):bottom
  const left=rect?.left??0,right=rect?.right??innerWidth
  if(x>=left&&x<=right&&y>=top&&y<=bottom){
   const zone=Math.max(1,Math.min(72,(edgeBottom-edgeTop)/3))
   const speed=y<edgeTop+zone?-Math.min(1,(edgeTop+zone-y)/zone):y>edgeBottom-zone?Math.min(1,(y-edgeBottom+zone)/zone):0
   if(speed){(container??document.scrollingElement)?.scrollBy({top:speed*720*elapsed/1000,behavior:'instant'});locate()}
  }
  frame=requestAnimationFrame(tick)
 }
 function start(event:DragEvent){
  const handle=event.target instanceof Element?event.target.closest<HTMLButtonElement>('[data-sort-handle]'):null
  if(!handle)return
  const row=item(handle)
  if(handle.disabled||!row||!canStart()){event.preventDefault();return}
  event.stopPropagation();source=row;row.dataset.sorting='true';container=row.closest('dialog');x=event.clientX;y=event.clientY
  if(event.dataTransfer){event.dataTransfer.effectAllowed='move';event.dataTransfer.setData('text/plain',row.dataset.sortId!);event.dataTransfer.setDragImage(row,12,12)}
  frame=requestAnimationFrame(tick)
 }
 function over(event:DragEvent){if(!source)return;event.preventDefault();event.stopPropagation();x=event.clientX;y=event.clientY;if(event.dataTransfer)event.dataTransfer.dropEffect='move';locate()}
 function drop(event:DragEvent){
  if(!source)return
  event.preventDefault();event.stopPropagation();x=event.clientX;y=event.clientY;locate()
  const move=target?{group:source.dataset.sortGroup!,source:source.dataset.sortId!,target:target.dataset.sortId!,after}:null
  stop();if(move)void onMove(move)
 }
 function leave(event:DragEvent){if(!event.relatedTarget&&(event.clientX<0||event.clientY<0||event.clientX>=innerWidth||event.clientY>=innerHeight)){x=-1;y=-1;mark(null)}}
 function key(event:KeyboardEvent){
  if(!event.altKey||!['ArrowUp','ArrowDown'].includes(event.key))return
  const handle=event.target instanceof Element?event.target.closest<HTMLButtonElement>('[data-sort-handle]'):null,row=item(handle)
  if(!handle||handle.disabled||!row||!canStart())return
  event.preventDefault();event.stopPropagation()
  const rows=[...(root.value?.querySelectorAll<HTMLElement>('[data-sort-id]')??[])].filter(node=>node.dataset.sortGroup===row.dataset.sortGroup&&node.getClientRects().length)
  const next=rows[rows.indexOf(row)+(event.key==='ArrowDown'?1:-1)]
  if(next)void onMove({group:row.dataset.sortGroup!,source:row.dataset.sortId!,target:next.dataset.sortId!,after:event.key==='ArrowDown'})
 }
 onMounted(()=>{root.value?.addEventListener('dragstart',start);root.value?.addEventListener('dragover',over);root.value?.addEventListener('drop',drop);root.value?.addEventListener('keydown',key);document.addEventListener('dragend',stop);document.addEventListener('drop',stop);document.addEventListener('dragleave',leave);window.addEventListener('blur',stop)})
 onUnmounted(()=>{stop();root.value?.removeEventListener('dragstart',start);root.value?.removeEventListener('dragover',over);root.value?.removeEventListener('drop',drop);root.value?.removeEventListener('keydown',key);document.removeEventListener('dragend',stop);document.removeEventListener('drop',stop);document.removeEventListener('dragleave',leave);window.removeEventListener('blur',stop)})
}
