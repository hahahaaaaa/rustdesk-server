<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  AlertTriangle,
  ArrowLeft,
  Clipboard,
  Copy,
  Expand,
  Loader2,
  Lock,
  LogOut,
  Monitor,
  Power,
  RefreshCw,
  Send,
  Shrink,
  X,
} from 'lucide-vue-next'
import { getApi } from '../lib/api'
import { WebControlClient } from '../lib/webcontrol/client'
import { formatId } from '../lib/format'
import { clearSession, session } from '../lib/session'
import { notify } from '../lib/toast'

const route = useRoute()
const router = useRouter()
const api = getApi()

const guid = computed(() => String(route.params.guid || ''))
const viewerEl = ref(null)
const frameEl = ref(null)
const canvasEl = ref(null)

const sessionInfo = ref(null)
const phaseText = ref('正在创建远控会话…')
const error = ref('')
const connecting = ref(true)
const connected = ref(false)
const hasVideo = ref(false)
const videoSize = ref({ width: 0, height: 0 })
const displays = ref([])
const currentDisplay = ref(0)
const quality = ref(3)
const isFullscreen = ref(false)

const passwordOpen = ref(false)
const passwordWrong = ref(false)
const password = ref('')
const passwordInput = ref(null)

const clipboardOpen = ref(false)
const remoteClipboard = ref('')
const clipboardDraft = ref('')

let client = null
let disposed = false

const title = computed(() => sessionInfo.value?.device?.name || '远程控制')
const deviceId = computed(() => sessionInfo.value?.device?.id || '')
const statusLabel = computed(() => {
  if (error.value) return '连接失败'
  if (connected.value) return '已连接'
  return phaseText.value
})
const displayOptions = computed(() =>
  displays.value.map((display, index) => ({
    value: index,
    label: display.name || `显示器 ${index + 1}`,
    online: display.online !== false,
  }))
)

function handlers() {
  return {
    onPhase: ({ text }) => {
      phaseText.value = text
    },
    onPasswordRequired: ({ wrong }) => {
      passwordWrong.value = wrong
      passwordOpen.value = true
      connecting.value = false
      if (wrong) password.value = ''
      nextTick(() => passwordInput.value?.focus())
    },
    onPeer: () => {
      connected.value = true
      connecting.value = false
      passwordOpen.value = false
      error.value = ''
      nextTick(() => frameEl.value?.focus())
    },
    onDisplays: ({ displays: list, current }) => {
      displays.value = Array.isArray(list) ? list : []
      currentDisplay.value = Number(current || 0)
    },
    onVideoSize: (size) => {
      videoSize.value = size
    },
    onClipboard: (text) => {
      remoteClipboard.value = text
      notify('已同步对方的剪贴板内容。', 'info')
    },
    onMessageBox: ({ title: boxTitle, text }) => {
      notify(`${boxTitle}${text ? `：${text}` : ''}`, 'info')
    },
    onPermission: ({ enabled }) => {
      if (!enabled) notify('对方限制了部分控制权限。', 'info')
    },
    onNotice: (text) => {
      if (text) notify(text, 'info')
    },
    onFrame: () => {
      hasVideo.value = true
    },
    onPeerClosed: ({ reason }) => {
      error.value = reason
    },
    onError: (message) => {
      error.value = message
      connecting.value = false
      connected.value = false
      passwordOpen.value = false
    },
    onClosed: (reason) => {
      connected.value = false
      if (!error.value) error.value = reason
    },
  }
}

async function bootstrap() {
  teardown()
  if (disposed) return
  error.value = ''
  hasVideo.value = false
  connected.value = false
  connecting.value = true
  passwordOpen.value = false
  password.value = ''
  passwordWrong.value = false
  displays.value = []
  currentDisplay.value = 0
  videoSize.value = { width: 0, height: 0 }
  phaseText.value = '正在创建远控会话…'

  try {
    const data = await api.createWebControlSession(guid.value)
    if (disposed) return
    sessionInfo.value = data
    if (!data.device?.online) {
      throw new Error('设备当前离线，无法发起远程控制。请确认设备已开机并启动 RustDesk。')
    }
    await nextTick()
    if (!canvasEl.value) throw new Error('远控画面初始化失败，请刷新页面重试。')
    client = new WebControlClient({
      session: { ...data, baseUrl: session.baseUrl },
      handlers: handlers(),
    })
    client.attachCanvas(canvasEl.value)
    await client.start()
  } catch (e) {
    if (disposed) return
    error.value = e?.message || '无法创建远控会话。'
    connecting.value = false
    connected.value = false
    if (e?.status === 401) {
      clearSession()
      router.replace('/login')
    }
  }
}

