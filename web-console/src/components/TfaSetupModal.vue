<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps({
  setup: { type: Object, required: true },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const code = ref('')
const copied = ref('')
let copyTimer = 0

function submit() {
  const value = code.value.trim()
  if (value.length < 6 || props.busy) return
  emit('submit', value)
}

async function copy(value, key) {
  try {
    await navigator.clipboard.writeText(value)
    copied.value = key
    clearTimeout(copyTimer)
    copyTimer = window.setTimeout(() => {
      copied.value = ''
    }, 1600)
  } catch {
    copied.value = ''
  }
}

function onKey(event) {
  if (event.key === 'Escape' && !props.busy) emit('cancel')
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  clearTimeout(copyTimer)
})
</script>

<template>
  <div class="modal-scrim" @click.self="!busy && emit('cancel')">
    <form
      class="modal tfa-modal"
      role="dialog"
      aria-modal="true"
      @submit.prevent="submit"
    >
      <h2 class="modal-title">启用双重认证</h2>
      <p class="modal-sub">
        在身份验证器应用（例如 Google Authenticator、Microsoft
        Authenticator）中选择“手动输入密钥”，然后输入下面这组密钥。
      </p>
      <div class="modal-body">
        <div class="secret-box">
          <div class="secret-meta">
            <span>账户</span>
            <strong>{{ setup.account }}</strong>
            <span>·</span>
            <span>{{ setup.issuer }}</span>
          </div>
          <code class="secret-value">{{ setup.secret }}</code>
          <div class="secret-actions">
            <button
              class="btn btn-secondary btn-sm"
              type="button"
              @click="copy(setup.secret, 'secret')"
            >
              {{ copied === 'secret' ? '已复制' : '复制密钥' }}
            </button>
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              @click="copy(setup.otpauth_url, 'url')"
            >
              {{ copied === 'url' ? '已复制链接' : '复制配置链接' }}
            </button>
          </div>
        </div>

        <div class="field">
          <label class="label" for="tfa-setup-code">
            6 位验证码
            <span class="hint">输入应用显示的当前动态码</span>
          </label>
          <input
            id="tfa-setup-code"
            v-model="code"
            class="input tfa-code"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="8"
            placeholder="000000"
            autofocus
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
          :disabled="busy || code.trim().length < 6"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '启用中…' : '启用双重认证' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.tfa-modal {
  width: min(480px, 100%);
}

.secret-box {
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--wash);
}

.secret-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  color: var(--muted);
}

.secret-meta strong {
  color: var(--ink);
  font-weight: 600;
}

.secret-value {
  display: block;
  margin-top: 8px;
  font-family: var(--font-mono);
  font-size: 15px;
  letter-spacing: 1.5px;
  color: var(--ink);
  word-break: break-all;
}

.secret-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.tfa-code {
  height: 44px;
  font-family: var(--font-mono);
  font-size: 18px;
  letter-spacing: 6px;
  text-align: center;
}
</style>
