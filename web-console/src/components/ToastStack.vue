<script setup>
import { AlertCircle, CheckCircle2 } from 'lucide-vue-next'
import { dismiss, toasts } from '../lib/toast'
</script>

<template>
  <div class="toast-stack" aria-live="polite">
    <div
      v-for="toast in toasts"
      :key="toast.id"
      class="toast"
      :class="{ error: toast.type === 'error' }"
      @click="dismiss(toast.id)"
    >
      <AlertCircle v-if="toast.type === 'error'" :size="15" />
      <CheckCircle2 v-else :size="15" />
      <span>{{ toast.message }}</span>
    </div>
  </div>
</template>
