export function formatId(id) {
  const digits = String(id || '').replace(/\s+/g, '')
  if (!/^\d{9,}$/.test(digits)) return String(id || '—')
  return digits.replace(/(\d{3})(?=\d)/g, '$1 ')
}

export function deviceGuid(device) {
  return device?.guid || device?.id || ''
}

export function deviceName(device) {
  return (
    device?.device_name ||
    device?.info?.device_name ||
    device?.info?.hostname ||
    device?.id ||
    '未命名设备'
  )
}

export function deviceOs(device) {
  const raw = String(
    device?.info?.os || device?.info?.platform || device?.os || ''
  ).toLowerCase()
  if (raw.includes('win')) return 'windows'
  if (raw.includes('mac') || raw.includes('darwin') || raw.includes('osx')) {
    return 'macos'
  }
  if (raw.includes('linux') || raw.includes('ubuntu') || raw.includes('debian')) {
    return 'linux'
  }
  if (raw.includes('android')) return 'android'
  if (raw.includes('ios') || raw.includes('iphone') || raw.includes('ipad')) {
    return 'ios'
  }
  return 'other'
}

export function osLabel(os) {
  return (
    {
      windows: 'Windows',
      macos: 'macOS',
      linux: 'Linux',
      android: 'Android',
      ios: 'iOS',
      other: '未知',
    }[os] || '未知'
  )
}

function parseTime(value) {
  if (!value) return null
  let normalized = String(value)
  if (!/Z$|[+-]\d{2}:?\d{2}$/.test(normalized)) normalized = `${normalized}Z`
  const date = new Date(normalized)
  return Number.isNaN(date.getTime()) ? null : date
}

export function isOnline(device) {
  if (device?.disabled) return false
  if (device?.status === 1 || device?.status === true || device?.status === 'online') {
    return true
  }
  if (device?.online === true) return true
  const last = parseTime(device?.last_online)
  if (!last) return false
  return Date.now() - last.getTime() < 2 * 60 * 1000
}

export function deviceState(device) {
  if (device?.disabled) return 'disabled'
  return isOnline(device) ? 'online' : 'offline'
}

export function timeAgo(value) {
  const date = parseTime(value)
  if (!date) return '从未'
  const seconds = Math.max(0, Math.round((Date.now() - date.getTime()) / 1000))
  if (seconds < 60) return '刚刚'
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.round(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.round(hours / 24)
  if (days < 30) return `${days} 天前`
  const months = Math.round(days / 30)
  if (months < 12) return `${months} 个月前`
  return `${Math.round(months / 12)} 年前`
}

export function exactTime(value) {
  const date = parseTime(value)
  if (!date) return '—'
  return new Intl.DateTimeFormat('zh-CN', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date)
}

export function initials(value) {
  const text = String(value || '').trim()
  if (!text) return '?'
  const parts = text.split(/[\s._-]+/).filter(Boolean)
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
  return `${parts[0][0]}${parts[parts.length - 1][0]}`.toUpperCase()
}

export function csvEscape(value) {
  const text = value == null ? '' : String(value)
  if (/[",\n]/.test(text)) return `"${text.replace(/"/g, '""')}"`
  return text
}
