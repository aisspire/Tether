import { onMounted, onBeforeUnmount, type Ref } from 'vue'

export function useRowMenus(root: Ref<HTMLElement | undefined>) {
  const openMenus = () => [...(root.value?.querySelectorAll<HTMLDetailsElement>('details.row-menu[open]') ?? [])]

  function dismissOutside(event: Event) {
    if (!(event.target instanceof Node)) return
    for (const menu of openMenus()) {
      if (!menu.contains(event.target)) menu.open = false
    }
  }

  function dismissAction(event: MouseEvent) {
    if (!(event.target instanceof Element)) return
    const button = event.target.closest<HTMLButtonElement>('.menu-panel > button')
    const menu = button?.closest<HTMLDetailsElement>('details.row-menu')
    if (menu && !button?.disabled) {
      const restoreFocus = menu.contains(document.activeElement)
      menu.open = false
      if (restoreFocus) menu.querySelector<HTMLElement>(':scope > summary')?.focus()
    }
  }

  function dismissEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented) return
    const menus = openMenus()
    if (!menus.length) return
    event.preventDefault()
    for (const menu of menus) {
      menu.open = false
      menu.querySelector<HTMLElement>(':scope > summary')?.focus()
    }
  }

  function keepOneOpen(event: Event) {
    const current = event.target
    if (!(current instanceof HTMLDetailsElement) || !current.matches('.row-menu[open]')) return
    for (const menu of openMenus()) {
      if (menu !== current) menu.open = false
    }
  }

  onMounted(() => {
    document.addEventListener('pointerdown', dismissOutside, true)
    document.addEventListener('click', dismissOutside, true)
    document.addEventListener('keydown', dismissEscape)
    root.value?.addEventListener('click', dismissAction)
    root.value?.addEventListener('toggle', keepOneOpen, true)
  })
  onBeforeUnmount(() => {
    document.removeEventListener('pointerdown', dismissOutside, true)
    document.removeEventListener('click', dismissOutside, true)
    document.removeEventListener('keydown', dismissEscape)
    root.value?.removeEventListener('click', dismissAction)
    root.value?.removeEventListener('toggle', keepOneOpen, true)
  })
}
