<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  ArrowUpRight,
  FolderPlus,
  Pencil,
  RefreshCw,
  Trash2,
  Users,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import GroupModal from '../components/GroupModal.vue'
import { getApi } from '../lib/api'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const groups = ref([])
const devices = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const modalOpen = ref(false)
const editingGroup = ref(null)
const modalBusy = ref(false)
const modalError = ref('')
const deleteTarget = ref(null)
const deleteBusy = ref(false)

const rows = computed(() => {
  const counts = new Map()
  for (const device of devices.value) {
    if (!device.device_group_name) continue
    counts.set(
      device.device_group_name,
      (counts.get(device.device_group_name) || 0) + 1
    )
  }
  return groups.value
    .map((group) => ({ ...group, count: counts.get(group.name) || 0 }))
    .sort((a, b) => a.name.localeCompare(b.name))
})

const stats = computed(() => {
  const grouped = devices.value.filter(
    (device) => Boolean(device.device_group_name)
  ).length
  return {
    groups: groups.value.length,
    grouped,
    ungrouped: devices.value.length - grouped,
  }
})

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [groupRows, deviceRows] = await Promise.all([
      api.getGroups(),
      api.getDevices(),
    ])
    groups.value = groupRows
    devices.value = deviceRows
  } catch (e) {
    error.value = e.message || '分组列表加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function openCreate() {
  editingGroup.value = null
  modalError.value = ''
  modalOpen.value = true
}

function openRename(group) {
  editingGroup.value = group
  modalError.value = ''
  modalOpen.value = true
}

function closeModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

async function submitGroup(payload) {
  modalBusy.value = true
  modalError.value = ''
  try {
    if (editingGroup.value) {
      await api.updateGroup(editingGroup.value.guid, payload)
      notify('分组已更新。')
    } else {
      await api.createGroup(payload)
      notify('分组已创建。')
    }
    modalOpen.value = false
    await load(false)
  } catch (e) {
    modalError.value = e.message || '分组保存失败。'
  } finally {
    modalBusy.value = false
  }
}

async function deleteGroup() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteGroup(deleteTarget.value.guid)
    notify('分组已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '分组删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

function viewDevices(group) {
  router.push({ path: '/devices', query: { group: group.name } })
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">组织</div>
          <h1 class="page-title">分组</h1>
          <p class="page-sub">
            将设备分组，便于在设备规模扩大后清晰管理访问策略与归属。
          </p>
        </div>
        <div class="page-actions">
          <button
            class="btn btn-secondary"
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
          <button class="btn btn-primary" type="button" @click="openCreate">
            <FolderPlus :size="15" />
            新建分组
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">分组数</span>
            <Users :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.groups }}</div>
          <div class="stat-note">当前服务器已定义</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已分组设备</span>
            <Users :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.grouped }}</div>
          <div class="stat-note">已分配分组的设备</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">未分组</span>
            <WifiOff :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.ungrouped }}</div>
          <div class="stat-note">仍需整理设备归属</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载分组
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法连接服务器</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else-if="!rows.length" class="panel">
        <div class="empty">
          <div class="empty-icon"><Users :size="18" /></div>
          <h3>暂无分组</h3>
          <p>
            创建第一个分组，按团队、地点或用途整理设备。
          </p>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <FolderPlus :size="15" />
            新建分组
          </button>
        </div>
      </div>

      <div v-else class="group-grid">
        <article v-for="group in rows" :key="group.guid" class="group-card">
          <div class="group-card-head">
            <div class="group-mark"><Users :size="15" /></div>
            <div style="min-width: 0; flex: 1">
              <div class="group-name">{{ group.name }}</div>
              <div class="group-count">{{ group.count }} 台设备</div>
            </div>
          </div>
          <p class="group-note">
            {{ group.note || '这个分组还没有备注。' }}
          </p>
          <div class="group-actions">
            <button
              class="group-link"
              type="button"
              @click="viewDevices(group)"
            >
              查看设备
              <ArrowUpRight :size="12" />
            </button>
            <div class="spacer" />
            <button
              class="icon-btn"
              type="button"
              title="重命名分组"
              @click="openRename(group)"
            >
              <Pencil :size="14" />
            </button>
            <button
              class="icon-btn danger"
              type="button"
              title="删除分组"
              @click="deleteTarget = group"
            >
              <Trash2 :size="14" />
            </button>
          </div>
        </article>
      </div>
    </main>

    <GroupModal
      v-if="modalOpen"
      :group="editingGroup"
      :busy="modalBusy"
      :error="modalError"
      @cancel="closeModal"
      @submit="submitGroup"
    />

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除此分组？"
      :message="`${deleteTarget.name} 将被删除。其中的设备仍保持注册状态，但会变为未分组。`"
      confirm-label="删除分组"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="deleteGroup"
    />
  </AppShell>
</template>
