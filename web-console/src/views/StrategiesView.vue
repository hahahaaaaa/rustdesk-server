<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  FolderTree,
  MonitorSmartphone,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Send,
  ShieldCheck,
  ShieldOff,
  SlidersHorizontal,
  Trash2,
  Users,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import StrategyAssignModal from '../components/StrategyAssignModal.vue'
import StrategyModal from '../components/StrategyModal.vue'
import { getApi } from '../lib/api'
import { exactTime, timeAgo } from '../lib/format'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const strategies = ref([])
const devices = ref([])
const users = ref([])
const groups = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const search = ref('')
const applied = ref('')

const modalOpen = ref(false)
const editingStrategy = ref(null)
const modalBusy = ref(false)
const modalError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)
const statusBusy = ref('')

const assignOpen = ref(false)
const assignTarget = ref(null)
const assignment = ref({ devices: [], users: [], groups: [] })
const assignBusy = ref(false)
const assignError = ref('')

const rows = computed(() => {
  const term = applied.value.toLowerCase()
  return strategies.value
    .filter(
      (strategy) =>
        !term ||
        `${strategy.name} ${strategy.note}`.toLowerCase().includes(term)
    )
    .sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'))
})

const hasFilters = computed(() => Boolean(applied.value))
const stats = computed(() => ({
  total: strategies.value.length,
  enabled: strategies.value.filter((strategy) => strategy.enabled).length,
  targets: strategies.value.reduce(
    (sum, strategy) =>
      sum +
      Number(strategy.device_count || 0) +
      Number(strategy.user_count || 0) +
      Number(strategy.group_count || 0),
    0
  ),
}))

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [strategyRows, deviceRows, groupRows, userRows] = await Promise.all([
      api.getStrategies(),
      api.getDevices(),
      api.getGroups(),
      api.getUsers(),
    ])
    strategies.value = strategyRows
    devices.value = deviceRows
    groups.value = groupRows
    users.value = userRows
  } catch (e) {
    error.value = e.message || '策略加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function applySearch() {
  applied.value = search.value.trim()
}

function clearSearch() {
  search.value = ''
  applied.value = ''
}

function openCreate() {
  editingStrategy.value = null
  modalError.value = ''
  modalOpen.value = true
}

function openEdit(strategy) {
  editingStrategy.value = strategy
  modalError.value = ''
  modalOpen.value = true
}

async function submitStrategy(payload) {
  modalBusy.value = true
  modalError.value = ''
  try {
    if (editingStrategy.value) {
      await api.updateStrategy(editingStrategy.value.guid, payload)
      notify('策略已更新。')
    } else {
      await api.createStrategy(payload)
      notify('策略已创建。')
    }
    modalOpen.value = false
    await load(false)
  } catch (e) {
    modalError.value = e.message || '策略保存失败。'
  } finally {
    modalBusy.value = false
  }
}

async function toggleStatus(strategy) {
  if (statusBusy.value) return
  statusBusy.value = strategy.guid
  try {
    const data = await api.setStrategyStatus(strategy.guid, !strategy.enabled)
    notify(data.enabled ? '策略已启用。' : '策略已停用。')
    await load(false)
  } catch (e) {
    notify(e.message || '策略状态更新失败。', 'error')
  } finally {
    statusBusy.value = ''
  }
}

