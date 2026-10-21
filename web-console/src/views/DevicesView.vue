<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Ban,
  ChevronLeft,
  ChevronRight,
  Download,
  MonitorPlay,
  MonitorSmartphone,
  MoreHorizontal,
  RefreshCw,
  Search,
  Trash2,
  Wifi,
  WifiOff,
  Power,
  PowerOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ControlDialog from '../components/ControlDialog.vue'
import DeviceDrawer from '../components/DeviceDrawer.vue'
import OsIcon from '../components/OsIcon.vue'
import StatusPill from '../components/StatusPill.vue'
import { getApi } from '../lib/api'
import {
  csvEscape,
  deviceGuid,
  deviceName,
  deviceOs,
  deviceState,
  exactTime,
  formatId,
  isOnline,
  osLabel,
  timeAgo,
} from '../lib/format'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const route = useRoute()
const router = useRouter()
const api = getApi()

const devices = ref([])
const groups = ref([])
const users = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const search = ref('')
const statusFilter = ref('all')
const platformFilter = ref('all')
const groupFilter = ref(
  typeof route.query.group === 'string' ? route.query.group : ''
)
const page = ref(1)
const pageSize = 12
const selected = ref(new Set())
const drawerDevice = ref(null)
const controlTarget = ref(null)
const deleteOpen = ref(false)
const bulkBusy = ref(false)

const statusOptions = [
  { value: 'all', label: '全部' },
  { value: 'online', label: '在线' },
  { value: 'offline', label: '离线' },
  { value: 'disabled', label: '已禁用' },
]

const stats = computed(() => {
  let online = 0
  let offline = 0
  let disabled = 0
  for (const device of devices.value) {
    const state = deviceState(device)
    if (state === 'online') online += 1
    else if (state === 'offline') offline += 1
    else disabled += 1
  }
  return { total: devices.value.length, online, offline, disabled }
})

const groupOptions = computed(() => {
  const names = new Set(groups.value.map((group) => group.name).filter(Boolean))
  for (const device of devices.value) {
    if (device.device_group_name) names.add(device.device_group_name)
  }
  return [...names].sort((a, b) => a.localeCompare(b))
})

const platformOptions = computed(() => {
  const values = new Set(devices.value.map(deviceOs))
  return [...values].sort((a, b) => osLabel(a).localeCompare(osLabel(b)))
})

const hasUngrouped = computed(() =>
  devices.value.some((device) => !device.device_group_name)
)

const hasFilters = computed(
  () =>
    Boolean(search.value.trim()) ||
    statusFilter.value !== 'all' ||
    platformFilter.value !== 'all' ||
    Boolean(groupFilter.value)
)

const filtered = computed(() => {
  const query = search.value.trim().toLowerCase().replace(/\s+/g, '')
  return devices.value.filter((device) => {
    if (
      statusFilter.value !== 'all' &&
      deviceState(device) !== statusFilter.value
    ) {
      return false
    }
    if (groupFilter.value === '__ungrouped__') {
      if (device.device_group_name) return false
    } else if (
      groupFilter.value &&
      device.device_group_name !== groupFilter.value
    ) {
      return false
    }
    if (
      platformFilter.value !== 'all' &&
      deviceOs(device) !== platformFilter.value
    ) {
      return false
    }
    if (query) {
      const haystack = [
        device.id,
        device.device_name,
        device.info?.device_name,
        device.info?.hostname,
        device.info?.username,
        device.user_name,
        device.device_group_name,
        device.note,
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
        .replace(/\s+/g, '')
      if (!haystack.includes(query)) return false
    }
    return true
  })
})

const pageCount = computed(() =>
  Math.max(1, Math.ceil(filtered.value.length / pageSize))
)
const paged = computed(() =>
  filtered.value.slice((page.value - 1) * pageSize, page.value * pageSize)
)
const rangeStart = computed(() =>
  filtered.value.length === 0 ? 0 : (page.value - 1) * pageSize + 1
)
const rangeEnd = computed(() =>
  Math.min(page.value * pageSize, filtered.value.length)
)

const selectedDevices = computed(() =>
  devices.value.filter((device) => selected.value.has(deviceGuid(device)))
)
const allPageSelected = computed(
  () =>
    paged.value.length > 0 &&
    paged.value.every((device) => selected.value.has(deviceGuid(device)))
)

watch([search, statusFilter, platformFilter, groupFilter], () => {
  page.value = 1
  selected.value = new Set()
})

watch(pageCount, (count) => {
  if (page.value > count) page.value = count
})

watch(
  () => route.query.group,
  (value) => {
    groupFilter.value = typeof value === 'string' ? value : ''
  }
)

watch([drawerDevice, controlTarget], ([drawer, control]) => {
  document.body.style.overflow = drawer || control ? 'hidden' : ''
})

onMounted(load)
onBeforeUnmount(() => {
  document.body.style.overflow = ''
})

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [deviceRows, groupRows] = await Promise.all([
      api.getDevices(),
      api.getGroups().catch(() => []),
    ])
    devices.value = [...deviceRows].sort((a, b) => {
      const onlineDelta = Number(isOnline(b)) - Number(isOnline(a))
      if (onlineDelta !== 0) return onlineDelta
      return deviceName(a).localeCompare(deviceName(b))
    })
    groups.value = groupRows
    try {
      users.value = await api.getUsers()
    } catch {
      users.value = []
    }
    selected.value = new Set()
  } catch (e) {
    error.value = e.message || '设备列表加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function setGroupFilter(value) {
  groupFilter.value = value
  const query = { ...route.query }
  if (value) query.group = value
  else delete query.group
  router.replace({ query })
}

