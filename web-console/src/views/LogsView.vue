<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  ChevronLeft,
  ChevronRight,
  FileDown,
  MonitorSmartphone,
  Pencil,
  RefreshCw,
  Search,
  ShieldAlert,
  Trash2,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime, timeAgo } from '../lib/format'
import { clearSession, can } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const TABS = [
  {
    key: 'conn',
    label: '连接日志',
    hint: '远程会话、认证与来源 IP',
    icon: MonitorSmartphone,
  },
  {
    key: 'file',
    label: '文件传输',
    hint: '文件与目录传输记录',
    icon: FileDown,
  },
  {
    key: 'alarm',
    label: '安全告警',
    hint: '登录风控与白名单拦截',
    icon: ShieldAlert,
  },
]

const CONN_TYPES = {
  0: '远程桌面',
  1: '文件传输',
  2: '端口转发',
  3: 'RDP',
  4: '摄像头',
  5: '终端',
  6: '其他',
}

const ALARM_TYPES = {
  0: 'IP 白名单拦截',
  1: '登录尝试超过 30 次',
  2: '一分钟内尝试 6 次登录',
  6: 'IPv6 前缀登录尝试异常',
  7: '终端系统登录退避',
  8: '终端系统登录并发超限',
  9: '会话范围越权',
  10: '设备 ID 白名单拦截',
}

const tab = ref('conn')
const rows = ref([])
const total = ref(0)
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const page = ref(1)
const pageSize = 25

const search = ref('')
const device = ref('')
const remote = ref('')
const type = ref('')
const applied = ref({ q: '', device: '', remote: '', type: '' })

const noteTarget = ref(null)
const noteText = ref('')
const noteBusy = ref(false)
const noteError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)

const activeTab = computed(() => TABS.find((item) => item.key === tab.value))
const canManage = computed(() => can('audit.manage'))
const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize)))
const rangeStart = computed(() =>
  total.value ? (page.value - 1) * pageSize + 1 : 0
)
const rangeEnd = computed(() => Math.min(total.value, page.value * pageSize))
const hasFilters = computed(() =>
  Boolean(
    applied.value.q ||
      applied.value.device ||
      applied.value.remote ||
      applied.value.type
  )
)

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    let data
    if (tab.value === 'conn') {
      data = await api.getConnLogs({
        current: page.value,
        pageSize,
        q: applied.value.q,
        device: applied.value.device,
        remote: applied.value.remote,
        connType: applied.value.type,
      })
    } else if (tab.value === 'file') {
      data = await api.getFileLogs({
        current: page.value,
        pageSize,
        q: applied.value.q,
        device: applied.value.device,
        remote: applied.value.remote,
      })
    } else {
      data = await api.getAlarmLogs({
        current: page.value,
        pageSize,
        q: applied.value.q,
        device: applied.value.device,
        alarmType: applied.value.type,
      })
    }
    rows.value = Array.isArray(data?.data) ? data.data : []
    total.value = Number(data?.total || 0)
  } catch (e) {
    error.value = e.message || '日志加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function switchTab(next) {
  if (tab.value === next) return
  tab.value = next
  rows.value = []
  total.value = 0
  page.value = 1
  resetFilters()
}

function applyFilters() {
  applied.value = {
    q: search.value.trim(),
    device: device.value.trim(),
    remote: remote.value.trim(),
    type: type.value,
  }
  page.value = 1
  load(false)
}

function resetFilters() {
  search.value = ''
  device.value = ''
  remote.value = ''
  type.value = ''
  applied.value = { q: '', device: '', remote: '', type: '' }
  page.value = 1
  load(false)
}

function setPage(next) {
  const target = Math.min(Math.max(1, next), pageCount.value)
  if (target === page.value) return
  page.value = target
  load(false)
}

function connTypeLabel(value) {
  return CONN_TYPES[value] ?? `类型 ${value}`
}

function alarmTypeLabel(value) {
  return ALARM_TYPES[value] ?? `告警 ${value}`
}

function fileTypeLabel(row) {
  if (row.file_type === 1) return '接收'
  if (row.file_type === 0) return '发送'
  return `类型 ${row.file_type}`
}

