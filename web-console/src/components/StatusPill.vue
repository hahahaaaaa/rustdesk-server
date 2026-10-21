<script setup>
import { computed } from 'vue'

const props = defineProps({
  state: { type: String, required: true },
})

const label = computed(
  () =>
    ({
      online: '在线',
      offline: '离线',
      disabled: '已禁用',
    })[props.state] || '未知'
)
</script>

<template>
  <span class="pill" :class="`pill-${state}`">
    <span class="dot" />
    {{ label }}
  </span>
</template>