function setPage(next) {
  page.value = Math.min(Math.max(next, 1), pageCount.value)
}

function toggleSelect(guid) {
  const next = new Set(selected.value)
  if (next.has(guid)) next.delete(guid)
  else next.add(guid)
  selected.value = next
}

function toggleSelectPage() {
  const next = new Set(selected.value)
  if (allPageSelected.value) {
    paged.value.forEach((device) => next.delete(deviceGuid(device)))
  } else {
    paged.value.forEach((device) => next.add(deviceGuid(device)))
  }
  selected.value = next
}

function openDevice(device) {
  drawerDevice.value = device
}

function openControl(device) {
  if (deviceState(device) !== 'online') {
    notify('设备不在线，无法发起远程控制。', 'error')
    return
  }
  controlTarget.value = device
}

async function handleDrawerUpdated(guid) {
  await load(false)
  drawerDevice.value =
    devices.value.find((device) => deviceGuid(device) === guid) || null
}

async function handleDrawerDeleted() {
  drawerDevice.value = null
  await load(false)
}

async function setSelectedDisabled(disabled) {
  const items = selectedDevices.value
  if (!items.length) return
  bulkBusy.value = true
  try {
    const results = await Promise.allSettled(
      items.map((device) =>
        api.setDeviceDisabled(deviceGuid(device), disabled)
      )
    )
    const failed = results.filter((result) => result.status === 'rejected')
    if (failed.length) {
      notify(
        `有 ${failed.length} 台设备（共 ${items.length} 台）更新失败。`,
        'error'
      )
    } else {
      notify(
        `已${disabled ? '禁用' : '启用'} ${items.length} 台设备。`
      )
    }
    selected.value = new Set()
    await load(false)
  } finally {
    bulkBusy.value = false
  }
}

async function deleteSelected() {
  const items = selectedDevices.value
  if (!items.length) return
  bulkBusy.value = true
  try {
    const results = await Promise.allSettled(
      items.map((device) => api.deleteDevice(deviceGuid(device)))
    )
    const failed = results.filter((result) => result.status === 'rejected')
    if (failed.length) {
      notify(
        `有 ${failed.length} 台设备（共 ${items.length} 台）删除失败。`,
        'error'
      )
    } else {
      notify(`已删除 ${items.length} 台设备。`)
    }
    selected.value = new Set()
    deleteOpen.value = false
    await load(false)
  } finally {
    bulkBusy.value = false
  }
}

function clearFilters() {
  search.value = ''
  statusFilter.value = 'all'
  platformFilter.value = 'all'
  if (route.query.group) setGroupFilter('')
  else groupFilter.value = ''
}

