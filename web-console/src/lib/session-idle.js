import { ACTIVITY_KEY, STORAGE_KEY, clearSession, session } from './session'
import { notify } from './toast'

export const IDLE_TIMEOUT_MS = 30 * 60 * 1000

const TOUCH_WRITE_INTERVAL_MS = 10 * 1000
const CHECK_INTERVAL_MS = 15 * 1000
const ACTIVITY_EVENTS = [
  'mousedown',
  'mousemove',
  'keydown',
  'wheel',
  'touchstart',
  'pointerdown',
  'scroll',
]

let lastActive = 0
let lastTouchWrite = 0
let installed = false

function storedActivity() {
  const value = Number(localStorage.getItem(ACTIVITY_KEY))
  return Number.isFinite(value) && value > 0 ? value : 0
}

function syncActivity() {
  const stored = storedActivity()
  if (stored > lastActive) lastActive = stored
}

function remember(now) {
  lastActive = now
  if (now - lastTouchWrite < TOUCH_WRITE_INTERVAL_MS) return
  lastTouchWrite = now
  localStorage.setItem(ACTIVITY_KEY, String(now))
}

export function isIdleExpired(now = Date.now()) {
  if (!session.token) return false
  syncActivity()
  if (!lastActive) lastActive = now
  return now - lastActive >= IDLE_TIMEOUT_MS
}

export function expireSession(reason = '长时间未操作，已自动退出登录。') {
  clearSession()
  notify(reason, 'error')
}

export function startIdleLogout(router) {
  if (installed) return
  installed = true
  lastActive = storedActivity() || Date.now()
  lastTouchWrite = lastActive
  localStorage.setItem(ACTIVITY_KEY, String(lastActive))

  const onActivity = () => {
    if (session.token) remember(Date.now())
  }
  ACTIVITY_EVENTS.forEach((name) =>
    window.addEventListener(name, onActivity, { capture: true, passive: true })
  )

  const redirect = () => {
    if (router.currentRoute.value.path !== '/login') {
      router.replace({ path: '/login', query: { reason: 'idle' } })
    }
  }

  const check = () => {
    if (!isIdleExpired()) return
    expireSession()
    redirect()
  }

  document.addEventListener('visibilitychange', () => {
    if (!session.token) return
    if (document.visibilityState === 'hidden') {
      remember(Date.now())
    } else {
      check()
    }
  })

  window.addEventListener('storage', (event) => {
    if (event.key === ACTIVITY_KEY) {
      const value = Number(event.newValue)
      if (Number.isFinite(value) && value > lastActive) lastActive = value
      check()
      return
    }
    if (event.key === STORAGE_KEY && !event.newValue) {
      session.baseUrl = ''
      session.token = ''
      session.user = null
      redirect()
    }
  })

  window.setInterval(check, CHECK_INTERVAL_MS)
  check()
}
