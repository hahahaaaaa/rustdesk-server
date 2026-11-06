<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  AlertTriangle,
  ArrowUp,
  Download,
  File as FileIcon,
  Folder,
  HardDrive,
  Loader2,
  RefreshCw,
  Upload,
  X,
} from 'lucide-vue-next'
import { getApi } from '../lib/api'
import { FILE_TYPE, FileTransferClient, joinPath, parentPath } from '../lib/webcontrol/file-transfer'
import { session } from '../lib/session'
import { notify } from '../lib/toast'

const props = defineProps({
  guid: { type: String, required: true },
  // Password used by the remote-control session; tried once before prompting.
  password: { type: String, default: '' },
})
const emit = defineEmits(['close'])

const api = getApi()

const phaseText = ref('正在建立文件传输连接…')
const error = ref('')
const connected = ref(false)
const separator = ref('\\')
const passwordOpen = ref(false)
const passwordWrong = ref(false)
const password = ref('')
const passwordInput = ref(null)
const currentPath = ref('')
const pathDraft = ref('')
const entries = ref([])
const loading = ref(false)
const showHidden = ref(false)
const jobs = ref([])
const fileInput = ref(null)
const dragging = ref(false)

let client = null
let disposed = false

const isDriveList = computed(() => separator.value === '\\' && currentPath.value === '/')
const canUpload = computed(() => connected.value && currentPath.value && !isDriveList.value)
const sortedEntries = computed(() =>
  entries.value
    .filter((entry) => showHidden.value || !entry.hidden)
    .slice()
    .sort((a, b) => {
      const da = isDir(a) ? 0 : 1
      const db = isDir(b) ? 0 : 1
      if (da !== db) return da - db
      return a.name.localeCompare(b.name, 'zh-CN', { numeric: true })
    })
)
const hasFinishedJobs = computed(() =>
  jobs.value.some((job) => ['done', 'error', 'canceled', 'skipped'].includes(job.state))
)

function isDir(entry) {
  return [FILE_TYPE.Dir, FILE_TYPE.DirLink, FILE_TYPE.DirDrive].includes(entry.type)
}

function handlers() {
  return {
    onPhase: ({ text }) => {
      phaseText.value = text
    },
    onPasswordRequired: ({ wrong }) => {
      passwordWrong.value = wrong
      passwordOpen.value = true
      if (wrong) password.value = ''
      nextTick(() => passwordInput.value?.focus())
    },
    onPeer: ({ separator: sep }) => {
      connected.value = true
      passwordOpen.value = false
      error.value = ''
      separator.value = sep || '/'
      loading.value = true
    },
    onDirectory: ({ path, entries: list }) => {
      loading.value = false
      currentPath.value = path
      pathDraft.value = path
      entries.value = list
    },
    onDirectoryError: (text) => {
      loading.value = false
      pathDraft.value = currentPath.value
      notify(text, 'error')
    },
    onJobs: (list) => {
      jobs.value = list
    },
    onUploadDone: ({ dir }) => {
      if (dir === currentPath.value) open(currentPath.value)
    },
    onNotice: (text) => {
      if (text) notify(text, 'info')
    },
    onMessageBox: ({ title, text }) => {
      notify(`${title}${text ? `：${text}` : ''}`, 'info')
    },
    onPeerClosed: ({ reason }) => {
      error.value = reason
    },
    onError: (message) => {
      error.value = message
      connected.value = false
      passwordOpen.value = false
      loading.value = false
    },
    onClosed: (reason) => {
      connected.value = false
      if (!error.value) error.value = reason
    },
  }
}

async function connect() {
  teardown()
  error.value = ''
  connected.value = false
  passwordOpen.value = false
  passwordWrong.value = false
  entries.value = []
  currentPath.value = ''
  pathDraft.value = ''
  phaseText.value = '正在建立文件传输连接…'
  try {
    const data = await api.createWebControlSession(props.guid)
    if (disposed) return
    if (!data.device?.online) throw new Error('设备当前离线。')
    client = new FileTransferClient({
      session: { ...data, baseUrl: session.baseUrl },
      handlers: handlers(),
      password: props.password,
    })
    client.setShowHidden(showHidden.value)
    await client.start()
  } catch (e) {
    if (disposed) return
    error.value = e?.message || '无法建立文件传输连接。'
  }
}

