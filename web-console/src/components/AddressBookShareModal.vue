<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Search, Share2 } from 'lucide-vue-next'
import { initials } from '../lib/format'

const props = defineProps({
  book: { type: Object, required: true },
  users: { type: Array, default: () => [] },
  shares: { type: Array, default: () => [] },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const rules = ref({})
for (const share of props.shares) {
  rules.value[share.user_id] = share.rule
}
const search = ref('')

const RULE_OPTIONS = [
  { value: 0, label: '无权限' },
  { value: 1, label: '只读' },
  { value: 2, label: '读写' },
  { value: 3, label: '完全控制' },
]

const visibleUsers = computed(() => {
  const term = search.value.trim().toLowerCase()
  return props.users.filter(
    (user) =>
      !term ||
      `${user.name} ${user.display_name || ''} ${user.email || ''}`
        .toLowerCase()
        .includes(term)
  )
})

const sharedCount = computed(
  () => Object.values(rules.value).filter((rule) => Number(rule) > 0).length
)

function submit() {
  if (props.busy) return
  const shares = Object.entries(rules.value)
    .map(([userId, rule]) => ({ userId: Number(userId), rule: Number(rule) }))
    .filter((share) => share.rule > 0)
  emit('submit', shares)
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
      class="modal modal-wide"
      role="dialog"
      aria-modal="true"
      @submit.prevent="submit"
    >
      <h2 class="modal-title">共享授权</h2>
      <p class="modal-sub">
        「{{ book.name }}」当前已授权 {{ sharedCount }} 位用户。只读用户只能查看设备，
        读写及完全控制用户可以增删设备与标签。
      </p>
      <div class="modal-body">
        <div class="search" style="max-width: none; margin-bottom: 10px">
          <Search :size="14" />
          <input
            v-model="search"
            class="input"
            type="search"
            placeholder="搜索用户名或邮箱"
            aria-label="搜索用户"
          />
        </div>

        <div v-if="!visibleUsers.length" class="share-empty">
          <Share2 :size="18" />
          <p>{{ users.length ? '没有匹配的用户。' : '还没有可授权的用户。' }}</p>
        </div>

        <div v-else class="share-list">
          <div v-for="user in visibleUsers" :key="user.id" class="share-row">
            <span class="avatar-sm">{{ initials(user.display_name || user.name) }}</span>
            <div class="share-ident">
              <strong>{{ user.display_name || user.name }}</strong>
              <small>{{ user.name }}<template v-if="user.email"> · {{ user.email }}</template></small>
            </div>
            <span v-if="user.status === 0" class="tag">已禁用</span>
            <select
              v-model.number="rules[user.id]"
              class="select share-rule"
              :aria-label="`${user.name} 的权限`"
            >
              <option
                v-for="option in RULE_OPTIONS"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
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
        <button type="submit" class="btn btn-primary" :disabled="busy">
          <span v-if="busy" class="spinner" />
          {{ busy ? '保存中…' : '保存授权' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.modal-wide {
  width: min(560px, 100%);
  max-height: min(86vh, 760px);
  display: flex;
  flex-direction: column;
}

.modal-body {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.share-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 420px;
  overflow-y: auto;
  padding: 2px;
}

.share-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: var(--radius-sm);
  box-shadow: inset 0 0 0 1px var(--line);
}

.avatar-sm {
  display: grid;
  place-items: center;
  flex: none;
  width: 28px;
  height: 28px;
  border-radius: 9999px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.share-ident {
  min-width: 0;
  flex: 1;
}

.share-ident strong {
  display: block;
  font-size: 12.5px;
  font-weight: 500;
}

.share-ident small {
  display: block;
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: var(--muted);
}

.share-rule {
  width: 108px;
  flex: none;
  height: 32px;
}

.share-empty {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 16px;
  border-radius: var(--radius-sm);
  background: var(--wash);
  color: var(--muted);
  font-size: 12.5px;
  line-height: 1.55;
}

.share-empty svg {
  flex: none;
  margin-top: 1px;
}
</style>