function teardown() {
  if (client) {
    try {
      client.close('页面已关闭。')
    } catch {
      // Ignore teardown failures.
    }
    client = null
  }
  connected.value = false
  hasVideo.value = false
}

function retry() {
  bootstrap()
}

function backToDevices() {
  router.push('/devices')
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
  phaseText.value = '等待对方在设备上确认连接…'
  notify('已发送连接请求，请在对方设备上点击“接受”。', 'info')
}

function setQuality(value) {
  quality.value = Number(value)
  client?.setQuality(quality.value)
}

function switchDisplay(value) {
  const index = Number(value)
  currentDisplay.value = index
  client?.switchDisplay(index)
}

function refresh() {
  client?.refresh()
  notify('已请求刷新画面。', 'info')
}

function ctrlAltDel() {
  client?.ctrlAltDel()
}

function lockScreen() {
  client?.lockScreen()
}

function disconnect() {
  client?.close('已断开远程控制。')
  connected.value = false
  router.push('/devices')
}

async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) await document.exitFullscreen()
    else await viewerEl.value?.requestFullscreen()
  } catch (e) {
    notify(e?.message || '无法切换全屏。', 'error')
  }
}

function openClipboard() {
  clipboardDraft.value = remoteClipboard.value
  clipboardOpen.value = true
}

async function copyRemoteClipboard() {
  const text = remoteClipboard.value
  if (!text) {
    notify('对方剪贴板为空。', 'info')
    return
  }
  try {
    await navigator.clipboard.writeText(text)
    notify('已复制对方的剪贴板内容。')
  } catch {
    notify('浏览器不允许写入剪贴板，请手动复制。', 'error')
  }
}

function sendClipboard() {
  const text = clipboardDraft.value
  if (!text) {
    notify('请先输入要发送的内容。', 'info')
    return
  }
  client?.sendClipboardText(text)
  notify('已发送到对方剪贴板。')
}

async function pasteLocalClipboard() {
  try {
    const text = await client?.pasteLocalClipboard()
    if (text) {
      clipboardDraft.value = text
      notify('已读取本地剪贴板并发送到对方。')
    }
  } catch (e) {
    notify(e?.message || '无法读取本地剪贴板。', 'error')
  }
}

function canTypeKeyboard() {
  if (!connected.value || passwordOpen.value || clipboardOpen.value) return false
  const tag = String(document.activeElement?.tagName || '').toLowerCase()
  return !['input', 'textarea', 'select'].includes(tag)
}

function onKeyDown(event) {
  if (!canTypeKeyboard()) return
  if (client?.sendKeyboardEvent(event, true)) {
    event.preventDefault()
    event.stopPropagation()
  }
}

function onKeyUp(event) {
  if (!canTypeKeyboard()) return
  if (client?.sendKeyboardEvent(event, false)) {
    event.preventDefault()
    event.stopPropagation()
  }
}

function onCompositionEnd(event) {
  if (!canTypeKeyboard()) return
  client?.sendComposition(event.data)
}

function onPointerDown(event) {
  if (!connected.value || passwordOpen.value || clipboardOpen.value) return
  frameEl.value?.focus()
  if (!event.isPrimary) return
  client?.mouseDown(event)
  try {
    canvasEl.value?.setPointerCapture(event.pointerId)
  } catch {
    // Pointer capture is best effort.
  }
}

function onPointerMove(event) {
  if (!connected.value || passwordOpen.value || clipboardOpen.value) return
  if (!event.isPrimary) return
  client?.mouseMove(event)
}

function onPointerUp(event) {
  if (!connected.value || passwordOpen.value || clipboardOpen.value) return
  if (!event.isPrimary) return
  client?.mouseUp(event)
}

function onWheel(event) {
  if (!connected.value || passwordOpen.value || clipboardOpen.value) return
  client?.mouseWheel(event)
}

function onContextMenu(event) {
  event.preventDefault()
}

function onFullscreenChange() {
  isFullscreen.value = Boolean(document.fullscreenElement)
}

