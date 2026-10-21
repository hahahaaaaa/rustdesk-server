<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  Check,
  Copy,
  Globe,
  MonitorPlay,
  Power,
  PowerOff,
  Trash2,
  X,
} from 'lucide-vue-next'
import ConfirmDialog from './ConfirmDialog.vue'
import OsIcon from './OsIcon.vue'
import StatusPill from './StatusPill.vue'
import { getApi } from '../lib/api'
import {
  openNativeControl,
  openWebControl,
  webClientUrl,
} from '../lib/control'
import {
  deviceGuid,
  deviceName,
  deviceOs,
  deviceState,
  exactTime,
  formatId,
  osLabel,
  timeAgo,
} from '../lib/format'
import { session } from '../lib/session'
import { notify } from '../lib/toast'

const props = defineProps({
  device: { type: Object, required: true },
  groups: { type: Array, default: () => [] },
  users: { type: Array, default: () => [] },
})

const emit = defineEmits(['close', 'updated', 'deleted'])

const api = getApi()
const guid = computed(() => deviceGuid(props.device))
const name = computed(() => deviceName(props.device))
const os = computed(() => deviceOs(props.device))
const state = computed(() => deviceState(props.device))
const online = computed(() => state.value === 'online')
const targetWebUrl = computed(() => webClientUrl())
const webReady = computed(
  () => online.value && !session.demo && Boolean(targetWebUrl.value)
)
const controlHint = computed(() => {
  if (!online.value) return '设备离线或已禁用，无法发起远程控制。'
  if (session.demo) return '演示模式不连接真实服务器，网页远控不可用。'
  if (!targetWebUrl.value) return '请先在设置中配置网页远控地址。'
  return '网页远控将在新标签页打开 RustDesk Server Pro 网页客户端，并自动复制设备 ID。'
})

const renamed = ref(name.value)
const note = ref(props.device.note || '')
const owner = ref(props.device.user_name || '')
const group = ref(props.device.device_group_name || '')
const busy = ref('')
const error = ref('')
const deleteOpen = ref(false)
const copied = ref(false)

watch(
  () => props.device,
  (device) => {
    renamed.value = deviceName(device)
    note.value = device.note || ''
    owner.value = device.user_name || ''
    group.value = device.device_group_name || ''
  }
)

const ownerOptions = computed(() => {
  const names = new Set(
    props.users.map((user) => user.name).filter(Boolean)
  )
  if (props.device.user_name) names.add(props.device.user_name)
  return [...names].sort((a, b) => a.localeCompare(b))
})

const groupOptions = computed(() => {
  const names = new Set(props.groups.map((item) => item.name).filter(Boolean))
  if (props.device.device_group_name) names.add(props.device.device_group_name)
  return [...names].sort((a, b) => a.localeCompare(b))
})

async function run(key, action, message) {
  busy.value = key
  error.value = ''
  try {
    await action()
    notify(message)
    emit('updated', guid.value)
  } catch (e) {
    error.value = e.message || '操作未能完成。'
    notify(error.value, 'error')
  } finally {
    busy.value = ''
  }
}

function controlNative() {
  openNativeControl(props.device)
}

function controlWeb() {
  openWebControl(props.device)
}

function rename() {
  const value = renamed.value.trim()
  if (!value) {
    error.value = '设备名称不能为空。'
    return
  }
  if (value === name.value) return
  run(
    'rename',
    () => api.assignDevice(guid.value, 'device_name', value),
    '设备已重命名。'
  )
}

function saveNote() {
  if (note.value === (props.device.note || '')) return
  run(
    'note',
    () => api.assignDevice(guid.value, 'note', note.value),
    '备注已保存。'
  )
}

function saveOwner() {
  if (owner.value === (props.device.user_name || '')) return
  run(
    'owner',
    () => api.assignDevice(guid.value, 'user_name', owner.value),
    owner.value ? '负责人已更新。' : '已清除负责人。'
  )
}

function saveGroup() {
  if (group.value === (props.device.device_group_name || '')) return
  run(
    'group',
    () => api.assignDevice(guid.value, 'device_group_name', group.value),
    group.value ? '设备分组已更新。' : '设备已移出分组。'
  )
}

function toggleDisabled() {
  const disable = !props.device.disabled
  run(
    'power',
    () => api.setDeviceDisabled(guid.value, disable),
    disable ? '设备已禁用。' : '设备已启用。'
  )
}

async function deleteDevice() {
  busy.value = 'delete'
  error.value = ''
  try {
    await api.deleteDevice(guid.value)
    notify('设备已删除。')
    emit('deleted', guid.value)
  } catch (e) {
    error.value = e.message || '设备删除失败。'
    notify(error.value, 'error')
    deleteOpen.value = false
  } finally {
    busy.value = ''
  }
}

async function copyId() {
  try {
    await navigator.clipboard.writeText(String(props.device.id || ''))
    copied.value = true
    notify('设备 ID 已复制。')
    window.setTimeout(() => {
      copied.value = false
    }, 1600)
  } catch {
    error.value = '浏览器阻止了剪贴板访问。'
  }
}

