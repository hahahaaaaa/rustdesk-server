<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps({
  book: { type: Object, default: null },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const name = ref(props.book?.name || '')
const note = ref(props.book?.note || '')
const password = ref(props.book?.info?.password || '')

function submit() {
  if (!name.value.trim() || props.busy) return
  emit('submit', {
    name: name.value.trim(),
    note: note.value.trim(),
    password: password.value.trim(),
  })
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
        {{ book ? '编辑共享地址簿' : '新建共享地址簿' }}
      </h2>
      <p class="modal-sub">
        共享地址簿由管理员维护，可授权给指定用户，在客户端中直接使用。
      </p>
      <div class="modal-body">
        <div class="field">
          <label class="label" for="ab-name">名称</label>
          <input
            id="ab-name"
            v-model="name"
            class="input"
            placeholder="例如：运维工具"
            maxlength="48"
            autofocus
          />
        </div>
        <div class="field">
          <label class="label" for="ab-note">
            备注 <span class="hint">可选</span>
          </label>
          <textarea
            id="ab-note"
            v-model="note"
            class="textarea"
            placeholder="这个地址簿用于什么？"
            maxlength="240"
          />
        </div>
        <div class="field">
          <label class="label" for="ab-password">
            访问密码 <span class="hint">可选</span>
          </label>
          <input
            id="ab-password"
            v-model="password"
            class="input"
            type="text"
            placeholder="留空表示不设置密码"
            maxlength="64"
            autocomplete="new-password"
          />
          <span class="hint" style="font-size: 11px">
            客户端访问该地址簿时使用，修改后需重新同步。
          </span>
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
          {{ busy ? '保存中…' : book ? '保存地址簿' : '创建地址簿' }}
        </button>
      </div>
    </form>
  </div>
</template>
