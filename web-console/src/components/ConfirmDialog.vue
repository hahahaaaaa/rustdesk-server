<script setup>
import { onBeforeUnmount, onMounted } from 'vue'

const props = defineProps({
  title: { type: String, required: true },
  message: { type: String, default: '' },
  confirmLabel: { type: String, default: '确认' },
  danger: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})

const emit = defineEmits(['cancel', 'confirm'])

function cancel() {
  if (!props.busy) emit('cancel')
}

function onKey(event) {
  if (event.key === 'Escape') cancel()
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="modal-scrim" @click.self="cancel">
    <div class="modal" role="alertdialog" aria-modal="true">
      <h2 class="modal-title">{{ title }}</h2>
      <p v-if="message" class="modal-sub">{{ message }}</p>
      <div class="modal-actions">
        <button class="btn btn-secondary" :disabled="busy" @click="cancel">
          取消
        </button>
        <button
          class="btn"
          :class="danger ? 'btn-danger' : 'btn-primary'"
          :disabled="busy"
          @click="emit('confirm')"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '处理中…' : confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
