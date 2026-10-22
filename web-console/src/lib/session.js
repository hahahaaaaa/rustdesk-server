import { computed, reactive } from 'vue'

const STORAGE_KEY = 'rd-console-session-v1'

function readStored() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null')
  } catch {
    return null
  }
}

const stored = readStored()
const origin = typeof window === 'undefined' ? '' : window.location.origin
const storedOrigin = String(stored?.baseUrl || '').replace(/\/+$/, '')
// Older versions allowed signing in to a remote server or a local demo.
// Drop such sessions so a stale token is never sent to this origin.
const sameService = !storedOrigin || storedOrigin === origin

// The console is served by the same service as the API, so the current origin
// is always authoritative.
export const session = reactive({
  baseUrl: origin,
  token: sameService ? stored?.token || '' : '',
  user: sameService ? stored?.user || null : null,
})

export const isAuthenticated = computed(() => Boolean(session.token))

export function saveSession({ token, user }) {
  session.baseUrl = origin
  session.token = token
  session.user = user
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      baseUrl: session.baseUrl,
      token: session.token,
      user: session.user,
    })
  )
}

export function clearSession() {
  session.baseUrl = ''
  session.token = ''
  session.user = null
  localStorage.removeItem(STORAGE_KEY)
}

export function displayName(user = session.user) {
  if (!user) return '管理员'
  return user.display_name || user.name || user.email || '管理员'
}

export function userHandle(user = session.user) {
  if (!user) return ''
  if (user.name && user.name !== user.display_name) return `@${user.name}`
  return user.email || user.name || ''
}