function onBlur() {
  client?.releaseAllKeys()
}

function onBeforeUnload() {
  client?.close('页面已关闭。')
}

onMounted(() => {
  bootstrap()
  document.addEventListener('fullscreenchange', onFullscreenChange)
  window.addEventListener('blur', onBlur)
  window.addEventListener('beforeunload', onBeforeUnload)
})

onBeforeUnmount(() => {
  disposed = true
  teardown()
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  window.removeEventListener('blur', onBlur)
  window.removeEventListener('beforeunload', onBeforeUnload)
})
</script>

<template>
  <div ref="viewerEl" class="wc-page">
    <header class="wc-bar">
      <button class="wc-icon" type="button" title="返回设备列表" @click="backToDevices">
        <ArrowLeft :size="18" />
      </button>
      <div class="wc-title">
        <span class="wc-dot" :class="{ live: connected && !error }" />
        <div class="wc-name">
          <strong>{{ title }}</strong>
          <span v-if="deviceId" class="wc-id">{{ formatId(deviceId) }}</span>
        </div>
        <span class="wc-phase">{{ statusLabel }}</span>
      </div>
      <div class="wc-tools">
        <label v-if="displayOptions.length > 1" class="wc-select" title="切换显示器">
          <Monitor :size="15" />
          <select :value="currentDisplay" :disabled="!connected" @change="switchDisplay($event.target.value)">
            <option v-for="display in displayOptions" :key="display.value" :value="display.value">
              {{ display.label }}{{ display.online ? '' : '（离线）' }}
            </option>
          </select>
        </label>
        <label class="wc-select" title="画面质量">
          <select :value="quality" :disabled="!connected" @change="setQuality($event.target.value)">
            <option :value="2">流畅</option>
            <option :value="3">均衡</option>
            <option :value="4">高清</option>
          </select>
        </label>
        <button class="wc-icon" type="button" title="剪贴板" :disabled="!connected" @click="openClipboard">
          <Clipboard :size="17" />
        </button>
        <button class="wc-icon" type="button" title="发送 Ctrl+Alt+Del" :disabled="!connected" @click="ctrlAltDel">
          <Power :size="17" />
        </button>
        <button class="wc-icon" type="button" title="锁定远程屏幕" :disabled="!connected" @click="lockScreen">
          <Lock :size="17" />
        </button>
        <button class="wc-icon" type="button" title="刷新画面" :disabled="!connected" @click="refresh">
          <RefreshCw :size="17" />
        </button>
        <button class="wc-icon" type="button" :title="isFullscreen ? '退出全屏' : '全屏'" @click="toggleFullscreen">
          <Shrink v-if="isFullscreen" :size="17" />
          <Expand v-else :size="17" />
        </button>
        <button class="wc-disconnect" type="button" @click="disconnect">
          <LogOut :size="15" />
          断开
        </button>
      </div>
    </header>

    <main class="wc-stage">
      <div
        ref="frameEl"
        class="wc-frame"
        :class="{ ready: hasVideo && connected }"
        tabindex="0"
        @keydown="onKeyDown"
        @keyup="onKeyUp"
        @compositionend="onCompositionEnd"
        @blur="onBlur"
      >
        <canvas
          ref="canvasEl"
          class="wc-canvas"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerUp"
          @wheel.prevent="onWheel"
          @contextmenu="onContextMenu"
        />
        <div v-if="!hasVideo" class="wc-overlay">
          <template v-if="error">
            <AlertTriangle :size="30" class="wc-overlay-icon error" />
            <h2>无法连接远程设备</h2>
            <p>{{ error }}</p>
            <div class="wc-overlay-actions">
              <button class="wc-btn ghost" type="button" @click="backToDevices">返回设备列表</button>
              <button class="wc-btn primary" type="button" @click="retry">重新连接</button>
            </div>
          </template>
          <template v-else>
            <Loader2 :size="30" class="wc-overlay-icon spin" />
            <h2>{{ connected ? '已连接，正在等待画面…' : '正在建立远程控制' }}</h2>
            <p>{{ phaseText }}</p>
            <p v-if="!connected" class="wc-overlay-tip">
              首次连接时，对方设备可能会弹出“接受连接”提示，需要对方确认后才能看到画面。
            </p>
          </template>
        </div>
      </div>
      <p v-if="connected && hasVideo" class="wc-hint">
        点击画面后即可使用键盘 · 滚轮滚动远程页面 · 右键打开远程菜单
        <span v-if="videoSize.width" class="wc-resolution">
          {{ videoSize.width }} × {{ videoSize.height }}
        </span>
      </p>
    </main>

    <div v-if="clipboardOpen" class="wc-drawer">
      <div class="wc-drawer-head">
        <h3>剪贴板</h3>
        <button class="wc-icon" type="button" title="关闭" @click="clipboardOpen = false">
          <X :size="16" />
        </button>
      </div>
      <section class="wc-clip-block">
        <div class="wc-clip-head">
          <span>对方剪贴板</span>
          <button class="wc-mini" type="button" @click="copyRemoteClipboard">
            <Copy :size="13" />
            复制
          </button>
        </div>
        <textarea
          class="wc-textarea"
          :value="remoteClipboard"
          readonly
          placeholder="尚未收到对方剪贴板内容。"
          rows="4"
        />
      </section>
      <section class="wc-clip-block">
        <div class="wc-clip-head">
          <span>发送到对方</span>
          <button class="wc-mini" type="button" @click="pasteLocalClipboard">
            <Clipboard :size="13" />
            读取本机剪贴板
          </button>
        </div>
        <textarea
          v-model="clipboardDraft"
          class="wc-textarea"
          placeholder="输入要发送到远程设备的文本…"
          rows="4"
        />
        <div class="wc-clip-actions">
          <button class="wc-btn primary" type="button" @click="sendClipboard">
            <Send :size="14" />
            发送
          </button>
        </div>
      </section>
      <p class="wc-clip-tip">发送后，可在远程设备中使用 Ctrl+V 粘贴。</p>
    </div>

    <div v-if="passwordOpen" class="wc-modal-scrim">
      <div class="wc-modal" role="dialog" aria-modal="true" aria-label="连接密码">
        <h2>输入连接密码</h2>
        <p class="wc-modal-text">
          设备 <span class="mono">{{ formatId(deviceId) }}</span> 需要密码验证。
          如果对方没有设置固定密码，可以请求对方在设备上确认连接。
        </p>
        <input
          ref="passwordInput"
          v-model="password"
          class="wc-input"
          type="password"
          autocomplete="off"
          placeholder="设备连接密码"
          @keyup.enter="submitPassword"
        />
        <p v-if="passwordWrong" class="wc-modal-error">密码错误，请重新输入。</p>
        <div class="wc-modal-actions">
          <button class="wc-btn ghost" type="button" @click="requestApproval">请求对方确认</button>
          <button class="wc-btn primary" type="button" @click="submitPassword">连接</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wc-page {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  background: #0b0d10;
  color: #e8eaed;
}

