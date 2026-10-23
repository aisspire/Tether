import type { ObjectDirective } from 'vue'

// A click may target the dialog when a gesture starts in its contents and ends
// on the backdrop. Require both ends of the primary gesture to be outside.
const cleanup = new WeakMap<HTMLDialogElement, () => void>()
export const vDialogDismiss: ObjectDirective<HTMLDialogElement, () => unknown> = {
 mounted(dialog, binding) {
  let pointer: number | null = null, complete = false
  const outside = (event: MouseEvent) => {
   const rect = dialog.getBoundingClientRect()
   return event.target === dialog && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom)
  }
  const reset = () => { pointer = null; complete = false }
  const down = (event: PointerEvent) => { reset(); if (event.button === 0 && event.isPrimary && outside(event)) pointer = event.pointerId }
  const up = (event: PointerEvent) => { complete = pointer === event.pointerId && outside(event); pointer = null }
  const click = (event: MouseEvent) => { const dismiss = complete && event.button === 0 && outside(event); reset(); if (dismiss) void binding.value() }
  dialog.addEventListener('pointerdown', down)
  dialog.addEventListener('pointerup', up)
  dialog.addEventListener('pointercancel', reset)
  dialog.addEventListener('close', reset)
  dialog.addEventListener('click', click)
  cleanup.set(dialog, () => {
   dialog.removeEventListener('pointerdown', down)
   dialog.removeEventListener('pointerup', up)
   dialog.removeEventListener('pointercancel', reset)
   dialog.removeEventListener('close', reset)
   dialog.removeEventListener('click', click)
  })
 },
 beforeUnmount(dialog) { cleanup.get(dialog)?.(); cleanup.delete(dialog) },
}
