<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps({
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const password = ref('')
const code = ref('')

function submit() {
  if (!password.value || code.value.trim().length < 6 || props.busy) return
  emit('submit', {
    password: password.value,
    code: code.value.trim(),
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
    <form
      class="modal"
      role="dialog"
      aria-modal="true"
      @submit.prevent="submit"
    >
      <h2 class="modal-title">关闭双重认证</h2>
      <p class="modal-sub">
        关闭后，登录此账号时不再要求动态验证码。请输入当前密码和验证器中的 6 位验证码以确认操作。
      </p>
      <div class="modal-body">
        <div class="field">
          <label class="label" for="tfa-disable-password">当前密码</label>
          <input
            id="tfa-disable-password"
            v-model="password"
            class="input"
            type="password"
            autocomplete="current-password"
            placeholder="••••••••"
            autofocus
          />
        </div>
        <div class="field">
          <label class="label" for="tfa-disable-code">
            验证码
            <span class="hint">身份验证器中的动态码</span>
          </label>
          <input
            id="tfa-disable-code"
            v-model="code"
            class="input tfa-code"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="8"
            placeholder="000000"
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
          class="btn btn-danger"
          :disabled="busy || !password || code.trim().length < 6"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '关闭中…' : '关闭双重认证' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.tfa-code {
  height: 42px;
  font-family: var(--font-mono);
  font-size: 18px;
  letter-spacing: 6px;
  text-align: center;
}
</style>
