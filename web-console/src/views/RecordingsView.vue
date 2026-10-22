<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  ChevronLeft,
  ChevronRight,
  Clock,
  Download,
  HardDrive,
  Pencil,
  Play,
  RefreshCw,
  Search,
  Trash2,
  Video,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime, timeAgo } from '../lib/format'
import { can, clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const rows = ref([])
const total = ref(0)
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const page = ref(1)
const pageSize = 24

const search = ref('')
const device = ref('')
const complete = ref('')
const applied = ref({ q: '', device: '', complete: '' })

const player = ref(null)
const noteTarget = ref(null)
const noteText = ref('')
const noteBusy = ref(false)
const noteError = ref('')
const deleteTarget = ref(null)
const deleteBusy = ref(false)

const canManage = computed(() => can('recording.manage'))
const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize)))
const rangeStart = computed(() =>
  total.value ? (page.value - 1) * pageSize + 1 : 0
)
const rangeEnd = computed(() => Math.min(total.value, page.value * pageSize))
const hasFilters = computed(() =>
  Boolean(applied.value.q || applied.value.device || applied.value.complete)
)

const stats = computed(() => {
  const size = rows.value.reduce((sum, row) => sum + Number(row.size || 0), 0)
  return {
    total: total.value,
    size: formatSize(size),
    complete: rows.value.filter((row) => row.complete).length,
    latest: rows.value[0]?.created_at || '',
  }
})

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const data = await api.getRecordings({
      current: page.value,
      pageSize,
      q: applied.value.q,
      device: applied.value.device,
      complete:
        applied.value.complete === ''
          ? null
          : applied.value.complete === '1',
    })
    rows.value = Array.isArray(data?.data) ? data.data : []
    total.value = Number(data?.total || 0)
  } catch (e) {
    error.value = e.message || '录像列表加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function applyFilters() {
  applied.value = {
    q: search.value.trim(),
    device: device.value.trim(),
    complete: complete.value,
  }
  page.value = 1
  load(false)
}

function resetFilters() {
  search.value = ''
  device.value = ''
  complete.value = ''
  applied.value = { q: '', device: '', complete: '' }
  page.value = 1
  load(false)
}

function setPage(next) {
  const target = Math.min(Math.max(1, next), pageCount.value)
  if (target === page.value) return
  page.value = target
  load(false)
}