async function deleteStrategy() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteStrategy(deleteTarget.value.guid)
    notify('策略已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '策略删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

async function openAssign(strategy) {
  assignError.value = ''
  try {
    const data = await api.getStrategyAssignments(strategy.guid)
    assignment.value = data
    assignTarget.value = strategy
    assignOpen.value = true
  } catch (e) {
    notify(e.message || '策略分配加载失败。', 'error')
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  }
}

async function submitAssign(selected) {
  if (!assignTarget.value) return
  const current = {
    peers: (assignment.value.devices || []).map((device) => device.guid),
    users: (assignment.value.users || []).map((user) => user.name),
    groups: (assignment.value.groups || []).map((group) => group.guid),
  }
  const removed = {
    peers: current.peers.filter((guid) => !selected.devices.includes(guid)),
    users: current.users.filter((name) => !selected.users.includes(name)),
    groups: current.groups.filter((guid) => !selected.groups.includes(guid)),
  }
  const added = {
    peers: selected.devices.filter((guid) => !current.peers.includes(guid)),
    users: selected.users.filter((name) => !current.users.includes(name)),
    groups: selected.groups.filter((guid) => !current.groups.includes(guid)),
  }
  const hasRemoved =
    removed.peers.length || removed.users.length || removed.groups.length
  const hasAdded =
    added.peers.length || added.users.length || added.groups.length
  if (!hasRemoved && !hasAdded) {
    assignOpen.value = false
    return
  }

  assignBusy.value = true
  assignError.value = ''
  try {
    if (hasRemoved) {
      await api.assignStrategy({ strategy: null, ...removed })
    }
    if (hasAdded) {
      await api.assignStrategy({
        strategy: assignTarget.value.guid,
        ...added,
      })
    }
    notify('策略分配已保存。')
    assignOpen.value = false
    await load(false)
  } catch (e) {
    assignError.value = e.message || '策略分配保存失败。'
  } finally {
    assignBusy.value = false
  }
}

function closeModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

function closeAssign() {
  if (assignBusy.value) return
  assignOpen.value = false
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">客户端策略</div>
          <h1 class="page-title">策略下发</h1>
          <p class="page-sub">
            统一管理远程控制的安全选项与功能开关，通过客户端心跳实时下发。
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
            <Plus :size="15" />
            新建策略
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">策略总数</span>
            <SlidersHorizontal :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">{{ hasFilters ? '符合搜索条件' : '已创建的下发策略' }}</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已启用</span>
            <ShieldCheck :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.enabled }}</div>
          <div class="stat-note">停用的策略不会下发到客户端</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">下发对象</span>
            <Send :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.targets }}</div>
          <div class="stat-note">设备、用户与设备分组的绑定总数</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载策略
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载策略</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="toolbar">
          <div class="search">
            <Search :size="14" />
            <input
              v-model="search"
              class="input"
              type="search"
              placeholder="搜索策略名称或备注"
              aria-label="搜索策略"
              @keyup.enter="applySearch"
            />
          </div>
          <button
            class="btn btn-secondary btn-sm"
            type="button"
            @click="applySearch"
          >
            搜索
          </button>
          <button
            v-if="hasFilters"
            class="btn btn-ghost btn-sm"
            type="button"
            @click="clearSearch"
          >
            重置
          </button>
          <div class="toolbar-spacer" />
          <span class="faint" style="font-size: 11.5px">
            共 {{ rows.length }} 个策略
          </span>
        </div>

        <div v-if="!rows.length" class="empty">
          <div class="empty-icon"><ShieldOff :size="18" /></div>
          <h3>{{ hasFilters ? '没有匹配的策略' : '暂无策略' }}</h3>
          <p>
            {{
              hasFilters
                ? '尝试调整搜索关键词。'
                : '创建策略并分配给设备、用户或分组，统一远程控制行为。'
            }}
          </p>
          <button
            v-if="hasFilters"
            class="btn btn-secondary"
            type="button"
            @click="clearSearch"
          >
            清除搜索
          </button>
          <button v-else class="btn btn-primary" type="button" @click="openCreate">
            <Plus :size="15" />
            新建策略
          </button>
        </div>

        <div v-else>
          <div class="table-wrap">
            <table class="table strategy-table">
              <thead>
                <tr>
                  <th>策略</th>
                  <th style="width: 126px">状态</th>
                  <th style="width: 186px">下发对象</th>
                  <th style="width: 150px">更新时间</th>
                  <th style="width: 172px">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="strategy in rows" :key="strategy.guid">
                  <td>
                    <div class="strategy-cell">
                      <span
                        class="strategy-mark"
                        :class="{ off: !strategy.enabled }"
                      >
                        <ShieldCheck v-if="strategy.enabled" :size="15" />
                        <ShieldOff v-else :size="15" />
                      </span>
                      <div class="strategy-copy">
                        <div class="strategy-name">{{ strategy.name }}</div>
                        <div class="strategy-note" :title="strategy.note">
                          {{ strategy.note || '暂无备注' }}
                        </div>
                      </div>
                    </div>
                  </td>
                  <td>
                    <div class="status-cell">
                      <button
                        class="switch"
                        :class="{ on: strategy.enabled }"
                        type="button"
                        role="switch"
                        :aria-checked="strategy.enabled"
                        :aria-label="`${strategy.enabled ? '停用' : '启用'}策略 ${strategy.name}`"
                        :disabled="statusBusy === strategy.guid"
                        @click="toggleStatus(strategy)"
                      />
                      <span class="status-text">
                        {{ strategy.enabled ? '已启用' : '已停用' }}
                      </span>
                    </div>
                  </td>
                  <td>
                    <div class="target-pills">
                      <span class="count-pill" title="设备">
                        <MonitorSmartphone :size="11" />
                        {{ strategy.device_count || 0 }}
                      </span>
                      <span class="count-pill" title="用户">
                        <Users :size="11" />
                        {{ strategy.user_count || 0 }}
                      </span>
                      <span class="count-pill" title="设备分组">
                        <FolderTree :size="11" />
                        {{ strategy.group_count || 0 }}
                      </span>
                    </div>
                  </td>
                  <td>
                    <div class="time-cell">{{ timeAgo(strategy.updated_at) }}</div>
                    <div class="time-full">
                      {{ exactTime(strategy.updated_at) }}
                    </div>
                  </td>
                  <td>
                    <div class="row-actions">
                      <button
                        class="row-link"
                        type="button"
                        @click="openAssign(strategy)"
                      >
                        <Send :size="13" />
                        分配
                      </button>
                      <button
                        class="icon-btn"
                        type="button"
                        title="编辑策略"
                        @click="openEdit(strategy)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        class="icon-btn danger"
                        type="button"
                        title="删除策略"
                        @click="deleteTarget = strategy"
                      >
                        <Trash2 :size="14" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="strategy-cards">
            <article
              v-for="strategy in rows"
              :key="strategy.guid"
              class="strategy-card"
            >
              <div class="strategy-card-head">
                <span
                  class="strategy-mark"
                  :class="{ off: !strategy.enabled }"
                >
                  <ShieldCheck v-if="strategy.enabled" :size="15" />
                  <ShieldOff v-else :size="15" />
                </span>
                <div class="strategy-copy">
                  <div class="strategy-name">{{ strategy.name }}</div>
                  <div class="strategy-note" :title="strategy.note">
                    {{ strategy.note || '暂无备注' }}
                  </div>
                </div>
                <button
                  class="switch"
                  :class="{ on: strategy.enabled }"
                  type="button"
                  role="switch"
                  :aria-checked="strategy.enabled"
                  :aria-label="`${strategy.enabled ? '停用' : '启用'}策略 ${strategy.name}`"
                  :disabled="statusBusy === strategy.guid"
                  @click="toggleStatus(strategy)"
                />
              </div>
              <div class="strategy-card-meta">
                <span class="count-pill" title="设备">
                  <MonitorSmartphone :size="11" />
                  {{ strategy.device_count || 0 }}
                </span>
                <span class="count-pill" title="用户">
                  <Users :size="11" />
                  {{ strategy.user_count || 0 }}
                </span>
                <span class="count-pill" title="设备分组">
                  <FolderTree :size="11" />
                  {{ strategy.group_count || 0 }}
                </span>
                <span class="strategy-card-time">
                  {{ timeAgo(strategy.updated_at) }}
                </span>
              </div>
              <div class="strategy-card-actions">
                <button
                  class="btn btn-secondary btn-sm"
                  type="button"
                  @click="openAssign(strategy)"
                >
                  <Send :size="13" />
                  分配对象
                </button>
                <button
                  class="icon-btn"
                  type="button"
                  title="编辑策略"
                  @click="openEdit(strategy)"
                >
                  <Pencil :size="14" />
                </button>
                <button
                  class="icon-btn danger"
                  type="button"
                  title="删除策略"
                  @click="deleteTarget = strategy"
                >
                  <Trash2 :size="14" />
                </button>
              </div>
            </article>
          </div>
        </div>
      </div>
    </main>

    <StrategyModal
      v-if="modalOpen"
      :strategy="editingStrategy"
      :busy="modalBusy"
      :error="modalError"
      @cancel="closeModal"
      @submit="submitStrategy"
    />

    <StrategyAssignModal
      v-if="assignOpen"
      :strategy="assignTarget"
      :devices="devices"
      :users="users"
      :groups="groups"
      :assignments="assignment"
      :busy="assignBusy"
      :error="assignError"
      @cancel="closeAssign"
      @submit="submitAssign"
    />

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除此策略？"
      :message="`${deleteTarget.name} 将被永久删除，所有设备、用户与分组的绑定会同时解除。`"
      confirm-label="删除策略"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="deleteStrategy"
    />
  </AppShell>