function exportCsv() {
  const header = [
    '设备 ID',
    '名称',
    '负责人',
    '分组',
    '平台',
    '版本',
    '状态',
    '最后在线',
    '备注',
  ]
  const lines = filtered.value.map((device) =>
    [
      device.id,
      deviceName(device),
      device.user_name || '',
      device.device_group_name || '',
      osLabel(deviceOs(device)),
      device.info?.version || '',
      deviceState(device),
      device.last_online || '',
      device.note || '',
    ]
      .map(csvEscape)
      .join(',')
  )
  const csv = `\uFEFF${[header.join(','), ...lines].join('\n')}`
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = 'rustdesk-devices.csv'
  anchor.click()
  URL.revokeObjectURL(url)
  notify(`已导出 ${filtered.value.length} 台设备。`)
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">设备总览</div>
          <h1 class="page-title">设备</h1>
          <p class="page-sub">
            查看所有已接入 RustDesk 服务器的设备，并在这里管理远程访问与归属。
          </p>
        </div>
        <div class="page-actions">
          <button class="btn btn-secondary" type="button" @click="exportCsv">
            <Download :size="15" />
            导出 CSV
          </button>
          <button
            class="btn btn-primary"
            type="button"
            :disabled="refreshing"
            @click="load(false)"
          >
            <RefreshCw
              :size="15"
              :style="
                refreshing ? { animation: 'spin .8s linear infinite' } : null
              "
            />
            刷新
          </button>
        </div>
      </div>

      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">设备总数</span>
            <MonitorSmartphone :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">
            本工作区共 {{ groupOptions.length }} 个分组
          </div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">在线</span>
            <Wifi :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.online }}</div>
          <div class="stat-note">可随时发起远程会话</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">离线</span>
            <WifiOff :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.offline }}</div>
          <div class="stat-note">当前未连接</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已禁用</span>
            <Ban :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.disabled }}</div>
          <div class="stat-note">远程访问已被阻止</div>
        </div>
      </div>

      <div class="panel">
        <div class="toolbar">
          <div class="search">
            <Search :size="14" />
            <input
              v-model="search"
              class="input"
              type="search"
              placeholder="搜索名称、ID、负责人或分组"
              aria-label="搜索设备"
            />
          </div>
          <div class="segmented" role="group" aria-label="按状态筛选">
            <button
              v-for="option in statusOptions"
              :key="option.value"
              class="seg"
              :class="{ active: statusFilter === option.value }"
              type="button"
              @click="statusFilter = option.value"
            >
              {{ option.label }}
            </button>
          </div>
          <div class="toolbar-spacer" />
          <select
            class="select"
            style="width: 138px; height: 34px"
            :value="groupFilter"
            aria-label="按分组筛选"
            @change="setGroupFilter($event.target.value)"
          >
            <option value="">全部分组</option>
            <option v-for="name in groupOptions" :key="name" :value="name">
              {{ name }}
            </option>
            <option v-if="hasUngrouped" value="__ungrouped__">
              未分组
            </option>
          </select>
          <select
            v-model="platformFilter"
            class="select"
            style="width: 128px; height: 34px"
            aria-label="按平台筛选"
          >
            <option value="all">全部平台</option>
            <option
              v-for="value in platformOptions"
              :key="value"
              :value="value"
            >
              {{ osLabel(value) }}
            </option>
          </select>
        </div>

        <div v-if="selected.size" class="bulk-bar">
          <strong>已选择 {{ selected.size }} 台设备</strong>
          <span class="faint">对当前选择应用操作</span>
          <div class="bulk-actions">
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              :disabled="bulkBusy"
              @click="setSelectedDisabled(false)"
            >
              <Power :size="13" />
              启用
            </button>
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              :disabled="bulkBusy"
              @click="setSelectedDisabled(true)"
            >
              <PowerOff :size="13" />
              禁用
            </button>
            <button
              class="btn btn-danger-ghost btn-sm"
              type="button"
              :disabled="bulkBusy"
              @click="deleteOpen = true"
            >
              <Trash2 :size="13" />
              删除
            </button>
          </div>
        </div>

        <div v-if="loading" class="loading-state">
          <span class="spinner-dark" />
          正在加载设备
        </div>

        <div v-else-if="error && devices.length === 0" class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法连接服务器</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>

        <template v-else>
          <div class="table-wrap">
            <table class="table">
              <thead>
                <tr>
                  <th class="cell-check">
                    <input
                      type="checkbox"
                      :checked="allPageSelected"
                      aria-label="选择本页全部设备"
                      @change="toggleSelectPage"
                    />
                  </th>
                  <th>设备</th>
                  <th>负责人</th>
                  <th>分组</th>
                  <th>平台</th>
                  <th>最后在线</th>
                  <th>状态</th>
                  <th class="cell-actions" />
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="device in paged"
                  :key="deviceGuid(device)"
                  :class="{
                    selected: selected.has(deviceGuid(device)),
                  }"
                  @click="openDevice(device)"
                >
                  <td class="cell-check" @click.stop>
                    <input
                      type="checkbox"
                      :checked="selected.has(deviceGuid(device))"
                      :aria-label="`选择 ${deviceName(device)}`"
                      @change="toggleSelect(deviceGuid(device))"
                    />
                  </td>
                  <td>
                    <div class="device-cell">
                      <div class="device-icon">
                        <OsIcon :os="deviceOs(device)" :size="15" />
                      </div>
                      <div class="device-meta">
                        <div class="device-name">{{ deviceName(device) }}</div>
                        <div class="device-id">{{ formatId(device.id) }}</div>
                      </div>
                    </div>
                  </td>
                  <td>
                    <div style="color: var(--ink)">
                      {{ device.user_name || '—' }}
                    </div>
                    <div
                      v-if="device.info?.username"
                      class="mono faint"
                      style="font-size: 10px"
                    >
                      @{{ device.info.username }}
                    </div>
                  </td>
                  <td>
                    <span v-if="device.device_group_name" class="tag">
                      {{ device.device_group_name }}
                    </span>
                    <span v-else class="faint">—</span>
                  </td>
                  <td>
                    <div>{{ osLabel(deviceOs(device)) }}</div>
                    <div
                      v-if="device.info?.version"
                      class="mono faint"
                      style="font-size: 10px"
                    >
                      v{{ device.info.version }}
                    </div>
                  </td>
                  <td>
                    <span :title="exactTime(device.last_online)">
                      {{ timeAgo(device.last_online) }}
                    </span>
                  </td>
                  <td><StatusPill :state="deviceState(device)" /></td>
                  <td class="cell-actions" @click.stop>
                    <div class="row-actions">
                      <button
                        class="icon-btn row-action"
                        type="button"
                        :title="
                          deviceState(device) === 'online'
                            ? '远程控制'
                            : '设备离线，无法发起远程控制'
                        "
                        :disabled="deviceState(device) !== 'online'"
                        @click="openControl(device)"
                      >
                        <MonitorPlay :size="15" />
                      </button>
                      <button
                        class="icon-btn row-action"
                        type="button"
                        title="查看详情"
                        @click="openDevice(device)"
                      >
                        <MoreHorizontal :size="16" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="device-cards">
            <div
              v-for="device in paged"
              :key="deviceGuid(device)"
              class="device-card"
              @click="openDevice(device)"
            >
              <div class="device-card-top">
                <div class="device-icon">
                  <OsIcon :os="deviceOs(device)" :size="16" />
                </div>
                <div class="device-meta">
                  <div class="device-name">{{ deviceName(device) }}</div>
                  <div class="device-id">{{ formatId(device.id) }}</div>
                </div>
                <StatusPill :state="deviceState(device)" />
              </div>
              <div class="device-card-meta">
                <span v-if="device.device_group_name" class="tag">
                  {{ device.device_group_name }}
                </span>
                <span class="tag">{{ osLabel(deviceOs(device)) }}</span>
                <span class="faint">{{ timeAgo(device.last_online) }}</span>
              </div>
              <button
                class="btn btn-secondary btn-sm card-control"
                type="button"
                :disabled="deviceState(device) !== 'online'"
                @click.stop="openControl(device)"
              >
                <MonitorPlay :size="13" />
                远程控制
              </button>
            </div>
          </div>

          <div v-if="!paged.length" class="empty">
            <div class="empty-icon"><Search :size="18" /></div>
            <h3>
              {{ hasFilters ? '没有符合条件的设备' : '还没有设备' }}
            </h3>
            <p v-if="hasFilters">
              换一个搜索词、分组或状态试试，筛选会作用于全部设备。
            </p>
            <p v-else>
              接入这台 RustDesk 服务器的设备会显示在这里。
            </p>
            <button
              v-if="hasFilters"
              class="btn btn-secondary"
              type="button"
              @click="clearFilters"
            >
              清除筛选
            </button>
            <button v-else class="btn btn-secondary" type="button" @click="load()">
              刷新
            </button>
          </div>

          <div v-if="paged.length" class="table-footer">
            <span>
              显示 {{ rangeStart }}–{{ rangeEnd }}，共 {{ filtered.length }} 台
            </span>
            <div class="pager">
              <button
                class="icon-btn"
                type="button"
                aria-label="上一页"
                :disabled="page === 1"
                @click="setPage(page - 1)"
              >
                <ChevronLeft :size="16" />
              </button>
              <span class="mono" style="padding: 0 4px">
                {{ page }} / {{ pageCount }}
              </span>
              <button
                class="icon-btn"
                type="button"
                aria-label="下一页"
                :disabled="page === pageCount"
                @click="setPage(page + 1)"
              >
                <ChevronRight :size="16" />
              </button>
            </div>
          </div>
        </template>
      </div>
    </main>

    <DeviceDrawer
      v-if="drawerDevice"
      :device="drawerDevice"
      :groups="groups"
      :users="users"
      @close="drawerDevice = null"
      @updated="handleDrawerUpdated"
      @deleted="handleDrawerDeleted"
    />

    <ControlDialog
      v-if="controlTarget"
      :device="controlTarget"
      @close="controlTarget = null"
    />

    <ConfirmDialog
      v-if="deleteOpen"
      title="删除所选设备？"
      :message="`将从 RustDesk 服务器移除 ${selected.size} 台设备。此操作无法撤销。`"
      confirm-label="删除设备"
      danger
      :busy="bulkBusy"
      @cancel="deleteOpen = false"
      @confirm="deleteSelected"
    />
  </AppShell>
</template>
