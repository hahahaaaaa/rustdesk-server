<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { FolderLock } from 'lucide-vue-next'

const props = defineProps({
  user: { type: Object, required: true },
  groups: { type: Array, default: () => [] },
  selected: { type: Array, default: () => [] },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const checked = ref(new Set(props.selected))

watch(
  () => props.selected,
  (selected) => {
    checked.value = new Set(selected)
  }
)

function toggle(guid) {
  const next = new Set(checked.value)
  if (next.has(guid)) next.delete(guid)
  else next.add(guid)
  checked.value = next
}

function submit() {
  if (props.busy) return
  emit('submit', [...checked.value])
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
      <h2 class="modal-title">设备分组授权</h2>
      <p class="modal-sub">
        选择 {{ user.display_name || user.name }} 可以管理的设备分组。授权后，该用户可以在客户端和控制台看到组内设备。
      </p>
      <div class="modal-body">
        <div v-if="!groups.length" class="access-empty">
          <FolderLock :size="18" />
          <p>还没有设备分组。请先在“分组”页面创建分组。</p>
        </div>
        <div v-else class="access-list">
          <label v-for="group in groups" :key="group.guid" class="access-option">
            <input
              type="checkbox"
              :checked="checked.has(group.guid)"
              @change="toggle(group.guid)"
            />
            <span class="access-body">
              <strong>{{ group.name }}</strong>
              <small>{{ group.note || '暂无备注' }}</small>
            </span>
            <span class="tag">{{ group.device_count || 0 }} 台</span>
          </label>
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
.access-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 320px;
  overflow-y: auto;
  padding: 2px;
}

.access-option {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 11px;
  border-radius: var(--radius-sm);
  box-shadow: inset 0 0 0 1px var(--line);
  cursor: pointer;
  transition: background 0.12s ease;
}

.access-option:hover {
  background: var(--wash);
}

.access-option input {
  accent-color: var(--ink);
}

.access-body {
  min-width: 0;
  flex: 1;
}

.access-body strong {
  display: block;
  font-size: 12.5px;
  font-weight: 500;
}

.access-body small {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.access-empty {
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

.access-empty svg {
  flex: none;
  margin-top: 1px;
}
</style>