function teardown() {
  if (!client) return
  try {
    client.close('文件传输已关闭。')
  } catch {
    // Ignore teardown failures.
  }
  client = null
}

function submitPassword() {
  if (!client) return
  client.submitPassword(password.value)
  passwordOpen.value = false
  phaseText.value = '正在验证密码…'
}

function requestApproval() {
  if (!client) return
  client.requestApproval()
  passwordOpen.value = false
  notify('已发送连接请求，请在对方设备上点击“接受”。', 'info')
}

function open(path) {
  if (!client || !connected.value) return
  loading.value = true
  client.readDir(path)
}

function openEntry(entry) {
  if (!isDir(entry)) return
  // Drive entries are reported as "C:"; list them from their root.
  const path =
    entry.type === FILE_TYPE.DirDrive
      ? `${entry.name.replace(/[\\/]+$/, '')}\\`
      : joinPath(currentPath.value, entry.name, separator.value)
  open(path)
}

function goUp() {
  open(parentPath(currentPath.value, separator.value))
}

function goToDraft() {
  const value = pathDraft.value.trim()
  open(value)
}

function toggleHidden() {
  showHidden.value = !showHidden.value
  client?.setShowHidden(showHidden.value)
  if (connected.value) open(currentPath.value)
}

function pickFiles() {
  fileInput.value?.click()
}

function uploadList(list) {
  if (!client || !canUpload.value) {
    notify('请先进入一个具体的文件夹再上传。', 'info')
    return
  }
  for (const file of list) client.uploadFile(file, currentPath.value)
}

function onFilesPicked(event) {
  const list = [...(event.target.files || [])]
  event.target.value = ''
  if (list.length) uploadList(list)
}

function onDrop(event) {
  dragging.value = false
  const items = [...(event.dataTransfer?.items || [])]
  if (items.some((item) => item.webkitGetAsEntry?.()?.isDirectory)) {
    notify('暂不支持上传文件夹，已忽略其中的文件夹。', 'info')
  }
  const list = [...(event.dataTransfer?.files || [])].filter((file, index) => {
    const entry = items[index]?.webkitGetAsEntry?.()
    return !entry || entry.isFile
  })
  if (list.length) uploadList(list)
}

function download(entry) {
  client?.downloadFile(entry)
}

function cancelJob(id) {
  client?.cancelJob(id)
}

function confirmOverwrite(id, overwrite) {
  client?.confirmOverwrite(id, overwrite)
}

function clearFinished() {
  client?.clearFinished()
}

