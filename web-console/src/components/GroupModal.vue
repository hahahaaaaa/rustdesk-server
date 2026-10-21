<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps({
  group: { type: Object, default: null },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const name = ref(props.group?.name || '')
const note = ref(props.group?.note || '')

function submit() {
  if (!name.value.trim() || props.busy) return
  emit('submit', { name: name.value.trim(), note: note.value.trim() })
}

function onKey(event) {
  if (event.key === 'Escape' && !props.busy) emit('cancel')
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="modal-scrim" @click.self="!busy && emit('cancel')">
    <form class="modal" role="dialog" aria-modal="true" @submit.prevent="submit">
      <h2 class="modal-title">
        {{ group ? '重命名分组' : '新建设备分组' }}
      </h2>
      <p class="modal-sub">
        {{
          group
            ? '更新分组名称与备注，设备的归属保持不变。'
            : '将设备分组，便于应用访问规则，也让设备管理更清晰。'
        }}
      </p>
      <div class="modal-body">
        <div class="field">
          <label class="label" for="group-name">名称</label>
          <input
            id="group-name"
            v-model="name"
            class="input"
            placeholder="例如：设计部"
            maxlength="48"
            autofocus
          />
        </div>
        <div class="field">
          <label class="label" for="group-note">
            备注 <span class="hint">可选</span>
          </label>
          <textarea
            id="group-note"
            v-model="note"
            class="textarea"
            placeholder="这个分组用于什么？"
            maxlength="240"
          />
        </div>
        <p v-if="error" class="field-error" style="margin-top: 10px">
          {{ error }}
        </p>
      </div>
      <div class="modal-actions">
        <button
          type="button"
          class="btn btn-secondary"
          :disabled="busy"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button
          type="submit"
          class="btn btn-primary"
          :disabled="busy || !name.trim()"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '保存中…' : group ? '保存分组' : '创建分组' }}
        </button>
      </div>
    </form>
  </div>
</template>
