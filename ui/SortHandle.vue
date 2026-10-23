<script setup lang="ts">
import UiIcon from './UiIcon.vue'
defineProps<{label:string;disabled?:boolean}>()
</script>
<template>
 <button type="button" class="icon-button sort-handle" data-sort-handle :draggable="!disabled" :disabled="disabled" :title="`${label}：拖拽排序，或按 Alt+↑/↓`" :aria-label="`${label}：拖拽排序，或按 Alt+↑/↓`"><UiIcon name="grip" :size="16" /></button>
</template>
