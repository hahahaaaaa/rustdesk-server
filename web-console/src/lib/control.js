import { session } from './session'
import { notify } from './toast'

const WEB_CLIENT_KEY = 'rd-console-webclient-url'

export function deviceId(device) {
  return String(device?.id || '').replace(/\s+/g, '')
}

export function nativeControlUrl(device) {
  const id = deviceId(device)
  return id ? `rustdesk://${id}` : ''
}

export function defaultWebClientUrl(baseUrl = session.baseUrl) {
  if (!baseUrl || String(baseUrl).startsWith('demo://')) return ''
  try {
    const url = new URL(baseUrl)
    return `${url.origin}/web`
  } catch {
    return ''
  }
}

export function storedWebClientUrl() {
  return localStorage.getItem(WEB_CLIENT_KEY) || ''
}

export function webClientUrl() {
  return storedWebClientUrl().trim() || defaultWebClientUrl()
}

export function normalizeWebClientUrl(value) {
  const text = String(value || '').trim()
  if (!text) return ''
  const url = /^https?:\/\//i.test(text) ? text : `https://${text}`
  new URL(url)
  return url.replace(/\/+$/, '')
}

export function saveWebClientUrl(value) {
  const normalized = normalizeWebClientUrl(value)
  if (normalized) localStorage.setItem(WEB_CLIENT_KEY, normalized)
  else localStorage.removeItem(WEB_CLIENT_KEY)
  return normalized
}

export function resetWebClientUrl() {
  localStorage.removeItem(WEB_CLIENT_KEY)
}

export async function copyDeviceId(device) {
  const id = deviceId(device)
  if (!id) throw new Error('此设备没有可用的 RustDesk ID。')
  await navigator.clipboard.writeText(id)
  return id
}

export function openNativeControl(device) {
  const url = nativeControlUrl(device)
  if (!url) {
    notify('此设备没有可用的 RustDesk ID。', 'error')
    return false
  }
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.style.display = 'none'
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  notify('正在调用本机 RustDesk 客户端…')
  return true
}

export async function openWebControl(device) {
  if (session.demo) {
    notify('演示模式无法发起网页远控。', 'error')
    return false
  }
  const url = webClientUrl()
  if (!url) {
    notify('请先在设置中配置网页远控地址。', 'error')
    return false
  }
  try {
    await copyDeviceId(device)
    notify('设备 ID 已复制，请在网页客户端中粘贴连接。')
  } catch {
    notify('已打开网页远控，请手动输入设备 ID。')
  }
  window.open(url, '_blank', 'noopener,noreferrer')
  return true
}
