<script setup>
import { computed, onBeforeUnmount, onMounted } from 'vue'
import {
  Copy,
  Globe,
  MonitorPlay,
  X,
} from 'lucide-vue-next'
import OsIcon from './OsIcon.vue'
import StatusPill from './StatusPill.vue'
import { deviceName, deviceOs, deviceState, formatId } from '../lib/format'
import {
  copyDeviceId,
  openNativeControl,
  openWebControl,
  webClientUrl,
} from '../lib/control'
import { session } from '../lib/session'
import { notify } from '../lib/toast'

const props = defineProps({
  device: { type: Object, required: true },
})

const emit = defineEmits(['close'])

const name = computed(() => deviceName(props.device))
const state = computed(() => deviceState(props.device))
const targetWebUrl = computed(() => webClientUrl())
const webReady = computed(() => !session.demo && Boolean(targetWebUrl.value))
const webHint = computed(() => {
  if (session.demo) return '演示模式不连接真实服务器，网页远控不可用。'
  if (!targetWebUrl.value) return '请先在设置中配置网页远控地址。'
  return `将打开 ${targetWebUrl.value}，设备 ID 会自动复制。`
})

function controlNative() {
  if (openNativeControl(props.device)) emit('close')
}

function controlWeb() {
  openWebControl(props.device).then((opened) => {
    if (opened) emit('close')
  })
}

async function copyId() {
  try {
    await copyDeviceId(props.device)
    notify('设备 ID 已复制。')
  } catch (e) {
    notify(e.message || '无法复制设备 ID。', 'error')
  }
}

function onKey(event) {
  if (event.key === 'Escape') emit('close')
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="modal-scrim" @click.self="emit('close')">
    <div class="modal control-modal" role="dialog" aria-modal="true" aria-label="远程控制">
      <div class="control-head">
        <div>
          <h2 class="modal-title">远程控制</h2>
          <p class="modal-sub">选择一种方式连接这台设备。</p>
        </div>
        <button
          class="icon-btn"
          type="button"
          title="关闭"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X :size="18" />
        </button>
      </div>

      <div class="control-device">
        <div class="device-icon"><OsIcon :os="deviceOs(device)" :size="16" /></div>
        <div class="device-meta" style="flex: 1">
          <div class="device-name">{{ name }}</div>
          <div class="device-id">{{ formatId(device.id) }}</div>
        </div>
        <StatusPill :state="state" />
      </div>

      <div class="control-options">
        <button class="control-option" type="button" @click="controlNative">
          <span class="control-icon"><MonitorPlay :size="17" /></span>
          <span class="control-body">
            <strong>本机客户端</strong>
            <span class="control-desc">通过 rustdesk:// 链接调用已安装的 RustDesk 客户端。</span>
          </span>
        </button>
        <button
          class="control-option"
          type="button"
          :disabled="!webReady"
          @click="controlWeb"
        >
          <span class="control-icon"><Globe :size="17" /></span>
          <span class="control-body">
            <strong>网页远控</strong>
            <span class="control-desc">{{ webHint }}</span>
          </span>
        </button>
      </div>

      <div class="control-foot">
        <span class="faint">需要 RustDesk Server Pro 的网页客户端支持。</span>
        <button class="btn btn-ghost btn-sm" type="button" @click="copyId">
          <Copy :size="13" />
          复制设备 ID
        </button>
      </div>
    </div>
  </div>
</template>
