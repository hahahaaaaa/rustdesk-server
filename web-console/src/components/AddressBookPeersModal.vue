<script setup>
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { Plus, RefreshCw, Server, Tag, Trash2, X } from 'lucide-vue-next'
import { osLabel } from '../lib/format'

const props = defineProps({
  book: { type: Object, required: true },
  peers: { type: Array, default: () => [] },
  tags: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'refresh', 'add', 'remove', 'add-tag'])

const form = reactive({
  id: '',
  alias: '',
  password: '',
  note: '',
  tags: [],
})
const newTag = ref('')
const confirming = ref('')
const adding = ref(false)

const canAdd = computed(() => form.id.trim().length > 0 && !props.busy)

watch(
  () => props.peers.length,
  (length, previous) => {
    if (length > previous) {
      form.id = ''
      form.alias = ''
      form.password = ''
      form.note = ''
      form.tags = []
    }
  }
)

watch(
  () => props.tags.length,
  (length, previous) => {
    if (length > previous) newTag.value = ''
  }
)

function toggleTag(name) {
  const index = form.tags.indexOf(name)
  if (index >= 0) form.tags.splice(index, 1)
  else form.tags.push(name)
}

function submit() {
  if (!canAdd.value) return
  emit('add', {
    id: form.id.trim(),
    alias: form.alias.trim(),
    password: form.password,
    note: form.note.trim(),
    tags: [...form.tags],
  })
}

function submitTag() {
  const name = newTag.value.trim()
  if (!name || props.busy) return
  emit('add-tag', name)
}

function removePeer(peer) {
  if (props.busy) return
  emit('remove', peer.id)
  confirming.value = ''
}

function tagColor(color) {
  const hex = (Number(color) >>> 0).toString(16).padStart(8, '0').slice(-6)
  return `#${hex}`
}

