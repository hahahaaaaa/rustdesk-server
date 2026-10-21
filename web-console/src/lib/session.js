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

export const session = reactive({
  baseUrl: stored?.baseUrl || '',
  token: stored?.token || '',
  user: stored?.user || null,
  demo: stored?.demo === true,
})

export const isAuthenticated = computed(() => Boolean(session.token))

export function saveSession({ baseUrl, token, user, demo = false }) {
  session.baseUrl = baseUrl
  session.token = token
  session.user = user
  session.demo = demo
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      baseUrl: session.baseUrl,
      token: session.token,
      user: session.user,
      demo: session.demo,
    })
  )
}

export function clearSession() {
  session.baseUrl = ''
  session.token = ''
  session.user = null
  session.demo = false
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
