import { session, saveSession } from './session'
import { DemoApi } from './demo'

export class ApiError extends Error {
  constructor(message, status = 0) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export function normalizeBaseUrl(value) {
  let url = String(value || '').trim()
  if (!url) throw new ApiError('请填写服务器地址。')
  if (!/^https?:\/\//i.test(url)) {
    const protocol = window.location.protocol === 'http:' ? 'http:' : 'https:'
    url = `${protocol}//${url}`
  }
  url = url.replace(/\/+$/, '')
  try {
    new URL(url)
  } catch {
    throw new ApiError('请输入有效的服务器地址，例如 https://rd.example.com:21114。')
  }
  return url
}

async function parseResponse(response) {
  const text = await response.text()
  let data = null
  if (text) {
    try {
      data = JSON.parse(text)
    } catch {
      data = text
    }
  }
  if (!response.ok) {
    const message =
      (data && data.error) ||
      (typeof data === 'string' && data.trim()) ||
      `请求失败（HTTP ${response.status}）。`
    throw new ApiError(message, response.status)
  }
  if (data && typeof data === 'object' && data.error) {
    throw new ApiError(data.error, response.status)
  }
  return data
}

export async function loginToServer({ baseUrl, username, password }) {
  const base = normalizeBaseUrl(baseUrl)
  let response
  try {
    response = await fetch(`${base}/api/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username,
        password,
        deviceInfo: { os: 'web', name: 'RustDesk 控制台', type: 'web' },
      }),
    })
  } catch {
    throw new ApiError(
      '无法连接服务器。请检查地址、协议和 API 端口；如果服务器正常，可能需要通过 CORS 允许当前来源。'
    )
  }
  const data = await parseResponse(response)
  if (!data?.access_token) {
    if (data?.type === 'email_check' || data?.type === 'tfa_check') {
      throw new ApiError('此账号需要两步验证，请改用访问令牌登录。')
    }
    throw new ApiError(data?.error || '服务器未返回访问令牌。')
  }
  return {
    token: data.access_token,
    user: {
      name: data.user?.name || username,
      display_name: data.user?.display_name || data.user?.name || username,
      email: data.user?.email || '',
      is_admin: data.user?.is_admin === true,
    },
  }
}

export class ApiClient {
  constructor(baseUrl, token) {
    this.baseUrl = normalizeBaseUrl(baseUrl)
    this.token = token
  }

  async request(path, { method = 'GET', body } = {}) {
    const headers = { Authorization: `Bearer ${this.token}` }
    if (body !== undefined) headers['Content-Type'] = 'application/json'
    let response
    try {
      response = await fetch(`${this.baseUrl}${path}`, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      })
    } catch {
      throw new ApiError('网络错误，无法连接服务器。')
    }
    if (response.status === 401) {
      throw new ApiError('登录已过期，请重新登录。', 401)
    }
    return parseResponse(response)
  }

  async getDevices() {
    const pageSize = 100
    const all = []
    let current = 1
    while (current <= 20) {
      const data = await this.request(
        `/api/devices?pageSize=${pageSize}&current=${current}`
      )
      const rows = Array.isArray(data?.data) ? data.data : []
      all.push(...rows)
      const total = Number(data?.total || all.length)
      if (rows.length < pageSize || all.length >= total) break
      current += 1
    }
    return all
  }

  async getGroups() {
    const pageSize = 100
    const all = []
    let current = 1
    while (current <= 10) {
      const data = await this.request(
        `/api/device-groups?pageSize=${pageSize}&current=${current}`
      )
      const rows = Array.isArray(data?.data) ? data.data : []
      all.push(...rows)
      const total = Number(data?.total || all.length)
      if (rows.length < pageSize || all.length >= total) break
      current += 1
    }
    return all
  }

  async getUsers() {
    const data = await this.request('/api/users?pageSize=200&current=1')
    return Array.isArray(data?.data) ? data.data : []
  }

  setDeviceDisabled(guid, disabled) {
    return this.request(`/api/devices/${encodeURIComponent(guid)}/${disabled ? 'disable' : 'enable'}`, {
      method: 'POST',
    })
  }

  deleteDevice(guid) {
    return this.request(`/api/devices/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  assignDevice(guid, type, value) {
    return this.request(`/api/devices/${encodeURIComponent(guid)}/assign`, {
      method: 'POST',
      body: { type, value },
    })
  }

  createGroup({ name, note }) {
    return this.request('/api/device-groups', {
      method: 'POST',
      body: note ? { name, note } : { name },
    })
  }

  updateGroup(guid, payload) {
    return this.request(`/api/device-groups/${encodeURIComponent(guid)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteGroup(guid) {
    return this.request(`/api/device-groups/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  async testConnection() {
    await this.request('/api/devices?pageSize=1&current=1')
    return true
  }
}

let demoClient = null

export function getApi() {
  if (session.demo) {
    if (!demoClient) demoClient = new DemoApi()
    return demoClient
  }
  return new ApiClient(session.baseUrl, session.token)
}

export function startDemoSession() {
  demoClient = null
  saveSession({
    baseUrl: 'demo://local',
    token: 'demo',
    demo: true,
    user: {
      name: 'demo',
      display_name: '演示管理员',
      email: 'demo@rustdesk.local',
      is_admin: true,
    },
  })
}
