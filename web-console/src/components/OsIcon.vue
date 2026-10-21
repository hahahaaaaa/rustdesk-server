<script setup>
import { Laptop, Monitor, Smartphone, Terminal } from 'lucide-vue-next'

defineProps({
  os: { type: String, default: 'other' },
  size: { type: [Number, String], default: 16 },
})
</script>

<template>
  <Monitor v-if="os === 'windows'" :size="size" />
  <Laptop v-else-if="os === 'macos'" :size="size" />
  <Terminal v-else-if="os === 'linux'" :size="size" />
  <Smartphone v-else-if="os === 'android' || os === 'ios'" :size="size" />
  <Monitor v-else :size="size" />
</template>