function formatSize(value) {
  const size = Number(value || 0)
  if (size < 1024) return `${size} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let current = size / 1024
  let index = 0
  while (current >= 1024 && index < units.length - 1) {
    current /= 1024
    index += 1
  }
  return `${current.toFixed(current >= 100 ? 0 : 1)} ${units[index]}`
}

function formatTime(seconds) {
  if (!seconds) return ''
  const date = new Date(seconds * 1000)
  const pad = (n) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

function jobPercent(job) {
  if (!job.total) return job.state === 'done' ? 100 : 0
  return Math.min(100, Math.round((job.transferred / job.total) * 100))
}

function jobStatus(job) {
  switch (job.state) {
    case 'waiting':
      return '等待对方响应…'
    case 'confirm':
      return job.message || '对方已存在同名文件。'
    case 'running':
      return `${formatSize(job.transferred)} / ${formatSize(job.total)}`
    case 'finishing':
      return '正在完成…'
    case 'done':
      return job.kind === 'upload' ? '上传完成' : '下载完成'
    default:
      return job.message || ''
  }
}

onMounted(connect)

onBeforeUnmount(() => {
  disposed = true
  teardown()
})
</script>

<template>
  <div
    class="ft-panel"
    :class="{ dragging }"
    @dragover.prevent="dragging = canUpload"
    @dragleave.self="dragging = false"
    @drop.prevent="onDrop"
  >
    <div class="ft-head">
      <h3>文件传输</h3>
      <button class="wc-icon" type="button" title="关闭" @click="emit('close')">
        <X :size="16" />
      </button>
    </div>

    <div v-if="error" class="ft-state">
      <AlertTriangle :size="22" class="ft-state-icon error" />
      <p>{{ error }}</p>
      <button class="wc-btn ghost" type="button" @click="connect">重新连接</button>
    </div>

    <div v-else-if="passwordOpen" class="ft-state">
      <p>{{ passwordWrong ? '密码错误，请重新输入。' : '文件传输需要设备连接密码。' }}</p>
      <input
        ref="passwordInput"
        v-model="password"
        class="wc-input"
        type="password"
        autocomplete="off"
        placeholder="设备连接密码"
        @keyup.enter="submitPassword"
      />
      <div class="ft-row">
        <button class="wc-btn ghost" type="button" @click="requestApproval">请求对方确认</button>
        <button class="wc-btn primary" type="button" @click="submitPassword">连接</button>
      </div>
    </div>

    <div v-else-if="!connected" class="ft-state">
      <Loader2 :size="22" class="ft-state-icon spin" />
      <p>{{ phaseText }}</p>
    </div>

    <template v-else>
      <div class="ft-toolbar">
        <button class="wc-icon" type="button" title="上一级" @click="goUp">
          <ArrowUp :size="16" />
        </button>
        <input
          v-model="pathDraft"
          class="ft-path"
          spellcheck="false"
          placeholder="输入路径后回车"
          @keyup.enter="goToDraft"
        />
        <button class="wc-icon" type="button" title="刷新" @click="open(currentPath)">
          <RefreshCw :size="16" />
        </button>
      </div>
      <div class="ft-toolbar">
        <button class="wc-mini" type="button" :disabled="!canUpload" @click="pickFiles">
          <Upload :size="13" />
          上传文件
        </button>
        <label class="ft-check">
          <input type="checkbox" :checked="showHidden" @change="toggleHidden" />
          显示隐藏文件
        </label>
        <input ref="fileInput" type="file" multiple hidden @change="onFilesPicked" />
      </div>

      <div class="ft-list">
        <div v-if="loading" class="ft-empty">
          <Loader2 :size="16" class="spin" />
          正在读取…
        </div>
        <div v-else-if="!sortedEntries.length" class="ft-empty">文件夹为空</div>
        <template v-else>
        <div
          v-for="entry in sortedEntries"
          :key="entry.name"
          class="ft-item"
          :class="{ dir: isDir(entry) }"
          @dblclick="openEntry(entry)"
        >
          <HardDrive v-if="entry.type === FILE_TYPE.DirDrive" :size="15" class="ft-icon" />
          <Folder v-else-if="isDir(entry)" :size="15" class="ft-icon dir" />
          <FileIcon v-else :size="15" class="ft-icon" />
          <button
            v-if="isDir(entry)"
            class="ft-name link"
            type="button"
            :title="entry.name"
            @click="openEntry(entry)"
          >
            {{ entry.name }}
          </button>
          <span v-else class="ft-name" :title="entry.name">{{ entry.name }}</span>
          <span v-if="!isDir(entry)" class="ft-meta" :title="formatTime(entry.modified)">
            {{ formatSize(entry.size) }}
          </span>
          <button
            v-if="!isDir(entry)"
            class="wc-icon ft-action"
            type="button"
            title="下载"
            @click="download(entry)"
          >
            <Download :size="15" />
          </button>
        </div>
        </template>
      </div>
      <p class="ft-tip">可将本机文件拖到此处上传到当前文件夹。</p>
    </template>

    <section v-if="jobs.length" class="ft-jobs">
      <div class="ft-jobs-head">
        <span>传输任务</span>
        <button v-if="hasFinishedJobs" class="wc-mini" type="button" @click="clearFinished">
          清除已结束
        </button>
      </div>
      <div v-for="job in jobs" :key="job.id" class="ft-job" :class="job.state">
        <div class="ft-job-line">
          <Upload v-if="job.kind === 'upload'" :size="13" />
          <Download v-else :size="13" />
          <span class="ft-name" :title="job.name">{{ job.name }}</span>
          <button
            v-if="['waiting', 'running', 'confirm'].includes(job.state)"
            class="wc-icon ft-action"
            type="button"
            title="取消"
            @click="cancelJob(job.id)"
          >
            <X :size="13" />
          </button>
        </div>
        <div v-if="['running', 'finishing'].includes(job.state)" class="ft-bar">
          <span :style="{ width: `${jobPercent(job)}%` }" />
        </div>
        <div class="ft-job-status">{{ jobStatus(job) }}</div>
        <div v-if="job.state === 'confirm'" class="ft-row">
          <button class="wc-mini" type="button" @click="confirmOverwrite(job.id, false)">跳过</button>
          <button class="wc-mini" type="button" @click="confirmOverwrite(job.id, true)">覆盖</button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.ft-panel {
  position: absolute;
  top: 54px;
  right: 12px;
  bottom: 12px;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: min(440px, calc(100vw - 24px));
  padding: 16px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  background: #14171c;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
  color: #e8eaed;
}

.ft-panel.dragging {
  border-color: rgba(62, 207, 142, 0.7);
}

.ft-head,
.ft-toolbar,
.ft-row,
.ft-jobs-head,
.ft-job-line {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ft-head {
  justify-content: space-between;
}

.ft-head h3 {
  font-size: 14px;
  font-weight: 600;
}

.ft-row {
  justify-content: flex-end;
}

.ft-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 28px 8px;
  color: #9aa3b0;
  font-size: 13px;
  text-align: center;
}

.ft-state .ft-row {
  align-self: stretch;
}

.ft-state-icon.error {
  color: #f59e0b;
}

.ft-path {
  flex: 1;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  outline: none;
  background: #0d0f12;
  color: #e8eaed;
  font-family: var(--font-mono);
  font-size: 12px;
}

.ft-path:focus {
  border-color: rgba(255, 255, 255, 0.26);
}

.ft-check {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #aab2bf;
  font-size: 12px;
}

.ft-list {
  flex: 1;
  min-height: 120px;
  overflow-y: auto;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: #0f1216;
}

.ft-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  color: #6f7885;
  font-size: 12px;
}

.ft-item {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 0 6px 0 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  font-size: 13px;
}

.ft-item:hover {
  background: rgba(255, 255, 255, 0.04);
}

.ft-icon {
  flex: none;
  color: #8b95a3;
}

.ft-icon.dir {
  color: #e0b25b;
}

.ft-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ft-name.link {
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.ft-meta {
  flex: none;
  color: #6f7885;
  font-family: var(--font-mono);
  font-size: 11px;
}

.ft-action {
  flex: none;
  width: 26px;
  height: 26px;
}

.ft-tip {
  color: #6f7885;
  font-size: 11px;
}

.ft-jobs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 38%;
  overflow-y: auto;
  padding-top: 10px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.ft-jobs-head {
  justify-content: space-between;
  color: #aab2bf;
  font-size: 12px;
}

.ft-job {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 8px 10px;
  border-radius: 8px;
  background: #0f1216;
  font-size: 12px;
}

.ft-job-status {
  color: #8b95a3;
  font-size: 11px;
}

.ft-job.error .ft-job-status {
  color: #f87171;
}

.ft-job.done .ft-job-status {
  color: #3ecf8e;
}

.ft-bar {
  height: 4px;
  overflow: hidden;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.08);
}

.ft-bar span {
  display: block;
  height: 100%;
  background: #3ecf8e;
  transition: width 0.2s;
}

.wc-icon {
  display: inline-grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: #c7cdd6;
}

.wc-icon:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.wc-mini {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  background: transparent;
  color: #c7cdd6;
  font-size: 11px;
}

.wc-mini:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.07);
}

.wc-mini:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.wc-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 34px;
  padding: 0 14px;
  border: 1px solid transparent;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
}

.wc-btn.primary {
  background: #f5f6f7;
  color: #15181d;
}

.wc-btn.ghost {
  border-color: rgba(255, 255, 255, 0.14);
  background: transparent;
  color: #d3d8e0;
}

.wc-input {
  width: 100%;
  height: 38px;
  padding: 0 12px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 8px;
  outline: none;
  background: #0d0f12;
  color: #e8eaed;
}

.spin {
  animation: ft-spin 1s linear infinite;
}

@keyframes ft-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
