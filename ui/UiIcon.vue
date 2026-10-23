<script setup lang="ts">
defineProps<{ name: string; size?: number }>()
</script>

<template>
  <svg
    :width="size || 18"
    :height="size || 18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <template v-if="name === 'host'">
      <rect x="3.5" y="4" width="17" height="7" rx="2" />
      <rect x="3.5" y="13" width="17" height="7" rx="2" />
      <path d="M7 7.5h.01M7 16.5h.01M11 7.5h5M11 16.5h5" />
    </template>
    <template v-else-if="name === 'plan'">
      <path d="M5 5.5h14M5 12h14M5 18.5h8" />
      <path d="m16.5 16 1.8 1.8 3-3.3" />
    </template>
    <template v-else-if="name === 'key'">
      <circle cx="8" cy="8" r="4.2" />
      <path d="m11.1 11.1 8.4 8.4m-2.7-2.7 2.2-2.2m-4.8 0 2.2-2.2" />
    </template>
    <template v-else-if="name === 'settings'">
      <path d="M4 7h16M4 17h16" />
      <circle cx="9" cy="7" r="2.4" fill="var(--surface-strong)" />
      <circle cx="15" cy="17" r="2.4" fill="var(--surface-strong)" />
    </template>
    <template v-else-if="name === 'alert'">
      <path d="M12 4 3.2 19.5h17.6L12 4Z" />
      <path d="M12 9.5v4.2M12 17h.01" />
    </template>
    <template v-else-if="name === 'help'">
      <circle cx="12" cy="12" r="8.5" />
      <path d="M9.7 9.3a2.4 2.4 0 1 1 3.4 2.2c-.8.4-1.1 1-1.1 1.8" />
      <path d="M12 17h.01" />
    </template>
    <template v-else-if="name === 'refresh'">
      <path d="M20 7.5V12h-4.5" />
      <path d="M19.2 12A7.4 7.4 0 1 0 17 17.2" />
    </template>
    <template v-else-if="name === 'plus'"><path d="M12 5v14M5 12h14" /></template>
    <template v-else-if="name === 'search'">
      <circle cx="10.7" cy="10.7" r="6.2" />
      <path d="m15.3 15.3 4.2 4.2" />
    </template>
    <template v-else-if="name === 'play'"><path d="M8.5 5.5 18 12l-9.5 6.5v-13Z" /></template>
    <template v-else-if="name === 'pause'">
      <path d="M9 6.5v11M15 6.5v11" stroke-width="2.6" />
    </template>
    <template v-else-if="name === 'stop'"><rect x="7" y="7" width="10" height="10" rx="1.7" /></template>
    <template v-else-if="name === 'power'"><path d="M12 3.5v8.2" /><path d="M6.7 6.4a8 8 0 1 0 10.6 0" /></template>
    <template v-else-if="name === 'copy'">
      <rect x="9" y="9" width="11" height="11" rx="2.2" />
      <path d="M15 9V6.2A2.2 2.2 0 0 0 12.8 4H6.2A2.2 2.2 0 0 0 4 6.2v6.6A2.2 2.2 0 0 0 6.2 15H9" />
    </template>
    <template v-else-if="name === 'more'">
      <circle cx="5" cy="12" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="19" cy="12" r="1.1" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'edit'">
      <path d="m4.5 19.5.9-4.2L16 4.7a2.3 2.3 0 0 1 3.3 3.3L8.7 18.6l-4.2.9Z" />
      <path d="m14.8 5.9 3.3 3.3" />
    </template>
    <template v-else-if="name === 'trash'">
      <path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l.8 12.2h9.4L17.5 7M10 11v4.5M14 11v4.5" />
    </template>
    <template v-else-if="name === 'sliders'">
      <path d="M4 7h6m4 0h6M4 17h10m4 0h2" />
      <circle cx="12" cy="7" r="2.2" />
      <circle cx="16" cy="17" r="2.2" />
    </template>
    <template v-else-if="name === 'activity'"><path d="M3 12h4l2.2-6 4.2 12 2.2-6H21" /></template>
    <template v-else-if="name === 'terminal'">
      <rect x="3.5" y="5" width="17" height="14" rx="2.5" />
      <path d="m7.5 9.5 3 2.5-3 2.5M13 15h4" />
    </template>
    <template v-else-if="name === 'zap'"><path d="M13.5 3 5 13.5h6L10.5 21 19 10.5h-6L13.5 3Z" /></template>
    <template v-else-if="name === 'chevron'"><path d="m9 6 6 6-6 6" /></template>
    <template v-else-if="name === 'chevron-down'"><path d="m6 9 6 6 6-6" /></template>
    <template v-else-if="name === 'close'"><path d="m6 6 12 12M18 6 6 18" /></template>
    <template v-else-if="name === 'check'"><path d="m5 12.5 4.3 4.3L19 7" /></template>
    <template v-else-if="name === 'link'">
      <path d="M10.5 13.5a4.2 4.2 0 0 0 6 0l2.4-2.4a4.2 4.2 0 1 0-6-6l-1.2 1.2" />
      <path d="M13.5 10.5a4.2 4.2 0 0 0-6 0l-2.4 2.4a4.2 4.2 0 1 0 6 6l1.2-1.2" />
    </template>
    <template v-else-if="name === 'folder'">
      <path d="M3.5 7.5A1.5 1.5 0 0 1 5 6h4l2 2.5h8A1.5 1.5 0 0 1 20.5 10v8A1.5 1.5 0 0 1 19 19.5H5A1.5 1.5 0 0 1 3.5 18V7.5Z" />
    </template>
    <template v-else-if="name === 'file'">
      <path d="M6 3.5h7l5 5v12H6v-17Z" />
      <path d="M13 3.5v5h5" />
    </template>
    <template v-else-if="name === 'clock'">
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
    </template>
    <template v-else-if="name === 'monitor'">
      <rect x="3" y="4.5" width="18" height="12" rx="2" />
      <path d="M8.5 20h7M12 16.5V20" />
    </template>
    <template v-else-if="name === 'moon'">
      <path d="M20 13.2A8.3 8.3 0 0 1 10.8 4 8.5 8.5 0 1 0 20 13.2Z" />
    </template>
    <template v-else-if="name === 'book'">
      <path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v16.5H6.5A2.5 2.5 0 0 0 4 22V5.5Z" />
      <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
    </template>
    <template v-else-if="name === 'up'"><path d="m6 14 6-6 6 6" /></template>
    <template v-else-if="name === 'down'"><path d="m6 10 6 6 6-6" /></template>
    <template v-else-if="name === 'reload'">
      <path d="M20 7.5V12h-4.5" />
      <path d="M19.2 12A7.4 7.4 0 1 0 17 17.2" />
    </template>
    <template v-else><circle cx="12" cy="12" r="8.5" /></template>
  </svg>
</template>