function peerText(row) {
  const name = String(row.peer_name || '').trim()
  const id = String(row.peer_id || '').trim()
  if (name && id) return `${name} · ${id}`
  return name || id || '—'
}

function duration(row) {
  if (!row.end_time) return '进行中'
  const seconds = Math.max(0, Number(row.end_time) - Number(row.created_at_ts))
  if (seconds < 60) return `${seconds} 秒`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟`
  const hours = Math.floor(minutes / 60)
  return `${hours} 小时 ${minutes % 60} 分钟`
}

function jsonText(value, raw) {
  if (value == null) return raw || ''
  if (typeof value === 'string') return value
  if (typeof value === 'object') {
    const preferred =
      value.message || value.info || value.ip || value.path || value.name
    if (preferred) return String(preferred)
    try {
      return JSON.stringify(value)
    } catch {
      return raw || ''
    }
  }
  return String(value)
}

function openNote(row) {
  noteTarget.value = row
  noteText.value = row.note || ''
  noteError.value = ''
}

function closeNote() {
  if (noteBusy.value) return
  noteTarget.value = null
}

async function saveNote() {
  if (!noteTarget.value) return
  noteBusy.value = true
  noteError.value = ''
  try {
    await api.updateLogNote(tab.value, noteTarget.value.id, noteText.value.trim())
    notify('备注已保存。')
    noteTarget.value = null
    await load(false)
  } catch (e) {
    noteError.value = e.message || '备注保存失败。'
  } finally {
    noteBusy.value = false
  }
}

async function removeLog() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteLog(tab.value, deleteTarget.value.id)
    notify('日志已删除。')
    deleteTarget.value = null
    if (rows.value.length === 1 && page.value > 1) page.value -= 1
    await load(false)
  } catch (e) {
    notify(e.message || '日志删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">安全与合规</div>
          <h1 class="page-title">连接与文件日志</h1>
          <p class="page-sub">
            查看客户端上报的远程会话、文件传输与安全告警，并可为记录添加备注。
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
        </div>
      </div>

      <div class="log-tabs" role="tablist" aria-label="日志类型">
        <button
          v-for="item in TABS"
          :key="item.key"
          class="log-tab"
          :class="{ active: tab === item.key }"
          type="button"
          role="tab"
          :aria-selected="tab === item.key"
          @click="switchTab(item.key)"
        >
          <component :is="item.icon" :size="16" />
          <span class="log-tab-text">
            <strong>{{ item.label }}</strong>
            <small>{{ item.hint }}</small>
          </span>
        </button>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载日志
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载日志</h3>
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
              placeholder="搜索设备、来源或内容"
              aria-label="搜索日志"
              @keyup.enter="applyFilters"
            />
          </div>
          <input
            v-model="device"
            class="input log-device"
            type="search"
            placeholder="设备 ID"
            aria-label="按设备筛选"
            @keyup.enter="applyFilters"
          />
          <input
            v-if="tab !== 'alarm'"
            v-model="remote"
            class="input log-remote"
            type="search"
            placeholder="对端 ID"
            aria-label="按对端筛选"
            @keyup.enter="applyFilters"
          />
          <select
            v-if="tab !== 'file'"
            v-model="type"
            class="select log-type"
            aria-label="按类型筛选"
            @change="applyFilters"
          >
            <option value="">
              {{ tab === 'conn' ? '全部连接类型' : '全部告警类型' }}
            </option>
            <template v-if="tab === 'conn'">
              <option
                v-for="(label, value) in CONN_TYPES"
                :key="value"
                :value="value"
              >
                {{ label }}
              </option>
            </template>
            <template v-else>
              <option
                v-for="(label, value) in ALARM_TYPES"
                :key="value"
                :value="value"
              >
                {{ label }}
              </option>
            </template>
          </select>
          <button
            class="btn btn-secondary btn-sm"
            type="button"
            @click="applyFilters"
          >
            筛选
          </button>
          <button
            v-if="hasFilters"
            class="btn btn-ghost btn-sm"
            type="button"
            @click="resetFilters"
          >
            重置
          </button>
          <div class="toolbar-spacer" />
          <span class="faint" style="font-size: 11.5px">
            共 {{ total }} 条记录
          </span>
        </div>

        <div v-if="!rows.length" class="empty">
          <div class="empty-icon">
            <component :is="activeTab.icon" :size="18" />
          </div>
          <h3>{{ hasFilters ? '没有匹配的记录' : '暂无日志' }}</h3>
          <p>
            {{
              hasFilters
                ? '尝试调整设备、类型或搜索关键词。'
                : 'RustDesk 客户端上报事件后，记录会显示在这里。'
            }}
          </p>
          <button
            v-if="hasFilters"
            class="btn btn-secondary"
            type="button"
            @click="resetFilters"
          >
            清除筛选
          </button>
        </div>

        <template v-else>
          <div class="table-wrap">
            <table v-if="tab === 'conn'" class="table logs-table">
              <thead>
                <tr>
                  <th style="width: 158px">时间</th>
                  <th>被控设备</th>
                  <th>操作方</th>
                  <th style="width: 110px">类型</th>
                  <th style="width: 150px">来源 IP</th>
                  <th style="width: 150px">会话状态</th>
                  <th style="width: 120px">备注</th>
                  <th style="width: 72px">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.id">
                  <td>
                    <div class="log-time">{{ exactTime(row.created_at) }}</div>
                    <div class="log-ago">{{ timeAgo(row.created_at) }}</div>
                  </td>
                  <td>
                    <div class="log-main mono">{{ row.device_id || '—' }}</div>
                    <div class="log-sub">{{ row.action === 'close' ? '关闭事件' : '连接事件' }}</div>
                  </td>
                  <td>
                    <div class="log-main">{{ peerText(row) }}</div>
                    <div class="log-sub">
                      {{
                        row.two_factor
                          ? '双因素认证'
                          : row.primary_auth
                            ? '密码认证'
                            : '无认证信息'
                      }}
                    </div>
                  </td>
                  <td>
                    <span class="tag tag-blue">{{ connTypeLabel(row.conn_type) }}</span>
                  </td>
                  <td class="mono log-sub">{{ row.ip || '—' }}</td>
                  <td>
                    <span
                      class="tag"
                      :class="row.end_time ? 'tag-muted' : 'tag-success'"
                    >
                      {{ duration(row) }}
                    </span>
                    <div v-if="row.closed_at" class="log-sub">
                      结束 {{ exactTime(row.closed_at) }}
                    </div>
                  </td>
                  <td>
                    <span class="log-note" :title="row.note">
                      {{ row.note || '—' }}
                    </span>
                  </td>
                  <td>
                    <div v-if="canManage" class="row-actions">
                      <button
                        class="icon-btn"
                        type="button"
                        title="编辑备注"
                        @click="openNote(row)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        class="icon-btn danger"
                        type="button"
                        title="删除日志"
                        @click="deleteTarget = row"
                      >
                        <Trash2 :size="14" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>

            <table v-else-if="tab === 'file'" class="table logs-table">
              <thead>
                <tr>
                  <th style="width: 158px">时间</th>
                  <th>设备</th>
                  <th>对端</th>
                  <th style="width: 90px">方向</th>
                  <th>文件或目录</th>
                  <th style="width: 120px">备注</th>
                  <th style="width: 72px">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.id">
                  <td>
                    <div class="log-time">{{ exactTime(row.created_at) }}</div>
                    <div class="log-ago">{{ timeAgo(row.created_at) }}</div>
                  </td>
                  <td>
                    <div class="log-main mono">{{ row.device_id || '—' }}</div>
                    <div class="log-sub">{{ row.device_uuid || '—' }}</div>
                  </td>
                  <td class="mono log-main">{{ row.peer_id || '—' }}</td>
                  <td>
                    <span class="tag tag-blue">{{ fileTypeLabel(row) }}</span>
                  </td>
                  <td>
                    <div class="log-path" :title="row.path">
                      {{ row.path || '—' }}
                    </div>
                    <div class="log-sub">
                      {{ row.is_file ? '文件' : '目录' }}
                      <template v-if="row.info_raw">
                        · {{ jsonText(row.info, row.info_raw) }}
                      </template>
                    </div>
                  </td>
                  <td>
                    <span class="log-note" :title="row.note">
                      {{ row.note || '—' }}
                    </span>
                  </td>
                  <td>
                    <div v-if="canManage" class="row-actions">
                      <button
                        class="icon-btn"
                        type="button"
                        title="编辑备注"
                        @click="openNote(row)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        class="icon-btn danger"
                        type="button"
                        title="删除日志"
                        @click="deleteTarget = row"
                      >
                        <Trash2 :size="14" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>

            <table v-else class="table logs-table">
              <thead>
                <tr>
                  <th style="width: 158px">时间</th>
                  <th>设备</th>
                  <th style="width: 190px">告警类型</th>
                  <th>详情</th>
                  <th>关联连接</th>
                  <th style="width: 120px">备注</th>
                  <th style="width: 72px">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.id">
                  <td>
                    <div class="log-time">{{ exactTime(row.created_at) }}</div>
                    <div class="log-ago">{{ timeAgo(row.created_at) }}</div>
                  </td>
                  <td>
                    <div class="log-main mono">{{ row.device_id || '—' }}</div>
                    <div class="log-sub">{{ row.device_uuid || '—' }}</div>
                  </td>
                  <td>
                    <span class="tag tag-danger">
                      {{ row.type_label || alarmTypeLabel(row.alarm_type) }}
                    </span>
                  </td>
                  <td>
                    <span class="log-path" :title="row.info_raw">
                      {{ jsonText(row.info, row.info_raw) || '—' }}
                    </span>
                  </td>
                  <td class="mono log-sub">
                    {{
                      row.conn_audit_ref ||
                      (row.conn_id != null ? `#${row.conn_id}` : '—')
                    }}
                  </td>
                  <td>
                    <span class="log-note" :title="row.note">
                      {{ row.note || '—' }}
                    </span>
                  </td>
                  <td>
                    <div v-if="canManage" class="row-actions">
                      <button
                        class="icon-btn"
                        type="button"
                        title="编辑备注"
                        @click="openNote(row)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        class="icon-btn danger"
                        type="button"
                        title="删除日志"
                        @click="deleteTarget = row"
                      >
                        <Trash2 :size="14" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="log-cards">
            <article
              v-for="row in rows"
              :key="`card-${row.id}`"
              class="log-card"
            >
              <div class="log-card-head">
                <span
                  v-if="tab === 'conn'"
                  class="tag tag-blue"
                >
                  {{ connTypeLabel(row.conn_type) }}
                </span>
                <span
                  v-else-if="tab === 'file'"
                  class="tag tag-blue"
                >
                  {{ fileTypeLabel(row) }}
                </span>
                <span v-else class="tag tag-danger">
                  {{ row.type_label || alarmTypeLabel(row.alarm_type) }}
                </span>
                <span class="log-card-ago">{{ timeAgo(row.created_at) }}</span>
              </div>

              <template v-if="tab === 'conn'">
                <div class="log-main mono">{{ row.device_id || '—' }}</div>
                <div class="log-sub">
                  {{ peerText(row) }} · {{ duration(row) }}
                </div>
                <div class="log-sub mono">{{ row.ip || '—' }}</div>
              </template>
              <template v-else-if="tab === 'file'">
                <div class="log-main mono">{{ row.device_id || '—' }}</div>
                <div class="log-sub">对端 {{ row.peer_id || '—' }}</div>
                <div class="log-path" :title="row.path">
                  {{ row.path || '—' }}
                </div>
              </template>
              <template v-else>
                <div class="log-main mono">{{ row.device_id || '—' }}</div>
                <div class="log-path" :title="row.info_raw">
                  {{ jsonText(row.info, row.info_raw) || '—' }}
                </div>
              </template>

              <div v-if="row.note" class="log-card-note">{{ row.note }}</div>
              <div class="log-card-time">{{ exactTime(row.created_at) }}</div>

              <div v-if="canManage" class="log-card-actions">
                <button
                  class="btn btn-ghost btn-sm"
                  type="button"
                  @click="openNote(row)"
                >
                  <Pencil :size="13" />
                  备注
                </button>
                <button
                  class="btn btn-ghost btn-sm log-card-delete"
                  type="button"
                  @click="deleteTarget = row"
                >
                  <Trash2 :size="13" />
                  删除
                </button>
              </div>
            </article>
          </div>

          <div class="table-footer">
            <span>显示 {{ rangeStart }}–{{ rangeEnd }}，共 {{ total }} 条</span>
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

    <div v-if="noteTarget" class="modal-scrim" @click.self="closeNote">
      <form class="modal" role="dialog" aria-modal="true" @submit.prevent="saveNote">
        <h2 class="modal-title">日志备注</h2>
        <p class="modal-sub">
          {{ noteTarget.device_id || '该记录' }} · {{ exactTime(noteTarget.created_at) }}
        </p>
        <div class="modal-body">
          <div class="field">
            <label class="label" for="log-note">备注内容</label>
            <textarea
              id="log-note"
              v-model="noteText"
              class="textarea"
              maxlength="240"
              placeholder="例如：已确认为运维操作"
              autofocus
            />
          </div>
          <p v-if="noteError" class="field-error">{{ noteError }}</p>
        </div>
        <div class="modal-actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="noteBusy"
            @click="closeNote"
          >
            取消
          </button>
          <button class="btn btn-primary" type="submit" :disabled="noteBusy">
            <span v-if="noteBusy" class="spinner" />
            {{ noteBusy ? '保存中…' : '保存备注' }}
          </button>
        </div>
      </form>
    </div>

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除这条日志？"
      :message="`${deleteTarget.device_id || '该记录'} 的日志将被永久删除，客户端不会重新上报。`"
      confirm-label="删除日志"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="removeLog"
    />
  </AppShell>
