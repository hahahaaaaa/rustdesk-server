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
} from '../lib/control'
import { notify } from '../lib/toast'

const props = defineProps({
  device: { type: Object, required: true },
})

const emit = defineEmits(['close'])

const name = computed(() => deviceName(props.device))
const state = computed(() => deviceState(props.device))
const online = computed(() => state.value === 'online')
const webReady = computed(() => online.value)
const webHint = computed(() => {
  if (!online.value) return '设备离线或已禁用，无法发起网页远控。'
  return '在浏览器中直接操作远程桌面，无需安装额外客户端。'
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
        <span class="faint">网页远控由本控制台直接提供，建议使用最新版 Chrome 或 Edge。</span>
        <button class="btn btn-ghost btn-sm" type="button" @click="copyId">
          <Copy :size="13" />
          复制设备 ID
        </button>
      </div>
    </div>
  </div>
</template>
