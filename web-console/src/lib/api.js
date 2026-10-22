import { session } from './session'

export class ApiError extends Error {
  constructor(message, status = 0) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export function normalizeBaseUrl(value) {
  let url = String(value || '').trim() || window.location.origin
  if (!/^https?:\/\//i.test(url)) {
    const protocol = window.location.protocol === 'http:' ? 'http:' : 'https:'
    url = `${protocol}//${url}`
  }
  url = url.replace(/\/+$/, '')
  try {
    new URL(url)
  } catch {
    throw new ApiError('服务器地址无效。')
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

export function mapUser(raw, fallbackName = '') {
  const user = raw || {}
  return {
    id: user.id ?? null,
    name: user.name || fallbackName,
    display_name: user.display_name || user.name || fallbackName,
    email: user.email || '',
    is_admin: user.is_admin === true,
    permissions: Array.isArray(user.permissions) ? user.permissions : [],
    roles: Array.isArray(user.roles) ? user.roles : [],
    info: user.info || {},
    third_auth_type: user.third_auth_type || null,
  }
}

export async function loginToServer({
  baseUrl,
  username,
  password,
  tfaCode = '',
  tfaSecret = '',
}) {
  const base = normalizeBaseUrl(baseUrl)
  const payload = {
    username,
    password,
    deviceInfo: { os: 'web', name: 'RustDesk 控制台', type: 'web' },
  }
  if (tfaCode) {
    payload.type = 'email_code'
    payload.verificationCode = tfaCode
    payload.tfaCode = tfaCode
    payload.secret = tfaSecret
  }
  let response
  try {
    response = await fetch(`${base}/api/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
  } catch {
    throw new ApiError('无法连接服务器，请确认 RustDesk 服务正在运行。')
  }
  const data = await parseResponse(response)
  if (
    !data?.access_token &&
    (data?.type === 'email_check' || data?.type === 'tfa_check')
  ) {
    return {
      tfaRequired: true,
      tfaType: data.tfa_type || 'totp',
      challenge: data.secret || '',
      user: data.user || null,
    }
  }
  if (!data?.access_token) {
    throw new ApiError(data?.error || '服务器未返回访问令牌。')
  }
  return {
    token: data.access_token,
    user: mapUser(data.user, username),
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

  createUser(payload) {
    return this.request('/api/users', { method: 'POST', body: payload })
  }

  updateUser(id, payload) {
    return this.request(`/api/users/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteUser(id) {
    return this.request(`/api/users/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    })
  }

  async getUserGroups(id) {
    const data = await this.request(
      `/api/users/${encodeURIComponent(id)}/device-groups`
    )
    return Array.isArray(data?.groups) ? data.groups : []
  }

  setUserGroups(id, groups) {
    return this.request(`/api/users/${encodeURIComponent(id)}/device-groups`, {
      method: 'PUT',
      body: { groups },
    })
  }

  async getTfaStatus() {
    const data = await this.request('/api/2fa')
    return { enabled: data?.enabled === true }
  }

  setupTfa() {
    return this.request('/api/2fa/setup', { method: 'POST' })
  }

  enableTfa(code) {
    return this.request('/api/2fa/enable', {
      method: 'POST',
      body: { code },
    })
  }

  disableTfa({ password, code }) {
    return this.request('/api/2fa/disable', {
      method: 'POST',
      body: { password, code },
    })
  }

  resetUserTfa(id) {
    return this.request(`/api/users/${encodeURIComponent(id)}/2fa/reset`, {
      method: 'POST',
    })
  }

  getAuditLogs({ current = 1, pageSize = 50, actor = '', action = '', q = '' } = {}) {
    const params = new URLSearchParams({
      current: String(current),
      pageSize: String(pageSize),
    })
    if (actor.trim()) params.set('actor', actor.trim())
    if (action.trim()) params.set('action', action.trim())
    if (q.trim()) params.set('q', q.trim())
    return this.request(`/api/audit?${params.toString()}`)
  }

  async getAuditActions() {
    const data = await this.request('/api/audit/actions')
    return Array.isArray(data?.actions) ? data.actions : []
  }

  async getAddressBooks({ q = '' } = {}) {
    const pageSize = 100
    const all = []
    let current = 1
    while (current <= 10) {
      const params = new URLSearchParams({
        current: String(current),
        pageSize: String(pageSize),
      })
      if (q.trim()) params.set('q', q.trim())
      const data = await this.request(`/api/address-books?${params.toString()}`)
      const rows = Array.isArray(data?.data) ? data.data : []
      all.push(...rows)
      const total = Number(data?.total || all.length)
      if (rows.length < pageSize || all.length >= total) break
      current += 1
    }
    return all
  }

  createAddressBook(payload) {
    return this.request('/api/address-books', { method: 'POST', body: payload })
  }

  updateAddressBook(guid, payload) {
    return this.request(`/api/address-books/${encodeURIComponent(guid)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteAddressBook(guid) {
    return this.request(`/api/address-books/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  async getStrategies() {
    const data = await this.request('/api/strategies')
    return Array.isArray(data) ? data : []
  }

  createStrategy(payload) {
    return this.request('/api/strategies', { method: 'POST', body: payload })
  }

  updateStrategy(guid, payload) {
    return this.request(`/api/strategies/${encodeURIComponent(guid)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteStrategy(guid) {
    return this.request(`/api/strategies/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  setStrategyStatus(guid, enabled) {
    return this.request(`/api/strategies/${encodeURIComponent(guid)}/status`, {
      method: 'PUT',
      body: { enabled },
    })
  }

  async getStrategyAssignments(guid) {
    const data = await this.request(
      `/api/strategies/${encodeURIComponent(guid)}/assignments`
    )
    return {
      devices: Array.isArray(data?.devices) ? data.devices : [],
      users: Array.isArray(data?.users) ? data.users : [],
      groups: Array.isArray(data?.groups) ? data.groups : [],
    }
  }

  assignStrategy({ strategy = null, peers = [], users = [], groups = [] }) {
    return this.request('/api/strategies/assign', {
      method: 'POST',
      body: {
        strategy: strategy || null,
        peers,
        users,
        groups,
      },
    })
  }

  async getAddressBookShares(guid) {
    const data = await this.request(
      `/api/address-books/${encodeURIComponent(guid)}/shares`
    )
    return Array.isArray(data?.shares) ? data.shares : []
  }

  setAddressBookShares(guid, shares) {
    return this.request(
      `/api/address-books/${encodeURIComponent(guid)}/shares`,
      { method: 'PUT', body: { shares } }
    )
  }

  async getAddressBookPeers(guid) {
    const pageSize = 100
    const all = []
    let current = 1
    while (current <= 20) {
      const data = await this.request(
        `/api/ab/peers?current=${current}&pageSize=${pageSize}&ab=${encodeURIComponent(guid)}`,
        { method: 'POST' }
      )
      const rows = Array.isArray(data?.data) ? data.data : []
      all.push(...rows)
      const total = Number(data?.total || all.length)
      if (rows.length < pageSize || all.length >= total) break
      current += 1
    }
    return all
  }

  addAddressBookPeer(guid, payload) {
    return this.request(`/api/ab/peer/add/${encodeURIComponent(guid)}`, {
      method: 'POST',
      body: payload,
    })
  }

  deleteAddressBookPeer(guid, ids) {
    return this.request(`/api/ab/peer/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
      body: ids,
    })
  }

  async getAddressBookTags(guid) {
    const data = await this.request(
      `/api/ab/tags/${encodeURIComponent(guid)}`,
      { method: 'POST' }
    )
    return Array.isArray(data) ? data : []
  }

  addAddressBookTag(guid, payload) {
    return this.request(`/api/ab/tag/add/${encodeURIComponent(guid)}`, {
      method: 'POST',
      body: payload,
    })
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

  createWebControlSession(guid) {
    return this.request('/api/web-control/session', {
      method: 'POST',
      body: { guid },
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

  getCurrentUser() {
    return this.request('/api/currentUser', { method: 'POST', body: {} })
  }

  async getConnLogs({
    current = 1,
    pageSize = 50,
    device = '',
    remote = '',
    connType = '',
    q = '',
  } = {}) {
    const params = new URLSearchParams({
      current: String(current),
      pageSize: String(pageSize),
    })
    if (device.trim()) params.set('device', device.trim())
    if (remote.trim()) params.set('remote', remote.trim())
    if (connType !== '') params.set('connType', String(connType))
    if (q.trim()) params.set('q', q.trim())
    return this.request(`/api/audits/conn?${params.toString()}`)
  }

  async getFileLogs({
    current = 1,
    pageSize = 50,
    device = '',
    remote = '',
    q = '',
  } = {}) {
    const params = new URLSearchParams({
      current: String(current),
      pageSize: String(pageSize),
    })
    if (device.trim()) params.set('device', device.trim())
    if (remote.trim()) params.set('remote', remote.trim())
    if (q.trim()) params.set('q', q.trim())
    return this.request(`/api/audits/file?${params.toString()}`)
  }

  async getAlarmLogs({
    current = 1,
    pageSize = 50,
    device = '',
    alarmType = '',
    q = '',
  } = {}) {
    const params = new URLSearchParams({
      current: String(current),
      pageSize: String(pageSize),
    })
    if (device.trim()) params.set('device', device.trim())
    if (alarmType !== '') params.set('alarmType', String(alarmType))
    if (q.trim()) params.set('q', q.trim())
    return this.request(`/api/audits/alarm?${params.toString()}`)
  }

  updateLogNote(kind, id, note) {
    return this.request(`/api/audits/${encodeURIComponent(kind)}/${id}`, {
      method: 'PATCH',
      body: { note },
    })
  }

  deleteLog(kind, id) {
    return this.request(`/api/audits/${encodeURIComponent(kind)}/${id}`, {
      method: 'DELETE',
    })
  }

  async getDeploymentTokens() {
    const data = await this.request('/api/deployment/tokens')
    return Array.isArray(data?.data) ? data.data : []
  }

  createDeploymentToken(name) {
    return this.request('/api/deployment/tokens', {
      method: 'POST',
      body: { name },
    })
  }

  updateDeploymentToken(id, payload) {
    return this.request(`/api/deployment/tokens/${id}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteDeploymentToken(id) {
    return this.request(`/api/deployment/tokens/${id}`, { method: 'DELETE' })
  }

  async getRecordings({
    current = 1,
    pageSize = 30,
    q = '',
    device = '',
    complete = null,
  } = {}) {
    const params = new URLSearchParams({
      current: String(current),
      pageSize: String(pageSize),
    })
    if (q.trim()) params.set('q', q.trim())
    if (device.trim()) params.set('device', device.trim())
    if (complete !== null) params.set('complete', complete ? '1' : '0')
    return this.request(`/api/recordings?${params.toString()}`)
  }

  updateRecordingNote(filename, note) {
    return this.request(`/api/recordings/${encodeURIComponent(filename)}`, {
      method: 'PATCH',
      body: { note },
    })
  }

  deleteRecording(filename) {
    return this.request(`/api/recordings/${encodeURIComponent(filename)}`, {
      method: 'DELETE',
    })
  }

  recordingDownloadUrl(filename) {
    return `${this.baseUrl}/api/recordings/${encodeURIComponent(
      filename
    )}/download?token=${encodeURIComponent(this.token)}`
  }

  async getRelays() {
    return this.request('/api/relays')
  }

  createRelay(payload) {
    return this.request('/api/relays', { method: 'POST', body: payload })
  }

  updateRelay(guid, payload) {
    return this.request(`/api/relays/${encodeURIComponent(guid)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteRelay(guid) {
    return this.request(`/api/relays/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  syncRelays() {
    return this.request('/api/relays/sync', { method: 'POST' })
  }

  getOidcSettings() {
    return this.request('/api/settings/oidc')
  }

  saveOidcSettings(payload) {
    return this.request('/api/settings/oidc', { method: 'PUT', body: payload })
  }

  testOidc() {
    return this.request('/api/settings/oidc/test', { method: 'POST' })
  }

  getSmtpSettings() {
    return this.request('/api/settings/smtp')
  }

  saveSmtpSettings(payload) {
    return this.request('/api/settings/smtp', { method: 'PUT', body: payload })
  }

  testSmtp(to, settings) {
    return this.request('/api/settings/smtp/test', {
      method: 'POST',
      body: { to, ...settings },
    })
  }

  getLdapSettings() {
    return this.request('/api/settings/ldap')
  }

  saveLdapSettings(payload) {
    return this.request('/api/settings/ldap', { method: 'PUT', body: payload })
  }

  testLdap(username, password, settings) {
    return this.request('/api/settings/ldap/test', {
      method: 'POST',
      body: { username, password, ...settings },
    })
  }

  async getPermissions() {
    const data = await this.request('/api/permissions')
    return Array.isArray(data?.data) ? data.data : []
  }

  async getRoles() {
    const data = await this.request('/api/roles')
    return Array.isArray(data?.data) ? data.data : []
  }

  createRole(payload) {
    return this.request('/api/roles', { method: 'POST', body: payload })
  }

  updateRole(guid, payload) {
    return this.request(`/api/roles/${encodeURIComponent(guid)}`, {
      method: 'PATCH',
      body: payload,
    })
  }

  deleteRole(guid) {
    return this.request(`/api/roles/${encodeURIComponent(guid)}`, {
      method: 'DELETE',
    })
  }

  async testConnection() {
    await this.request('/api/devices?pageSize=1&current=1')
    return true
  }
}

export function getApi() {
  return new ApiClient(session.baseUrl || window.location.origin, session.token)
}

export async function getOidcStatus() {
  let response
  try {
    response = await fetch(`${window.location.origin}/api/oidc/status`, {
      headers: { Accept: 'application/json' },
    })
  } catch {
    return { enabled: false, name: '' }
  }
  const data = await parseResponse(response)
  return { enabled: data?.enabled === true, name: data?.name || '单点登录' }
}

export async function startOidcLogin() {
  const base = window.location.origin
  let response
  try {
    response = await fetch(`${base}/api/oidc/auth`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        op: 'web',
        id: '',
        uuid: '',
        deviceInfo: { os: 'web', name: 'RustDesk 控制台', type: 'web' },
        apiDomain: base,
      }),
    })
  } catch {
    throw new ApiError('无法连接服务器，请稍后重试。')
  }
  const data = await parseResponse(response)
  if (!data?.url) throw new ApiError('服务器未返回单点登录地址。')
  return data.url
}

export async function exchangeOidcCode(code) {
  let response
  try {
    response = await fetch(`${window.location.origin}/api/oidc/exchange`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    })
  } catch {
    throw new ApiError('无法连接服务器，请稍后重试。')
  }
  const data = await parseResponse(response)
  if (!data?.access_token) throw new ApiError('单点登录未返回访问令牌。')
  return { token: data.access_token, user: mapUser(data.user) }
}
