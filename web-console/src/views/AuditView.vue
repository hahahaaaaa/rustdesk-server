<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Activity,
  ChevronLeft,
  ChevronRight,
  Clock,
  RefreshCw,
  ScrollText,
  Search,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import { getApi } from '../lib/api'
import { exactTime, initials, timeAgo } from '../lib/format'
import { clearSession } from '../lib/session'

const router = useRouter()
const api = getApi()

const logs = ref([])
const total = ref(0)
const actions = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const page = ref(1)
const pageSize = 25

const search = ref('')
const actor = ref('')
const action = ref('')
const applied = ref({ q: '', actor: '', action: '' })

const ACTION_LABELS = {
  login: '登录',
  login_failed: '登录失败',
  logout: '退出登录',
  'user.create': '创建用户',
  'user.update': '更新用户',
  'user.delete': '删除用户',
  'user.groups': '调整分组授权',
  'user.tfa_enable': '启用双重认证',
  'user.tfa_disable': '关闭双重认证',
  'user.tfa_reset': '重置双重认证',
  'device.assign': '修改设备',
  'device.disable': '禁用设备',
  'device.enable': '启用设备',
  'device.delete': '删除设备',
  'group.create': '创建分组',
  'group.update': '更新分组',
  'group.delete': '删除分组',
  'address_book.create': '创建地址簿',
  'address_book.update': '更新地址簿',
  'address_book.delete': '删除地址簿',
  'address_book.share': '调整共享授权',
  'strategy.create': '创建策略',
  'strategy.update': '更新策略',
  'strategy.status': '启用/停用策略',
  'strategy.assign': '分配策略',
  'strategy.delete': '删除策略',
  'web_control.start': '发起网页远控',
}

const DETAIL_LABELS = {
  unknown_user: '用户名不存在',
  bad_password: '密码错误',
  bad_tfa_code: '动态验证码错误',
}

const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize)))
const rangeStart = computed(() =>
  total.value ? (page.value - 1) * pageSize + 1 : 0
)
const rangeEnd = computed(() => Math.min(total.value, page.value * pageSize))
const hasFilters = computed(
  () => Boolean(applied.value.q || applied.value.actor || applied.value.action)
)
const stats = computed(() => ({
  total: total.value,
  actions: actions.value.length,
  latest: logs.value[0]?.created_at || '',
}))