function onKey(event) {
  if (event.key === 'Escape' && !props.busy) emit('cancel')
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="modal-scrim" @click.self="!busy && emit('cancel')">
    <div class="modal peers-modal" role="dialog" aria-modal="true">
      <div class="peers-head">
        <div>
          <h2 class="modal-title">设备管理</h2>
          <p class="modal-sub">
            「{{ book.name }}」中的设备会同步到获授权用户的客户端。
          </p>
        </div>
        <button
          class="icon-btn"
          type="button"
          title="刷新设备"
          :disabled="loading"
          @click="emit('refresh')"
        >
          <RefreshCw
            :size="15"
            :style="loading ? { animation: 'spin .8s linear infinite' } : null"
          />
        </button>
      </div>

      <div class="modal-body">
        <form class="peer-form" @submit.prevent="submit">
          <div class="peer-form-grid">
            <div class="field">
              <label class="label" for="peer-id">设备 ID</label>
              <input
                id="peer-id"
                v-model="form.id"
                class="input"
                placeholder="例如：123456789"
                maxlength="64"
                autocomplete="off"
              />
            </div>
            <div class="field">
              <label class="label" for="peer-alias">别名</label>
              <input
                id="peer-alias"
                v-model="form.alias"
                class="input"
                placeholder="可选"
                maxlength="64"
              />
            </div>
            <div class="field">
              <label class="label" for="peer-password">连接密码</label>
              <input
                id="peer-password"
                v-model="form.password"
                class="input"
                type="text"
                placeholder="可选"
                maxlength="64"
                autocomplete="new-password"
              />
            </div>
            <div class="field">
              <label class="label" for="peer-note">备注</label>
              <input
                id="peer-note"
                v-model="form.note"
                class="input"
                placeholder="可选"
                maxlength="120"
              />
            </div>
          </div>

          <div class="tag-line">
            <span class="tag-line-label"><Tag :size="12" /> 标签</span>
            <div v-if="tags.length" class="tag-picks">
              <button
                v-for="tag in tags"
                :key="tag.name"
                class="tag-pick"
                :class="{ active: form.tags.includes(tag.name) }"
                type="button"
                @click="toggleTag(tag.name)"
              >
                <span class="tag-dot" :style="{ background: tagColor(tag.color) }" />
                {{ tag.name }}
              </button>
            </div>
            <span v-else class="faint" style="font-size: 11.5px">暂无标签</span>
          </div>

          <div class="peer-form-foot">
            <div class="new-tag">
              <input
                v-model="newTag"
                class="input"
                placeholder="新标签名称"
                maxlength="24"
                aria-label="新标签名称"
                @keydown.enter.prevent="submitTag"
              />
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                :disabled="!newTag.trim() || busy"
                @click="submitTag"
              >
                <Plus :size="13" />
                添加标签
              </button>
            </div>
            <button class="btn btn-primary btn-sm" type="submit" :disabled="!canAdd">
              <span v-if="busy" class="spinner" />
              <Plus v-else :size="14" />
              添加设备
            </button>
          </div>
          <p v-if="error" class="field-error" style="margin-top: 8px">
            {{ error }}
          </p>
        </form>

        <div v-if="loading" class="loading-state" style="padding: 26px">
          <span class="spinner-dark" />
          正在加载设备
        </div>

        <div v-else-if="!peers.length" class="peers-empty">
          这个地址簿还没有设备，使用上方表单添加。
        </div>

        <div v-else class="peer-list">
          <div v-for="peer in peers" :key="peer.id" class="peer-row">
            <div class="peer-main">
              <div class="peer-name-line">
                <span class="peer-alias">{{ peer.alias || peer.id }}</span>
                <span v-if="peer.same_server" class="tag tag-blue">
                  <Server :size="10" />
                  本服务器
                </span>
                <span
                  v-for="tagName in peer.tags"
                  :key="tagName"
                  class="tag"
                >
                  {{ tagName }}
                </span>
              </div>
              <div class="peer-meta">
                <span class="mono">{{ peer.id }}</span>
                <template v-if="peer.platform">
                  · {{ osLabel(peer.platform) }}
                </template>
                <template v-if="peer.note">· {{ peer.note }}</template>
              </div>
            </div>
            <div v-if="confirming === peer.id" class="peer-confirm">
              <span>确认移除？</span>
              <button
                class="btn btn-danger btn-sm"
                type="button"
                :disabled="busy"
                @click="removePeer(peer)"
              >
                移除
              </button>
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                :disabled="busy"
                @click="confirming = ''"
              >
                取消
              </button>
            </div>
            <button
              v-else
              class="icon-btn danger"
              type="button"
              title="从地址簿移除"
              @click="confirming = peer.id"
            >
              <Trash2 :size="14" />
            </button>
          </div>
        </div>
      </div>

      <div class="modal-actions">
        <button
          class="btn btn-secondary"
          type="button"
          :disabled="busy"
          @click="emit('cancel')"
        >
          <X :size="14" />
          关闭
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.peers-modal {
  width: min(720px, 100%);
  max-height: min(90vh, 820px);
  display: flex;
  flex-direction: column;
}

.peers-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.peers-head > div {
  flex: 1;
}

.modal-body {
  min-height: 0;
  overflow-y: auto;
  margin-top: 16px;
  padding-right: 2px;
}

.peer-form {
  padding: 14px;
  border-radius: var(--radius-sm);
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
}

.peer-form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 12px;
}

.peer-form-grid .field + .field {
  margin-top: 0;
}

.tag-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}

.tag-line-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11.5px;
  color: var(--muted);
}

.tag-picks {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.tag-pick {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 8px;
  border: 0;
  border-radius: 4px;
  background: #fff;
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--muted);
  font-size: 11px;
}

.tag-pick.active {
  background: var(--ink);
  box-shadow: none;
  color: #fff;
}

.tag-dot {
  width: 6px;
  height: 6px;
  border-radius: 9999px;
}

.peer-form-foot {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 12px;
}

.new-tag {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
}

.new-tag .input {
  max-width: 180px;
  height: 30px;
}

.peers-empty {
  padding: 28px 16px;
  text-align: center;
  color: var(--muted);
  font-size: 12.5px;
}

.peer-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 12px;
}

.peer-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  box-shadow: inset 0 0 0 1px var(--line);
}

.peer-main {
  min-width: 0;
  flex: 1;
}

.peer-name-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.peer-alias {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.peer-meta {
  margin-top: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: var(--muted);
}

.peer-confirm {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
  font-size: 11.5px;
  color: var(--red);
}

.modal-actions {
  margin-top: 16px;
}

@media (max-width: 640px) {
  .peer-form-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .peer-form-foot {
    flex-direction: column;
    align-items: stretch;
  }

  .new-tag {
    flex-direction: column;
    align-items: stretch;
  }

  .new-tag .input {
    max-width: none;
  }
}
</style>