function formatSize(bytes) {
  const value = Number(bytes || 0)
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  if (value < 1024 * 1024 * 1024) {
    return `${(value / (1024 * 1024)).toFixed(1)} MB`
  }
  return `${(value / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

function downloadUrl(row) {
  return api.recordingDownloadUrl(row.filename)
}

function openPlayer(row) {
  player.value = { ...row, url: downloadUrl(row) }
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
    await api.updateRecordingNote(noteTarget.value.filename, noteText.value.trim())
    notify('录像备注已保存。')
    noteTarget.value = null
    await load(false)
  } catch (e) {
    noteError.value = e.message || '备注保存失败。'
  } finally {
    noteBusy.value = false
  }
}

async function removeRecording() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteRecording(deleteTarget.value.filename)
    notify('录像已删除。')
    deleteTarget.value = null
    if (rows.value.length === 1 && page.value > 1) page.value -= 1
    await load(false)
  } catch (e) {
    notify(e.message || '录像删除失败。', 'error')
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
          <div class="page-eyebrow">运维</div>
          <h1 class="page-title">会话录像</h1>
          <p class="page-sub">
            查看客户端上传的远程会话录像，可直接在浏览器中回放、下载或归档。
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
            <span class="stat-label">录像总数</span>
            <Video :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">客户端上传的会话录像</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">当前页容量</span>
            <HardDrive :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.size }}</div>
          <div class="stat-note">当前页 {{ stats.complete }} 个已完成</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">最近上传</span>
            <Clock :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">
            {{ stats.latest ? timeAgo(stats.latest) : '—' }}
          </div>
          <div class="stat-note">当前页最新的一条录像</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载录像
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载录像</h3>
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
              placeholder="搜索文件名或备注"
              aria-label="搜索录像"
              @keyup.enter="applyFilters"
            />
          </div>
          <input
            v-model="device"
            class="input rec-device"
            type="search"
            placeholder="设备 ID"
            aria-label="按设备筛选"
            @keyup.enter="applyFilters"
          />
          <select
            v-model="complete"
            class="select rec-status"
            aria-label="按上传状态筛选"
            @change="applyFilters"
          >
            <option value="">全部状态</option>
            <option value="1">已完成</option>
            <option value="0">上传中</option>
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
            共 {{ total }} 个录像
          </span>
        </div>

        <div v-if="!rows.length" class="empty">
          <div class="empty-icon"><Video :size="18" /></div>
          <h3>{{ hasFilters ? '没有匹配的录像' : '暂无会话录像' }}</h3>
          <p>
            {{
              hasFilters
                ? '尝试调整设备、状态或搜索关键词。'
                : '在 RustDesk 客户端设置中启用「会话录像」后，会话结束时会自动上传。'
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
            <table class="table recordings-table">
              <thead>
                <tr>
                  <th>文件</th>
                  <th style="width: 160px">设备</th>
                  <th style="width: 96px">大小</th>
                  <th style="width: 110px">状态</th>
                  <th style="width: 158px">上传时间</th>
                  <th style="width: 150px">备注</th>
                  <th style="width: 150px">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.filename">
                  <td>
                    <div class="rec-file">
                      <Video :size="14" />
                      <span class="rec-name" :title="row.filename">
                        {{ row.filename }}
                      </span>
                    </div>
                  </td>
                  <td class="mono rec-sub">{{ row.device_id || '—' }}</td>
                  <td class="rec-sub">{{ formatSize(row.size) }}</td>
                  <td>
                    <span
                      class="pill"
                      :class="row.complete ? 'pill-online' : 'pill-disabled'"
                    >
                      <span class="dot" />
                      {{ row.complete ? '已完成' : '上传中' }}
                    </span>
                  </td>
                  <td>
                    <div class="rec-time">{{ exactTime(row.created_at) }}</div>
                    <div class="rec-ago">{{ timeAgo(row.created_at) }}</div>
                  </td>
                  <td>
                    <span class="rec-note" :title="row.note">
                      {{ row.note || '—' }}
                    </span>
                  </td>
                  <td>
                    <div class="row-actions">
                      <button
                        class="icon-btn"
                        type="button"
                        title="播放录像"
                        :disabled="!row.complete"
                        @click="openPlayer(row)"
                      >
                        <Play :size="14" />
                      </button>
                      <a
                        class="icon-btn"
                        :href="downloadUrl(row)"
                        :download="row.filename"
                        title="下载录像"
                      >
                        <Download :size="14" />
                      </a>
                      <button
                        v-if="canManage"
                        class="icon-btn"
                        type="button"
                        title="编辑备注"
                        @click="openNote(row)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        v-if="canManage"
                        class="icon-btn danger"
                        type="button"
                        title="删除录像"
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

          <div class="rec-cards">
            <article
              v-for="row in rows"
              :key="`card-${row.filename}`"
              class="rec-card"
            >
              <div class="rec-card-head">
                <div class="rec-file">
                  <Video :size="14" />
                  <span class="rec-name" :title="row.filename">
                    {{ row.filename }}
                  </span>
                </div>
                <span
                  class="pill"
                  :class="row.complete ? 'pill-online' : 'pill-disabled'"
                >
                  <span class="dot" />
                  {{ row.complete ? '已完成' : '上传中' }}
                </span>
              </div>
              <div class="rec-card-meta">
                <span class="mono">{{ row.device_id || '—' }}</span>
                <span>{{ formatSize(row.size) }}</span>
                <span>{{ timeAgo(row.created_at) }}</span>
              </div>
              <div v-if="row.note" class="rec-card-note">{{ row.note }}</div>
              <div class="rec-card-actions">
                <button
                  class="btn btn-ghost btn-sm"
                  type="button"
                  :disabled="!row.complete"
                  @click="openPlayer(row)"
                >
                  <Play :size="13" />
                  播放
                </button>
                <a
                  class="btn btn-ghost btn-sm"
                  :href="downloadUrl(row)"
                  :download="row.filename"
                >
                  <Download :size="13" />
                  下载
                </a>
                <button
                  v-if="canManage"
                  class="btn btn-ghost btn-sm"
                  type="button"
                  @click="openNote(row)"
                >
                  <Pencil :size="13" />
                  备注
                </button>
                <button
                  v-if="canManage"
                  class="btn btn-ghost btn-sm rec-card-delete"
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
            <span>显示 {{ rangeStart }}–{{ rangeEnd }}，共 {{ total }} 个</span>
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

    <div v-if="player" class="modal-scrim" @click.self="player = null">
      <div class="modal player-modal" role="dialog" aria-modal="true">
        <h2 class="modal-title">录像回放</h2>
        <p class="modal-sub">
          {{ player.filename }} · {{ formatSize(player.size) }}
        </p>
        <div class="modal-body">
          <video
            class="player-video"
            :src="player.url"
            controls
            autoplay
            playsinline
          >
            当前浏览器不支持内嵌视频播放，请下载录像后播放。
          </video>
          <p class="player-tip">
            若无法播放，可能是浏览器不支持该录像编码，请下载后使用本地播放器打开。
          </p>
        </div>
        <div class="modal-actions">
          <a
            class="btn btn-secondary"
            :href="player.url"
            :download="player.filename"
          >
            <Download :size="14" />
            下载录像
          </a>
          <button class="btn btn-primary" type="button" @click="player = null">
            关闭
          </button>
        </div>
      </div>
    </div>

    <div v-if="noteTarget" class="modal-scrim" @click.self="closeNote">
      <form class="modal" role="dialog" aria-modal="true" @submit.prevent="saveNote">
        <h2 class="modal-title">录像备注</h2>
        <p class="modal-sub">{{ noteTarget.filename }}</p>
        <div class="modal-body">
          <div class="field">
            <label class="label" for="recording-note">备注内容</label>
            <textarea
              id="recording-note"
              v-model="noteText"
              class="textarea"
              maxlength="240"
              placeholder="例如：客户现场排障，已确认无敏感操作"
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
      title="删除这个录像？"
      :message="`${deleteTarget.filename} 将从服务器永久删除，无法恢复。`"
      confirm-label="删除录像"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="removeRecording"
    />
  </AppShell>
</template>

<style scoped>
.recordings-table {
  min-width: 960px;
}

.recordings-table tbody tr {
  cursor: default;
}

.rec-file {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--ink-soft);
}

.rec-name {
  display: block;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 11px;
}

.rec-sub {
  font-size: 11.5px;
  color: var(--muted);
}

.rec-time {
  font-size: 12px;
  color: var(--ink);
  white-space: nowrap;
}

.rec-ago {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
}

.rec-note {
  display: block;
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11.5px;
  color: var(--muted);
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.row-actions a.icon-btn {
  display: grid;
  place-items: center;
}

.icon-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.rec-device {
  width: 150px;
  flex: none;
  height: 34px;
}

.rec-status {
  width: 130px;
  flex: none;
  height: 34px;
}

.player-modal {
  max-width: 860px;
  width: min(860px, calc(100vw - 32px));
}

.player-video {
  display: block;
  width: 100%;
  max-height: 62vh;
  border-radius: var(--radius-sm);
  background: #000;
}

.player-tip {
  margin: 10px 0 0;
  font-size: 11px;
  line-height: 1.6;
  color: var(--faint);
}

.rec-cards {
  display: none;
  flex-direction: column;
}

.rec-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.rec-card:last-child {
  border-bottom: 0;
}

.rec-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.rec-card-head .rec-file {
  flex: 1;
  min-width: 0;
}

.rec-card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  margin-top: 8px;
  font-size: 11px;
  color: var(--faint);
}

.rec-card-note {
  margin-top: 8px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--muted);
}

.rec-card-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.rec-card-delete {
  margin-left: auto;
  color: var(--red);
}

@media (max-width: 960px) {
  .rec-cards {
    display: flex;
  }

  .rec-device,
  .rec-status {
    width: 100%;
    flex: 1 1 100%;
  }
}
</style>
