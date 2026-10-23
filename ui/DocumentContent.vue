<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{ text: string; hideTitle?: boolean }>()
// A small text-only Markdown subset. Plugin documentation never executes HTML or scripts.
const blocks = computed(() => {
  const result: { kind: 'heading' | 'paragraph' | 'list' | 'code'; text: string; level?: number; items?: string[] }[] = []
  let code: string[] | null = null
  for (const line of props.text.replaceAll('\r\n', '\n').split('\n')) {
    if (line.startsWith('```')) {
      if (code) { result.push({ kind: 'code', text: code.join('\n') }); code = null }
      else code = []
    } else if (code) code.push(line)
    else if (!line.trim()) result.push({ kind: 'paragraph', text: '' })
    else {
      const heading = /^(#{1,6})\s+(.+)$/.exec(line)
      const item = /^[-*]\s+(.+)$/.exec(line)
      const previous = result.at(-1)
      if (heading) result.push({ kind: 'heading', text: heading[2]!, level: Math.min(heading[1]!.length + 1, 6) })
      else if (item) {
        if (previous?.kind === 'list') previous.items!.push(item[1]!)
        else result.push({ kind: 'list', text: '', items: [item[1]!] })
      } else if (previous?.kind === 'paragraph' && previous.text) previous.text += '\n' + line
      else result.push({ kind: 'paragraph', text: line })
    }
  }
  if (code) result.push({ kind: 'code', text: code.join('\n') })
  return result.filter(block => block.text || block.kind === 'list').filter((block, index) => !(props.hideTitle && index === 0 && block.kind === 'heading' && block.level === 2))
})
</script>

<template>
  <article class="document-content">
    <template v-for="(block, index) in blocks" :key="index">
      <component :is="`h${block.level}`" v-if="block.kind === 'heading'">{{ block.text }}</component>
      <ul v-else-if="block.kind === 'list'"><li v-for="(item, i) in block.items" :key="i">{{ item }}</li></ul>
      <pre v-else-if="block.kind === 'code'"><code>{{ block.text }}</code></pre>
      <p v-else>{{ block.text }}</p>
    </template>
  </article>
</template>

<style scoped>
.document-content { max-width: 72ch; margin: 0 auto; line-height: 1.78; overflow-wrap: anywhere; font-size: 13px; }
.document-content :is(h2,h3,h4,h5,h6) { margin: 26px 0 8px; font-size: 15px; letter-spacing: -.01em; }
.document-content h2 { margin-top: 26px; padding-bottom: 9px; border-bottom: 1px solid var(--border); font-size: 19px; letter-spacing: -.02em; }
.document-content h2:first-child { margin-top: 0; }
.document-content h3 { color: var(--text); }
.document-content > :first-child { margin-top: 0; }
.document-content p { margin: 9px 0; color: var(--muted); white-space: pre-line; }
.document-content ul { margin: 8px 0; padding-left: 20px; }
.document-content li { margin: 4px 0; color: var(--muted); }
.document-content li::marker { color: var(--accent); }
.document-content pre { margin: 10px 0; padding: 13px 14px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface-soft); white-space: pre-wrap; font-size: 11.5px; line-height: 1.65; }
.document-content code { color: var(--text); font-family: 'Cascadia Code', Consolas, monospace; }
.document-content pre code { color: var(--muted); }
</style>