.wc-bar {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 14px;
  background: rgba(18, 21, 26, 0.96);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(12px);
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
  transition: background 0.15s, color 0.15s;
}

.wc-icon:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.wc-icon:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.wc-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.wc-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6b7280;
  box-shadow: 0 0 0 3px rgba(107, 114, 128, 0.2);
}

.wc-dot.live {
  background: #3ecf8e;
  box-shadow: 0 0 0 3px rgba(62, 207, 142, 0.2);
}

.wc-name {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.wc-name strong {
  overflow: hidden;
  max-width: 260px;
  font-size: 14px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wc-id {
  font-family: var(--font-mono);
  font-size: 11px;
  color: #8b93a1;
}

.wc-phase {
  overflow: hidden;
  max-width: 320px;
  color: #8b93a1;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wc-tools {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
}

.wc-select {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #c7cdd6;
  font-size: 12px;
}

.wc-select select {
  border: 0;
  outline: 0;
  background: transparent;
  color: #e8eaed;
  font-size: 12px;
  cursor: pointer;
}

.wc-select select option {
  background: #16191e;
  color: #e8eaed;
}

.wc-select select:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.wc-disconnect {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  margin-left: 4px;
  border: 1px solid rgba(255, 91, 79, 0.4);
  border-radius: 8px;
  background: rgba(229, 72, 77, 0.12);
  color: #ff9d97;
  font-size: 12px;
  font-weight: 500;
}

.wc-disconnect:hover {
  background: rgba(229, 72, 77, 0.22);
}

.wc-stage {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 0;
  padding: 14px;
  overflow: hidden;
}

.wc-frame {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  min-height: 0;
  border-radius: 10px;
  outline: none;
  overflow: hidden;
}

.wc-frame.ready {
  background: #000;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.06);
}

.wc-canvas {
  display: block;
  max-width: 100%;
  max-height: 100%;
  width: auto;
  height: auto;
  touch-action: none;
  user-select: none;
  -webkit-user-select: none;
}

.wc-frame.ready .wc-canvas {
  cursor: default;
}

.wc-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 24px;
  text-align: center;
}

.wc-overlay h2 {
  font-size: 17px;
  font-weight: 600;
}

.wc-overlay p {
  max-width: 560px;
  color: #9aa3b0;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.wc-overlay-tip {
  color: #6f7885 !important;
  font-size: 12px !important;
}

.wc-overlay-icon {
  color: #8b93a1;
}

.wc-overlay-icon.error {
  color: #ff8d86;
}

.wc-overlay-actions {
  display: flex;
  gap: 10px;
  margin-top: 8px;
}

.wc-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
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

.wc-btn.primary:hover {
  background: #fff;
}

.wc-btn.ghost {
  border-color: rgba(255, 255, 255, 0.14);
  background: transparent;
  color: #d3d8e0;
}

.wc-btn.ghost:hover {
  background: rgba(255, 255, 255, 0.08);
}

.wc-hint {
  display: flex;
  align-items: center;
  gap: 12px;
  color: #6f7885;
  font-size: 12px;
}

.wc-resolution {
  font-family: var(--font-mono);
  color: #59616d;
}

.wc-drawer {
  position: absolute;
  top: 54px;
  right: 12px;
  bottom: 12px;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(380px, calc(100vw - 24px));
  padding: 16px;
  overflow-y: auto;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  background: #14171c;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
}

.wc-drawer-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.wc-drawer-head h3 {
  font-size: 14px;
  font-weight: 600;
}

.wc-clip-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.wc-clip-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: #aab2bf;
  font-size: 12px;
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

.wc-mini:hover {
  background: rgba(255, 255, 255, 0.07);
}

.wc-textarea {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  outline: none;
  background: #0d0f12;
  color: #e8eaed;
  font-family: var(--font-sans);
  font-size: 13px;
  line-height: 1.55;
  resize: vertical;
}

.wc-textarea:focus {
  border-color: rgba(255, 255, 255, 0.24);
}

.wc-clip-actions {
  display: flex;
  justify-content: flex-end;
}

.wc-clip-tip {
  margin-top: auto;
  color: #6f7885;
  font-size: 11px;
}

.wc-modal-scrim {
  position: absolute;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(4, 6, 9, 0.72);
  backdrop-filter: blur(4px);
}

.wc-modal {
  width: min(420px, 100%);
  padding: 22px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  background: #15181d;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
}

.wc-modal h2 {
  font-size: 17px;
  font-weight: 600;
}

.wc-modal-text {
  margin: 8px 0 16px;
  color: #9aa3b0;
  font-size: 13px;
  line-height: 1.6;
}

.wc-modal-text .mono {
  font-family: var(--font-mono);
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
  font-size: 13px;
}

.wc-input:focus {
  border-color: rgba(255, 255, 255, 0.3);
}

.wc-modal-error {
  margin-top: 8px;
  color: #ff8d86;
  font-size: 12px;
}

.wc-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 18px;
}

.wc-modal-actions .wc-btn.ghost {
  border-color: rgba(255, 255, 255, 0.14);
}

.spin {
  animation: wc-spin 1s linear infinite;
}

@keyframes wc-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 760px) {
  .wc-bar {
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 10px;
  }

  .wc-name strong {
    max-width: 140px;
  }

  .wc-phase {
    display: none;
  }

  .wc-tools {
    width: 100%;
    margin-left: 0;
    padding-bottom: 2px;
    overflow-x: auto;
  }

  .wc-select span,
  .wc-select option {
    font-size: 11px;
  }

  .wc-disconnect {
    padding: 0 10px;
  }

  .wc-stage {
    padding: 8px;
  }

  .wc-hint {
    font-size: 11px;
    text-align: center;
  }

  .wc-drawer {
    top: auto;
    left: 8px;
    right: 8px;
    bottom: 8px;
    width: auto;
    max-height: 70vh;
  }
}
</style>
