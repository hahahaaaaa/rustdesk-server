import { session } from './session'
import { notify } from './toast'

export function deviceId(device) {
  return String(device?.id || '').replace(/\s+/g, '')
}

export function nativeControlUrl(device) {
  const id = deviceId(device)
  return id ? `rustdesk://${id}` : ''
}

export function webControlPath(device) {
  const guid = String(device?.guid || '').trim()
  return guid ? `/control/${encodeURIComponent(guid)}` : ''
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
    notify('演示模式不连接真实服务器，网页远控不可用。', 'error')
    return false
  }
  const path = webControlPath(device)
  if (!path) {
    notify('此设备缺少内部标识，无法打开网页远控。', 'error')
    return false
  }
  window.location.hash = `#${path}`
  return true
}
