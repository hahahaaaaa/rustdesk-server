<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps({
  user: { type: Object, default: null },
  self: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const username = ref(props.user?.name || '')
const displayName = ref(props.user?.display_name || '')
const email = ref(props.user?.email || '')
const password = ref('')
const note = ref(props.user?.note || '')
const isAdmin = ref(props.user?.is_admin === true)
const enabled = ref(props.user ? props.user.status === 1 : true)
const localError = ref('')

const isEdit = computed(() => Boolean(props.user))

function submit() {
  if (props.busy) return
  const name = username.value.trim()
  if (!isEdit.value && name.length < 3) {
    localError.value = '用户名至少需要 3 个字符。'
    return
  }
  if ((!isEdit.value || password.value) && password.value.length < 6) {
    localError.value = '密码至少需要 6 个字符。'
    return
  }
  localError.value = ''
  const payload = {
    display_name: displayName.value.trim(),
    email: email.value.trim(),
    note: note.value.trim(),
    is_admin: isAdmin.value,
  }
  if (isEdit.value) {
    if (password.value) payload.password = password.value
    if (!props.self) payload.status = enabled.value ? 1 : 0
  } else {
    payload.username = name
    payload.password = password.value
  }
  emit('submit', payload)
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
      <h2 class="modal-title">{{ isEdit ? '编辑用户' : '新建用户' }}</h2>
      <p class="modal-sub">
        {{
          isEdit
            ? `更新 ${user.display_name || user.name} 的资料、角色与登录密码。`
            : '创建账号后，用户即可登录客户端与控制台。'
        }}
      </p>
      <div class="modal-body">
        <div v-if="!isEdit" class="field">
          <label class="label" for="user-name">用户名</label>
          <input
            id="user-name"
            v-model="username"
            class="input"
            placeholder="例如：alice"
            autocomplete="off"
            autofocus
          />
        </div>
        <div v-else class="field">
          <span class="label">用户名</span>
          <div class="readonly-value">@{{ user.name }}</div>
        </div>
        <div class="field">
          <label class="label" for="user-display-name">
            显示名称 <span class="hint">可选</span>
          </label>
          <input
            id="user-display-name"
            v-model="displayName"
            class="input"
            placeholder="例如：Alice Chen"
            autocomplete="off"
          />
        </div>
        <div class="field">
          <label class="label" for="user-email">
            邮箱 <span class="hint">可选</span>
          </label>
          <input
            id="user-email"
            v-model="email"
            class="input"
            type="email"
            placeholder="name@example.com"
            autocomplete="off"
          />
        </div>
        <div class="field">
          <label class="label" for="user-password">
            密码
            <span class="hint">{{ isEdit ? '留空则不修改' : '至少 6 个字符' }}</span>
          </label>
          <input
            id="user-password"
            v-model="password"
            class="input"
            type="password"
            :placeholder="isEdit ? '保持当前密码' : '设置登录密码'"
            autocomplete="new-password"
          />
        </div>
        <div class="field">
          <label class="label" for="user-note">
            备注 <span class="hint">可选</span>
          </label>
          <textarea
            id="user-note"
            v-model="note"
            class="textarea"
            placeholder="团队、职责或其他说明"
            maxlength="240"
          />
        </div>
        <label class="check-row">
          <input v-model="isAdmin" type="checkbox" :disabled="self" />
          <span>
            管理员权限
            <small>{{ self ? '不能取消当前登录账号的管理员权限。' : '可以管理全部设备、用户与分组。' }}</small>
          </span>
        </label>
        <label v-if="isEdit" class="check-row">
          <input v-model="enabled" type="checkbox" :disabled="self" />
          <span>
            启用账号
            <small>{{ self ? '不能停用当前登录的账号。' : '停用后该账号无法登录，已签发的令牌立即失效。' }}</small>
          </span>
        </label>
        <p v-if="localError || error" class="field-error" style="margin-top: 10px">
          {{ localError || error }}
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
          :disabled="busy || (!isEdit && !username.trim())"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '保存中…' : isEdit ? '保存用户' : '创建用户' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.readonly-value {
  height: 36px;
  display: flex;
  align-items: center;
  padding: 0 11px;
  border-radius: var(--radius-sm);
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  font-family: var(--font-mono);
  font-size: 12.5px;
  color: var(--muted);
}

.check-row {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  margin-top: 14px;
  font-size: 12.5px;
  color: var(--ink-soft);
  cursor: pointer;
}

.check-row input {
  margin-top: 2px;
  accent-color: var(--ink);
}

.check-row small {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--faint);
}
</style>