function onKey(event) {
  if (event.key !== 'Escape') return
  if (deleteOpen.value) {
    deleteOpen.value = false
  } else {
    emit('close')
  }
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="overlay" @click="emit('close')" />
  <aside class="drawer" role="dialog" aria-modal="true" aria-label="设备详情">
    <header class="drawer-head">
      <div class="device-icon"><OsIcon :os="os" :size="18" /></div>
      <div class="drawer-title-wrap">
        <div class="drawer-title">{{ name }}</div>
        <div class="drawer-sub">
          <span>{{ formatId(device.id) }}</span>
          <button
            class="icon-btn"
            type="button"
            style="width: 22px; height: 22px"
            title="复制设备 ID"
            @click="copyId"
          >
            <Check v-if="copied" :size="12" />
            <Copy v-else :size="12" />
          </button>
        </div>
      </div>
      <StatusPill :state="state" />
      <button
        class="icon-btn"
        type="button"
        title="关闭"
        aria-label="关闭"
        @click="emit('close')"
      >
        <X :size="18" />
      </button>
    </header>

    <div class="drawer-body">
      <div
        v-if="error"
        class="login-error"
        style="margin-top: 0; margin-bottom: 18px"
      >
        {{ error }}
      </div>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">远程控制</span>
        </div>
        <div class="drawer-control">
          <button
            class="btn btn-primary"
            type="button"
            :disabled="!device.id || !online"
            @click="controlNative"
          >
            <MonitorPlay :size="15" />
            发起远程控制
          </button>
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="!webReady"
            @click="controlWeb"
          >
            <Globe :size="15" />
            网页远控
          </button>
        </div>
        <p class="control-note">{{ controlHint }}</p>
      </section>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">概览</span>
        </div>
        <dl class="detail-list">
          <div class="detail-row">
            <dt>平台</dt>
            <dd>
              {{ osLabel(os) }}
              <span v-if="device.info?.version" class="faint">
                · v{{ device.info.version }}
              </span>
            </dd>
          </div>
          <div class="detail-row">
            <dt>设备用户</dt>
            <dd>{{ device.info?.username || '—' }}</dd>
          </div>
          <div class="detail-row">
            <dt>主机名</dt>
            <dd class="mono">{{ device.info?.hostname || '—' }}</dd>
          </div>
          <div class="detail-row">
            <dt>负责人</dt>
            <dd>{{ device.user_name || '未分配' }}</dd>
          </div>
          <div class="detail-row">
            <dt>分组</dt>
            <dd>{{ device.device_group_name || '未分组' }}</dd>
          </div>
          <div class="detail-row">
            <dt>最后在线</dt>
            <dd>
              {{ timeAgo(device.last_online) }}
              <span class="faint">· {{ exactTime(device.last_online) }}</span>
            </dd>
          </div>
          <div v-if="device.info?.cpu" class="detail-row">
            <dt>CPU</dt>
            <dd>{{ device.info.cpu }}</dd>
          </div>
        </dl>
      </section>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">重命名设备</span>
        </div>
        <div class="inline-form">
          <input
            v-model="renamed"
            class="input"
            maxlength="64"
            placeholder="设备名称"
            @keyup.enter="rename"
          />
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="busy === 'rename'"
            @click="rename"
          >
            <span v-if="busy === 'rename'" class="spinner" />
            保存
          </button>
        </div>
      </section>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">访问归属</span>
        </div>
        <div class="field">
          <label class="label" for="device-owner">负责人</label>
          <div class="inline-form">
            <select id="device-owner" v-model="owner" class="select">
              <option value="">未分配</option>
              <option v-for="name in ownerOptions" :key="name" :value="name">
                {{ name }}
              </option>
            </select>
            <button
              class="btn btn-secondary"
              type="button"
              :disabled="busy === 'owner'"
              @click="saveOwner"
            >
              保存
            </button>
          </div>
        </div>
        <div class="field">
          <label class="label" for="device-group">设备分组</label>
          <div class="inline-form">
            <select id="device-group" v-model="group" class="select">
              <option value="">不分组</option>
              <option v-for="name in groupOptions" :key="name" :value="name">
                {{ name }}
              </option>
            </select>
            <button
              class="btn btn-secondary"
              type="button"
              :disabled="busy === 'group'"
              @click="saveGroup"
            >
              保存
            </button>
          </div>
        </div>
      </section>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">备注</span>
        </div>
        <textarea
          v-model="note"
          class="textarea"
          maxlength="240"
          placeholder="为此设备添加内部备注"
        />
        <div style="display: flex; justify-content: flex-end; margin-top: 8px">
          <button
            class="btn btn-secondary btn-sm"
            type="button"
            :disabled="busy === 'note'"
            @click="saveNote"
          >
            {{ busy === 'note' ? '保存中…' : '保存备注' }}
          </button>
        </div>
      </section>

      <section class="drawer-section">
        <div class="section-head">
          <span class="mono-label">管理</span>
        </div>
        <div class="action-grid">
          <button
            class="action-tile"
            type="button"
            :disabled="busy === 'power'"
            @click="toggleDisabled"
          >
            <PowerOff v-if="!device.disabled" :size="15" />
            <Power v-else :size="15" />
            {{ device.disabled ? '启用设备' : '禁用设备' }}
          </button>
          <button class="action-tile" type="button" @click="copyId">
            <Copy :size="15" />
            复制设备 ID
          </button>
        </div>

        <div class="danger-zone" style="margin-top: 14px">
          <h4>删除设备</h4>
          <p>
            从服务器移除此设备。客户端需要重新注册后才能再次连接。
          </p>
          <button
            class="btn btn-danger-ghost btn-sm"
            type="button"
            @click="deleteOpen = true"
          >
            <Trash2 :size="14" />
            删除设备
          </button>
        </div>
      </section>
    </div>
  </aside>

  <ConfirmDialog
    v-if="deleteOpen"
    title="删除此设备？"
    :message="`${name} 将从 RustDesk 服务器中移除，此操作无法撤销。`"
    confirm-label="删除设备"
    danger
    :busy="busy === 'delete'"
    @cancel="deleteOpen = false"
    @confirm="deleteDevice"
  />
</template>