onMounted(async () => {
  await Promise.all([load(), loadActions()])
})

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const data = await api.getAuditLogs({
      current: page.value,
      pageSize,
      actor: applied.value.actor,
      action: applied.value.action,
      q: applied.value.q,
    })
    logs.value = Array.isArray(data?.data) ? data.data : []
    total.value = Number(data?.total || 0)
  } catch (e) {
    error.value = e.message || '审计日志加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

async function loadActions() {
  try {
    actions.value = await api.getAuditActions()
  } catch {
    actions.value = []
  }
}

function applyFilters() {
  applied.value = {
    q: search.value.trim(),
    actor: actor.value.trim(),
    action: action.value,
  }
  page.value = 1
  load(false)
}

function resetFilters() {
  search.value = ''
  actor.value = ''
  action.value = ''
  applied.value = { q: '', actor: '', action: '' }
  page.value = 1
  load(false)
}

function setPage(next) {
  const target = Math.min(Math.max(1, next), pageCount.value)
  if (target === page.value) return
  page.value = target
  load(false)
}

function actionLabel(value) {
  return ACTION_LABELS[value] || value
}

function actionTone(value) {
  if (
    value === 'login_failed' ||
    value.endsWith('.delete') ||
    value === 'device.disable' ||
    value === 'user.tfa_disable' ||
    value === 'user.tfa_reset'
  ) {
    return 'tag-danger'
  }
  if (
    value === 'login' ||
    value.endsWith('.create') ||
    value === 'device.enable' ||
    value === 'user.tfa_enable'
  ) {
    return 'tag-success'
  }
  if (
    value.endsWith('.update') ||
    value === 'device.assign' ||
    value === 'user.groups' ||
    value === 'address_book.share' ||
    value === 'strategy.assign' ||
    value === 'strategy.status'
  ) {
    return 'tag-info'
  }
  return ''
}

function detailText(row) {
  const detail = String(row.detail || '')
  if (DETAIL_LABELS[detail]) return DETAIL_LABELS[detail]
  if (!detail) return '—'
  if (row.action === 'device.assign') return `字段：${detail}`
  return detail
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">安全与合规</div>
          <h1 class="page-title">审计日志</h1>
          <p class="page-sub">
            记录登录、设备、分组与账号管理操作，用于安全审计和问题追溯。
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

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">{{ hasFilters ? '筛选结果' : '记录总数' }}</span>
            <ScrollText :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">
            {{ hasFilters ? '符合当前筛选条件' : '服务器保留的操作记录' }}
          </div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">操作类型</span>
            <Activity :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.actions }}</div>
          <div class="stat-note">已出现过的操作种类</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">最近事件</span>
            <Clock :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">
            {{ stats.latest ? timeAgo(stats.latest) : '—' }}
          </div>
          <div class="stat-note">当前页最新的一条记录</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载审计日志
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载审计日志</h3>
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
              placeholder="搜索目标或详情"
              aria-label="搜索审计日志"
              @keyup.enter="applyFilters"
            />
          </div>
          <input
            v-model="actor"
            class="input toolbar-actor"
            type="search"
            placeholder="操作者"
            aria-label="按操作者筛选"
            @keyup.enter="applyFilters"
          />
          <select
            v-model="action"
            class="select toolbar-action"
            aria-label="按操作类型筛选"
            @change="applyFilters"
          >
            <option value="">全部操作</option>
            <option v-for="item in actions" :key="item" :value="item">
              {{ actionLabel(item) }}
            </option>
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

        <div v-if="!logs.length" class="empty">
          <div class="empty-icon"><ScrollText :size="18" /></div>
          <h3>{{ hasFilters ? '没有匹配的记录' : '暂无审计记录' }}</h3>
          <p>
            {{
              hasFilters
                ? '尝试调整操作者、操作类型或搜索关键词。'
                : '登录、设备、分组与用户操作发生后会显示在这里。'
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
            <table class="table audit-table">
              <thead>
                <tr>
                  <th style="width: 168px">时间</th>
                  <th style="width: 176px">操作者</th>
                  <th style="width: 148px">操作</th>
                  <th>目标</th>
                  <th>详情</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in logs" :key="row.id">
                  <td>
                    <div class="audit-time">{{ exactTime(row.created_at) }}</div>
                    <div class="audit-ago">{{ timeAgo(row.created_at) }}</div>
                  </td>
                  <td>
                    <div class="actor-cell">
                      <span class="avatar-sm">{{ initials(row.actor) }}</span>
                      <span class="actor-name">{{ row.actor || '系统' }}</span>
                    </div>
                  </td>
                  <td>
                    <span class="tag" :class="actionTone(row.action)">
                      {{ actionLabel(row.action) }}
                    </span>
                  </td>
                  <td>
                    <span class="target-cell" :title="row.target">
                      {{ row.target || '—' }}
                    </span>
                  </td>
                  <td>
                    <span class="detail-cell">{{ detailText(row) }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="audit-cards">
            <article v-for="row in logs" :key="row.id" class="audit-card">
              <div class="audit-card-head">
                <span class="avatar-sm">{{ initials(row.actor) }}</span>
                <div class="audit-card-actor">
                  <span class="actor-name">{{ row.actor || '系统' }}</span>
                  <span class="audit-ago">{{ timeAgo(row.created_at) }}</span>
                </div>
                <span class="tag" :class="actionTone(row.action)">
                  {{ actionLabel(row.action) }}
                </span>
              </div>
              <div class="audit-card-line">
                <span class="audit-card-label">目标</span>
                <span class="audit-card-value">{{ row.target || '—' }}</span>
              </div>
              <div class="audit-card-line">
                <span class="audit-card-label">详情</span>
                <span class="audit-card-value">{{ detailText(row) }}</span>
              </div>
              <div class="audit-card-time">{{ exactTime(row.created_at) }}</div>
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
  </AppShell>
</template>

<style scoped>
.audit-table {
  min-width: 820px;
}

.audit-table tbody tr {
  cursor: default;
}

.audit-time {
  font-size: 12.5px;
  color: var(--ink);
  white-space: nowrap;
}

.audit-ago {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
}

.actor-cell {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.avatar-sm {
  display: grid;
  place-items: center;
  flex: none;
  width: 26px;
  height: 26px;
  border-radius: 9999px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.actor-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ink);
  font-size: 12.5px;
}

.target-cell {
  display: block;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
}

.detail-cell {
  display: block;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.tag-info {
  background: var(--blue-wash);
  box-shadow: none;
  color: #0068d6;
}

.toolbar-actor {
  width: 150px;
  flex: none;
  height: 34px;
}

.toolbar-action {
  width: 176px;
  flex: none;
  height: 34px;
}

.audit-cards {
  display: none;
  flex-direction: column;
}

.audit-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.audit-card:last-child {
  border-bottom: 0;
}

.audit-card-head {
  display: flex;
  align-items: center;
  gap: 9px;
}

.audit-card-actor {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}

.audit-card-actor .audit-ago {
  margin-top: 0;
}

.audit-card-line {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-top: 9px;
  font-size: 12px;
}

.audit-card-label {
  flex: none;
  color: var(--faint);
  font-size: 11px;
}

.audit-card-value {
  min-width: 0;
  color: var(--muted);
  overflow-wrap: anywhere;
}

.audit-card-time {
  margin-top: 9px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
}

@media (max-width: 960px) {
  .audit-cards {
    display: flex;
  }
}

@media (max-width: 720px) {
  .toolbar-actor,
  .toolbar-action {
    width: 100%;
    flex: 1 1 100%;
  }
}
</style>