</template>

<style scoped>
.log-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-bottom: 16px;
}

.log-tab {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 13px 15px;
  border: 1px solid var(--line);
  border-radius: var(--radius-md);
  background: var(--canvas);
  color: var(--muted);
  text-align: left;
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}

.log-tab:hover {
  background: var(--wash);
  color: var(--ink);
}

.log-tab.active {
  border-color: var(--ink);
  background: var(--wash);
  color: var(--ink);
}

.log-tab-text {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.log-tab-text strong {
  font-size: 12.5px;
  font-weight: 600;
}

.log-tab-text small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10.5px;
  color: var(--faint);
}

.logs-table {
  min-width: 980px;
}

.logs-table tbody tr {
  cursor: default;
}

.log-time {
  font-size: 12.5px;
  color: var(--ink);
  white-space: nowrap;
}

.log-ago {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
}

.log-main {
  font-size: 12.5px;
  color: var(--ink);
}

.log-sub {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--faint);
}

.log-path {
  display: block;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
}

.log-note {
  display: block;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11.5px;
  color: var(--muted);
}

.tag-muted {
  background: var(--wash);
  box-shadow: none;
  color: var(--muted);
}

.tag-danger {
  background: var(--red-wash);
  box-shadow: none;
  color: var(--red);
}

.tag-success {
  background: var(--green-wash);
  box-shadow: none;
  color: var(--green);
}

.tag-blue {
  background: var(--blue-wash);
  box-shadow: none;
  color: #0068d6;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.log-device {
  width: 150px;
  flex: none;
  height: 34px;
}

.log-remote {
  width: 150px;
  flex: none;
  height: 34px;
}

.log-type {
  width: 176px;
  flex: none;
  height: 34px;
}

.log-cards {
  display: none;
  flex-direction: column;
}

.log-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.log-card:last-child {
  border-bottom: 0;
}

.log-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
}

.log-card-ago {
  font-size: 11px;
  color: var(--faint);
}

.log-card-note {
  margin-top: 8px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--muted);
}

.log-card-time {
  margin-top: 8px;
  font-size: 11px;
  color: var(--faint);
}

.log-card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.log-card-delete {
  margin-left: auto;
  color: var(--red);
}

@media (max-width: 960px) {
  .log-cards {
    display: flex;
  }

  .log-tabs {
    grid-template-columns: 1fr;
  }

  .log-device,
  .log-remote,
  .log-type {
    width: 100%;
    flex: 1 1 100%;
  }
}
</style>