</template>

<style scoped>
.strategy-table {
  min-width: 860px;
}

.strategy-table tbody tr {
  cursor: default;
}

.strategy-cell {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
}

.strategy-mark {
  display: grid;
  place-items: center;
  flex: none;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--ink);
}

.strategy-mark.off {
  color: var(--faint);
}

.strategy-copy {
  min-width: 0;
}

.strategy-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.strategy-note {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
  font-size: 11px;
  color: var(--muted);
}

.status-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.switch {
  position: relative;
  flex: none;
  width: 32px;
  height: 18px;
  padding: 0;
  border: 0;
  border-radius: 9999px;
  background: var(--line-strong);
  transition: background 0.15s ease;
}

.switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.22);
  transition: transform 0.15s ease;
}

.switch.on {
  background: var(--ink);
}

.switch.on::after {
  transform: translateX(14px);
}

.switch:disabled {
  opacity: 0.5;
  cursor: wait;
}

.status-text {
  font-size: 11.5px;
  color: var(--muted);
  white-space: nowrap;
}

.target-pills {
  display: flex;
  align-items: center;
  gap: 4px;
}

.count-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 22px;
  padding: 0 7px;
  border-radius: 9999px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  font-size: 11px;
  color: var(--muted);
}

.time-cell {
  font-size: 12px;
  color: var(--ink);
  white-space: nowrap;
}

.time-full {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.row-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 500;
}

.row-link:hover {
  background: var(--wash);
  color: var(--ink);
}

.strategy-cards {
  display: none;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}

.strategy-card {
  padding: 14px;
  border-radius: var(--radius);
  background: #fff;
  box-shadow: inset 0 0 0 1px var(--line);
}

.strategy-card-head {
  display: flex;
  align-items: center;
  gap: 9px;
}

.strategy-card-head .strategy-copy {
  flex: 1;
}

.strategy-card-meta {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 12px;
}

.strategy-card-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--faint);
}

.strategy-card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

.strategy-card-actions .btn {
  margin-right: auto;
}

@media (max-width: 960px) {
  .strategy-cards {
    display: flex;
  }
}
</style>
