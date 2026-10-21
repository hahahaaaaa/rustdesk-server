const STORAGE_KEY = 'rd-console-demo-fleet-v3'

const GROUPS = [
  {
    guid: 'grp-design',
    name: '设计部',
    note: '设计工作室使用的色彩校准 Mac 工作站。',
  },
  {
    guid: 'grp-engineering',
    name: '工程部',
    note: '平台团队的构建、测试与预发布机器。',
  },
  {
    guid: 'grp-support',
    name: '支持部',
    note: '前台与支持团队用于远程会话的笔记本。',
  },
  {
    guid: 'grp-kiosks',
    name: '展示终端',
    note: '大厅与展厅中的公共演示设备。',
  },
]

const USERS = [
  { id: 1, name: 'maya', display_name: '陈雨桐', email: 'maya@acme.com', note: '设计团队负责人', status: 1, is_admin: true },
  { id: 2, name: 'alex', display_name: '李昂', email: 'alex@acme.com', note: '平台工程', status: 1, is_admin: false },
  { id: 3, name: 'priya', display_name: '王思远', email: 'priya@acme.com', note: '质量保障', status: 1, is_admin: false, tfa_enabled: true },
  { id: 4, name: 'noah', display_name: '周诺', email: 'noah@acme.com', note: '', status: 1, is_admin: false },
  { id: 5, name: 'lena', display_name: '赵琳', email: 'lena@acme.com', note: '', status: 0, is_admin: false },
]

const USER_GROUPS = {
  1: ['grp-design'],
  2: ['grp-engineering'],
  3: ['grp-engineering', 'grp-support'],
  4: [],
  5: ['grp-support'],
}

function isoAgo(minutes) {
  return new Date(Date.now() - minutes * 60 * 1000).toISOString()
}

function seedAudit() {
  const rows = [
    { actor: 'maya', action: 'login', target: '528610344', detail: 'macos', minutes: 0.3 },
    { actor: 'alex', action: 'device.assign', target: 'dev-win-01', detail: 'note', minutes: 42 },
    { actor: 'maya', action: 'user.groups', target: 'priya', detail: 'grp-engineering,grp-support', minutes: 95 },
    { actor: 'maya', action: 'group.create', target: 'grp-kiosks', detail: '展示终端', minutes: 180 },
    { actor: 'priya', action: 'login', target: '185440912', detail: 'android', minutes: 220 },
    { actor: 'maya', action: 'device.disable', target: 'dev-linux-02', detail: '', minutes: 340 },
    { actor: 'alex', action: 'device.enable', target: 'dev-win-01', detail: '', minutes: 400 },
    { actor: 'maya', action: 'address_book.share', target: 'ab-ops-tools', detail: '2', minutes: 430 },
    { actor: 'maya', action: 'strategy.create', target: 'st-kiosk', detail: '展厅只读', minutes: 250 },
    { actor: 'maya', action: 'strategy.assign', target: 'st-kiosk', detail: 'peers=1,users=0,groups=1', minutes: 245 },
    { actor: 'unknown', action: 'login_failed', target: '', detail: 'bad_password', minutes: 460 },
    { actor: 'maya', action: 'user.create', target: 'noah', detail: '', minutes: 620 },
    { actor: 'maya', action: 'user.update', target: 'lena', detail: '', minutes: 700 },
    { actor: 'alex', action: 'group.update', target: 'grp-engineering', detail: '工程部', minutes: 900 },
    { actor: 'maya', action: 'device.delete', target: 'dev-win-09', detail: '330188745', minutes: 1400 },
    { actor: 'maya', action: 'user.delete', target: '测试账号', detail: '', minutes: 2100 },
    { actor: 'lena', action: 'login_failed', target: '', detail: 'unknown_user', minutes: 2600 },
    { actor: 'maya', action: 'address_book.create', target: 'ab-support-kiosks', detail: '支持与展厅', minutes: 3100 },
    { actor: 'maya', action: 'group.delete', target: 'grp-legacy', detail: '', minutes: 3200 },
    { actor: 'maya', action: 'logout', target: '', detail: '', minutes: 3600 },
  ]
  return rows.map((row, index) => ({
    id: rows.length - index,
    actor: row.actor,
    action: row.action,
    target: row.target,
    detail: row.detail,
    created_at: isoAgo(row.minutes),
  }))
}

function seedAddressBooks() {
  return [
    {
      guid: 'ab-ops-tools',
      name: '运维工具',
      owner: 'maya',
      note: '值班团队使用的共享地址簿，包含服务器与网络设备。',
      info: { password: 'ops2026' },
      created_at: isoAgo(60 * 24 * 30),
      updated_at: isoAgo(180),
      tags: [
        { name: '值班', color: 4280391411 },
        { name: '网络', color: 4283215696 },
      ],
      shares: [
        { user_id: 2, rule: 2 },
        { user_id: 3, rule: 1 },
      ],
      peers: [
        {
          id: '772046198',
          hash: '',
          password: 'relay-pw',
          username: 'deploy',
          hostname: 'staging-relay',
          platform: 'linux',
          alias: '预发布中继',
          tags: ['值班'],
          forceAlwaysRelay: 'true',
          rdpPort: '',
          rdpUsername: '',
          loginName: '',
          device_group_name: '工程部',
          note: '冒烟测试用中继。',
          same_server: true,
        },
        {
          id: '336758021',
          hash: '',
          password: '',
          username: 'ci',
          hostname: 'ci-sandbox-02',
          platform: 'linux',
          alias: 'CI 沙箱',
          tags: ['值班', '网络'],
          forceAlwaysRelay: 'false',
          rdpPort: '',
          rdpUsername: '',
          loginName: '',
          device_group_name: '工程部',
          note: '',
          same_server: true,
        },
      ],
    },
    {
      guid: 'ab-design-assets',
      name: '设计资产',
      owner: 'maya',
      note: '设计团队的渲染机与色彩校准工作站。',
      info: {},
      created_at: isoAgo(60 * 24 * 12),
      updated_at: isoAgo(60 * 24 * 2),
      tags: [{ name: '渲染', color: 4294951115 }],
      shares: [{ user_id: 4, rule: 1 }],
      peers: [
        {
          id: '528610344',
          hash: '',
          password: '',
          username: 'maya',
          hostname: 'design-01',
          platform: 'macos',
          alias: '设计工作室 · iMac',
          tags: ['渲染'],
          forceAlwaysRelay: 'false',
          rdpPort: '',
          rdpUsername: '',
          loginName: '',
          device_group_name: '设计部',
          note: '渲染期间请勿重启。',
          same_server: true,
        },
      ],
    },
    {
      guid: 'ab-support-kiosks',
      name: '支持与展厅',
      owner: 'alex',
      note: '支持部笔记本与大厅演示终端，供访客排障使用。',
      info: { password: 'kiosk' },
      created_at: isoAgo(60 * 24 * 7),
      updated_at: isoAgo(60 * 20),
      tags: [],
      shares: [],
      peers: [
        {
          id: '620199438',
          hash: '',
          password: '',
          username: 'lena',
          hostname: '',
          platform: 'android',
          alias: '三星 Tab S9',
          tags: [],
          forceAlwaysRelay: 'false',
          rdpPort: '',
          rdpUsername: '',
          loginName: '',
          device_group_name: '支持部',
          note: '',
          same_server: true,
        },
        {
          id: '291663570',
          hash: '',
          password: '',
          username: 'demo',
          hostname: 'kiosk-a',
          platform: 'windows',
          alias: '展厅终端 A',
          tags: [],
          forceAlwaysRelay: 'false',
          rdpPort: '',
          rdpUsername: '',
          loginName: '',
          device_group_name: '展示终端',
          note: '',
          same_server: true,
        },
      ],
    },
  ]
}

function seedStrategies() {
  return [
    {
      guid: 'st-baseline',
      name: '安全基线',
      note: '工程与支持团队的默认远程控制安全设置。',
      enabled: true,
      config_options: {
        'access-mode': 'full',
        'verification-method': 'use-both-passwords',
        'approve-mode': 'both',
        'enable-file-transfer': 'N',
        'enable-clipboard': 'Y',
        'enable-terminal': 'N',
        'allow-auto-update': 'Y',
      },
      assignments: {
        devices: ['dev-win-01'],
        users: ['priya'],
        groups: ['grp-engineering'],
      },
      created_at: isoAgo(60 * 24 * 20),
      updated_at: isoAgo(120),
      modified_at: Math.floor(Date.now() / 1000) - 120 * 60,
    },
    {
      guid: 'st-kiosk',
      name: '展厅只读',
      note: '访客演示设备仅允许查看，关闭剪贴板、文件传输与终端。',
      enabled: true,
      config_options: {
        'access-mode': 'view',
        'approve-mode': 'click',
        'enable-file-transfer': 'N',
        'enable-clipboard': 'N',
        'enable-terminal': 'N',
        'enable-audio': 'N',
        'auto-disconnect-timeout': '10',
      },
      assignments: {
        devices: ['dev-kiosk-01'],
        users: [],
        groups: ['grp-kiosks'],
      },
      created_at: isoAgo(60 * 24 * 12),
      updated_at: isoAgo(260),
      modified_at: Math.floor(Date.now() / 1000) - 260 * 60,
    },
    {
      guid: 'st-night',
      name: '夜间维护窗口',
      note: '维护期间临时放开终端与文件传输，平时保持停用。',
      enabled: false,
      config_options: {
        'enable-terminal': 'Y',
        'enable-file-transfer': 'Y',
        'allow-auto-disconnect': 'Y',
        'auto-disconnect-timeout': '30',
      },
      assignments: { devices: [], users: ['alex'], groups: [] },
      created_at: isoAgo(60 * 24 * 8),
      updated_at: isoAgo(60 * 24 * 3),
      modified_at: Math.floor(Date.now() / 1000) - 60 * 60 * 24 * 3,
    },
  ]
}

function seedDevices() {
  return [
    {
      guid: 'dev-mac-01',
      id: '528610344',
      device_name: '设计工作室 · iMac',
      user_name: 'maya',
      device_group_name: '设计部',
      note: '主要动效设计工作站，渲染期间请勿重启。',
      status: 1,
      disabled: false,
      last_online: isoAgo(0.5),
      info: {
        os: 'macos',
        username: 'maya',
        version: '1.4.0',
        device_name: '设计工作室 · iMac',
        hostname: 'design-01',
        cpu: 'Apple M3 Pro',
      },
    },
    {
      guid: 'dev-mac-02',
      id: '614882057',
      device_name: 'MacBook Pro 16"',
      user_name: 'maya',
      device_group_name: '设计部',
      note: '',
      status: 1,
      disabled: false,
      last_online: isoAgo(1),
      info: {
        os: 'macos',
        username: 'maya',
        version: '1.4.0',
        device_name: 'MacBook Pro 16"',
        hostname: 'maya-mbp',
      },
    },
    {
      guid: 'dev-win-01',
      id: '903271684',
      device_name: '构建机 · Windows',
      user_name: 'alex',
      device_group_name: '工程部',
      note: '用于 Windows 正式版构建，保存着 EV 签名密钥。',
      status: 1,
      disabled: false,
      last_online: isoAgo(0.2),
      info: {
        os: 'windows',
        username: 'alex',
        version: '1.3.9',
        device_name: '构建机 · Windows',
        hostname: 'build-win-03',
        cpu: 'Ryzen 9 7950X',
      },
    },
    {
      guid: 'dev-win-02',
      id: '412905773',
      device_name: 'QA 测试台 04',
      user_name: 'priya',
      device_group_name: '工程部',
      note: '连接采集设备，显示缩放请保持 100%。',
      status: 0,
      disabled: false,
      last_online: isoAgo(108),
      info: {
        os: 'windows',
        username: 'qa',
        version: '1.3.7',
        device_name: 'QA 测试台 04',
        hostname: 'qa-bench-04',
      },
    },
    {
      guid: 'dev-linux-01',
      id: '772046198',
      device_name: '预发布中继 · Ubuntu',
      user_name: 'alex',
      device_group_name: '工程部',
      note: '运行预发布中继，并提供冒烟测试用的虚拟显示器。',
      status: 1,
      disabled: false,
      last_online: isoAgo(2),
      info: {
        os: 'linux',
        username: 'deploy',
        version: '1.4.0',
        device_name: '预发布中继 · Ubuntu',
        hostname: 'staging-relay',
        cpu: 'Intel Xeon E-2388G',
      },
    },
    {
      guid: 'dev-linux-02',
      id: '336758021',
      device_name: 'CI 沙箱',
      user_name: 'noah',
      device_group_name: '工程部',
      note: '',
      status: 0,
      disabled: true,
      last_online: isoAgo(60 * 24 * 6),
      info: {
        os: 'linux',
        username: 'ci',
        version: '1.3.4',
        device_name: 'CI 沙箱',
        hostname: 'ci-sandbox-02',
      },
    },
    {
      guid: 'dev-android-01',
      id: '185440912',
      device_name: 'Pixel 8 · 支持机',
      user_name: 'priya',
      device_group_name: '支持部',
      note: '用于复现客户问题的设备，Android 15。',
      status: 1,
      disabled: false,
      last_online: isoAgo(5),
      info: {
        os: 'android',
        username: 'support',
        version: '1.4.0',
        device_name: 'Pixel 8 · 支持机',
      },
    },
    {
      guid: 'dev-android-02',
      id: '620199438',
      device_name: '三星 Tab S9',
      user_name: 'lena',
      device_group_name: '支持部',
      note: '用于移动端键盘测试的平板。',
      status: 0,
      disabled: false,
      last_online: isoAgo(60 * 30),
      info: {
        os: 'android',
        username: 'lena',
        version: '1.3.8',
        device_name: '三星 Tab S9',
      },
    },
    {
      guid: 'dev-win-03',
      id: '847512003',
      device_name: '前台 · Surface',
      user_name: 'lena',
      device_group_name: '支持部',
      note: '大厅签到终端，已批准屏幕录制权限。',
      status: 1,
      disabled: false,
      last_online: isoAgo(3),
      info: {
        os: 'windows',
        username: 'frontdesk',
        version: '1.4.0',
        device_name: '前台 · Surface',
        hostname: 'frontdesk-01',
      },
    },
    {
      guid: 'dev-kiosk-01',
      id: '291663570',
      device_name: '展厅终端 A',
      user_name: 'noah',
      device_group_name: '展示终端',
      note: '仅用于访客演示，空闲 10 分钟后结束会话。',
      status: 1,
      disabled: false,
      last_online: isoAgo(0.4),
      info: {
        os: 'windows',
        username: 'demo',
        version: '1.4.0',
        device_name: '展厅终端 A',
        hostname: 'kiosk-a',
      },
    },
    {
      guid: 'dev-kiosk-02',
      id: '758310426',
      device_name: '展厅终端 B',
      user_name: 'noah',
      device_group_name: '展示终端',
      note: '',
      status: 0,
      disabled: false,
      last_online: isoAgo(60 * 52),
      info: {
        os: 'linux',
        username: 'demo',
        version: '1.3.9',
        device_name: '展厅终端 B',
        hostname: 'kiosk-b',
      },
    },
    {
      guid: 'dev-mac-03',
      id: '139084725',
      device_name: '王思远 · MacBook Air',
      user_name: 'priya',
      device_group_name: '支持部',
      note: '',
      status: 1,
      disabled: false,
      last_online: isoAgo(8),
      info: {
        os: 'macos',
        username: 'priya',
        version: '1.4.0',
        device_name: '王思远 · MacBook Air',
        hostname: 'priya-mba',
      },
    },
    {
      guid: 'dev-win-04',
      id: '560298341',
      device_name: '财务 · ThinkPad',
      user_name: 'lena',
      device_group_name: '',
      note: '财务团队迁移设备期间暂未分配。',
      status: 0,
      disabled: false,
      last_online: isoAgo(60 * 24 * 3),
      info: {
        os: 'windows',
        username: 'finance',
        version: '1.3.6',
        device_name: '财务 · ThinkPad',
        hostname: 'fin-07',
      },
    },
    {
      guid: 'dev-linux-03',
      id: '684112976',
      device_name: '监控大屏',
      user_name: 'alex',
      device_group_name: '工程部',
      note: '只读的 Grafana 展示大屏，请勿发起远程会话。',
      status: 0,
      disabled: true,
      last_online: isoAgo(60 * 24 * 12),
      info: {
        os: 'linux',
        username: 'grafana',
        version: '1.3.2',
        device_name: '监控大屏',
        hostname: 'wall-display',
      },
    },
  ]
}

function readStored() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed.devices) || !Array.isArray(parsed.groups)) {
      return null
    }
    return parsed
  } catch {
    return null
  }
}

const wait = (ms = 160) => new Promise((resolve) => setTimeout(resolve, ms))

function clone(value) {
  return JSON.parse(JSON.stringify(value))
}

function demoSecret() {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'
  let secret = ''
  for (let index = 0; index < 32; index += 1) {
    secret += alphabet[Math.floor(Math.random() * alphabet.length)]
  }
  return secret
}

function sanitizeConfig(options) {
  const out = {}
  for (const [key, value] of Object.entries(options || {})) {
    if (typeof value !== 'string') {
      throw new Error(`策略配置“${key}”的值必须是字符串。`)
    }
    const name = key.trim()
    if (!name) throw new Error('策略配置的键不能为空。')
    out[name] = value
  }
  return out
}

export class DemoApi {
  constructor() {
    const stored = readStored()
    if (stored) {
      this.devices = stored.devices
      this.groups = stored.groups
      this.users = Array.isArray(stored.users) ? stored.users : clone(USERS)
      this.users = this.users.map((user) => ({ tfa_enabled: false, ...user }))
      this.userGroups =
        stored.userGroups && typeof stored.userGroups === 'object'
          ? stored.userGroups
          : clone(USER_GROUPS)
      this.audit = Array.isArray(stored.audit) ? stored.audit : seedAudit()
      this.addressBooks = Array.isArray(stored.addressBooks)
        ? stored.addressBooks
        : seedAddressBooks()
      this.strategies = Array.isArray(stored.strategies)
        ? stored.strategies
        : seedStrategies()
      this.tfa =
        stored.tfa && typeof stored.tfa === 'object'
          ? { enabled: stored.tfa.enabled === true, pendingSecret: '' }
          : { enabled: false, pendingSecret: '' }
    } else {
      this.devices = seedDevices()
      this.groups = clone(GROUPS)
      this.users = clone(USERS)
      this.userGroups = clone(USER_GROUPS)
      this.audit = seedAudit()
      this.addressBooks = seedAddressBooks()
      this.strategies = seedStrategies()
      this.tfa = { enabled: false, pendingSecret: '' }
      this.persist()
    }
  }

  persist() {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        devices: this.devices,
        groups: this.groups,
        users: this.users,
        userGroups: this.userGroups,
        audit: this.audit,
        addressBooks: this.addressBooks,
        strategies: this.strategies,
        tfa: { enabled: this.tfa.enabled === true },
      })
    )
  }

  record(actor, action, target = '', detail = '') {
    const nextId = Math.max(0, ...this.audit.map((row) => row.id)) + 1
    this.audit.push({
      id: nextId,
      actor,
      action,
      target,
      detail,
      created_at: new Date().toISOString(),
    })
  }

  detachStrategyTarget(kind, target) {
    for (const strategy of this.strategies) {
      const list = strategy.assignments?.[kind]
      if (Array.isArray(list)) {
        strategy.assignments[kind] = list.filter((value) => value !== target)
      }
    }
  }

  async getAuditLogs({
    current = 1,
    pageSize = 50,
    actor = '',
    action = '',
    q = '',
  } = {}) {
    await wait(120)
    const actorTerm = String(actor || '').trim().toLowerCase()
    const term = String(q || '').trim().toLowerCase()
    const rows = this.audit
      .filter((row) => !actorTerm || row.actor.toLowerCase().includes(actorTerm))
      .filter((row) => !action || row.action === action)
      .filter(
        (row) =>
          !term ||
          `${row.target} ${row.detail}`.toLowerCase().includes(term)
      )
      .sort((a, b) => b.id - a.id)
    const start = (Math.max(1, current) - 1) * pageSize
    return {
      total: rows.length,
      data: clone(rows.slice(start, start + pageSize)),
    }
  }

  async getAuditActions() {
    await wait(60)
    return [...new Set(this.audit.map((row) => row.action))].sort()
  }

  async getDevices() {
    await wait()
    return clone(this.devices)
  }

  async getGroups() {
    await wait(100)
    return clone(this.groups)
  }

  async getUsers() {
    await wait(80)
    return clone(this.users)
  }

  async createUser(payload) {
    await wait()
    const name = String(payload.username || '').trim()
    if (this.users.some((user) => user.name.toLowerCase() === name.toLowerCase())) {
      throw new Error('用户名已存在。')
    }
    const user = {
      id: Math.max(0, ...this.users.map((item) => item.id)) + 1,
      name,
      display_name: String(payload.display_name || '').trim() || name,
      email: String(payload.email || '').trim(),
      note: String(payload.note || '').trim(),
      status: 1,
      is_admin: payload.is_admin === true,
      tfa_enabled: false,
    }
    this.users.push(user)
    this.userGroups[user.id] = []
    this.record('demo', 'user.create', user.name)
    this.persist()
    return clone(user)
  }

  async updateUser(id, payload) {
    await wait()
    const user = this.users.find((item) => item.id === Number(id))
    if (!user) throw new Error('未找到用户。')
    if (payload.display_name !== undefined) {
      user.display_name = String(payload.display_name).trim() || user.name
    }
    if (payload.email !== undefined) user.email = String(payload.email).trim()
    if (payload.note !== undefined) user.note = String(payload.note).trim()
    if (payload.status !== undefined) user.status = Number(payload.status)
    if (payload.is_admin !== undefined) user.is_admin = payload.is_admin === true
    this.record('demo', 'user.update', user.name)
    this.persist()
    return clone(user)
  }

  async deleteUser(id) {
    await wait()
    const user = this.users.find((item) => item.id === Number(id))
    if (!user) throw new Error('未找到用户。')
    this.users = this.users.filter((item) => item.id !== user.id)
    delete this.userGroups[user.id]
    for (const book of this.addressBooks) {
      book.shares = book.shares.filter((share) => share.user_id !== user.id)
    }
    this.detachStrategyTarget('users', user.name)
    for (const device of this.devices) {
      if (device.user_name === user.name) {
        device.user_name = ''
      }
    }
    this.record('demo', 'user.delete', user.name)
    this.persist()
    return { success: true }
  }

  async getUserGroups(id) {
    await wait(80)
    return clone(this.userGroups[Number(id)] || [])
  }

  async setUserGroups(id, groups) {
    await wait()
    this.userGroups[Number(id)] = clone(groups)
    const user = this.users.find((item) => item.id === Number(id))
    this.record('demo', 'user.groups', user?.name || String(id), groups.join(','))
    this.persist()
    return { success: true, groups: clone(groups) }
  }

  async getTfaStatus() {
    await wait(60)
    return { enabled: this.tfa.enabled === true }
  }

  async setupTfa() {
    await wait()
    if (this.tfa.enabled) throw new Error('双重认证已启用。')
    const secret = demoSecret()
    this.tfa.pendingSecret = secret
    this.persist()
    return {
      secret,
      issuer: 'RustDesk Server',
      account: 'demo',
      otpauth_url: `otpauth://totp/RustDesk%20Server:demo?secret=${secret}&issuer=RustDesk%20Server&algorithm=SHA1&digits=6&period=30`,
    }
  }

  async enableTfa(code) {
    await wait()
    if (this.tfa.enabled) throw new Error('双重认证已启用。')
    if (!/^\d{6}$/.test(String(code || '').trim())) {
      throw new Error('请输入 6 位数字验证码。演示模式可输入任意 6 位数字。')
    }
    this.tfa.enabled = true
    this.tfa.pendingSecret = ''
    this.record('demo', 'user.tfa_enable', 'demo')
    this.persist()
    return { success: true, enabled: true }
  }

  async disableTfa({ password, code } = {}) {
    await wait()
    if (!String(password || '').trim()) throw new Error('请输入当前密码。')
    if (!/^\d{6}$/.test(String(code || '').trim())) {
      throw new Error('请输入 6 位数字验证码。演示模式可输入任意 6 位数字。')
    }
    this.tfa.enabled = false
    this.tfa.pendingSecret = ''
    this.record('demo', 'user.tfa_disable', 'demo')
    this.persist()
    return { success: true, enabled: false }
  }

  async resetUserTfa(id) {
    await wait()
    const user = this.users.find((item) => item.id === Number(id))
    if (!user) throw new Error('未找到用户。')
    user.tfa_enabled = false
    this.record('demo', 'user.tfa_reset', user.name)
    this.persist()
    return { success: true }
  }

  findDevice(guid) {
    const device = this.devices.find((item) => item.guid === guid)
    if (!device) throw new Error('未找到设备。')
    return device
  }

  async setDeviceDisabled(guid, disabled) {
    await wait()
    const device = this.findDevice(guid)
    device.disabled = disabled
    if (disabled) device.status = 0
    this.record('demo', disabled ? 'device.disable' : 'device.enable', guid)
    this.persist()
    return { success: true }
  }

  async deleteDevice(guid) {
    await wait()
    const device = this.findDevice(guid)
    this.devices = this.devices.filter((item) => item.guid !== guid)
    this.detachStrategyTarget('devices', guid)
    this.record('demo', 'device.delete', guid, device.id || '')
    this.persist()
    return { success: true }
  }

  async createWebControlSession() {
    await wait()
    throw new Error('演示模式不连接真实服务器，无法发起网页远控。')
  }

  async assignDevice(guid, type, value) {
    await wait()
    const device = this.findDevice(guid)
    switch (type) {
      case 'device_name':
        device.device_name = value
        if (device.info) device.info.device_name = value
        break
      case 'note':
        device.note = value
        break
      case 'device_group_name':
        device.device_group_name = value
        break
      case 'user_name':
        device.user_name = value
        break
      case 'device_username':
        if (device.info) device.info.username = value
        break
      default:
        break
    }
    this.record('demo', 'device.assign', guid, type)
    this.persist()
    return { success: true }
  }

  async createGroup({ name, note }) {
    await wait()
    const exists = this.groups.some(
      (group) => group.name.toLowerCase() === name.toLowerCase()
    )
    if (exists) throw new Error('同名分组已存在。')
    const group = {
      guid: `grp-${Date.now().toString(36)}`,
      name,
      note: note || '',
    }
    this.groups.push(group)
    this.record('demo', 'group.create', group.guid, group.name)
    this.persist()
    return clone(group)
  }

  async updateGroup(guid, payload) {
    await wait()
    const group = this.groups.find((item) => item.guid === guid)
    if (!group) throw new Error('未找到分组。')
    if (payload.name && payload.name !== group.name) {
      const exists = this.groups.some(
        (item) =>
          item.guid !== guid &&
          item.name.toLowerCase() === payload.name.toLowerCase()
      )
      if (exists) throw new Error('同名分组已存在。')
      for (const device of this.devices) {
        if (device.device_group_name === group.name) {
          device.device_group_name = payload.name
        }
      }
      group.name = payload.name
    }
    if (payload.note !== undefined) group.note = payload.note
    this.record('demo', 'group.update', guid, group.name)
    this.persist()
    return { success: true }
  }

  async deleteGroup(guid) {
    await wait()
    const group = this.groups.find((item) => item.guid === guid)
    if (!group) throw new Error('未找到分组。')
    this.groups = this.groups.filter((item) => item.guid !== guid)
    this.detachStrategyTarget('groups', guid)
    for (const device of this.devices) {
      if (device.device_group_name === group.name) {
        device.device_group_name = ''
      }
    }
    this.record('demo', 'group.delete', guid)
    this.persist()
    return { success: true }
  }

  findBook(guid) {
    const book = this.addressBooks.find((item) => item.guid === guid)
    if (!book) throw new Error('未找到地址簿。')
    return book
  }

  bookSummary(book) {
    return {
      ...clone(book),
      peer_count: book.peers.length,
      share_count: book.shares.length,
    }
  }

  async getAddressBooks({ q = '' } = {}) {
    await wait(120)
    const term = String(q || '').trim().toLowerCase()
    return this.addressBooks
      .filter(
        (book) =>
          !term ||
          `${book.name} ${book.owner} ${book.note}`.toLowerCase().includes(term)
      )
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((book) => this.bookSummary(book))
  }

  async createAddressBook(payload) {
    await wait()
    const name = String(payload.name || '').trim()
    if (!name) throw new Error('请填写地址簿名称。')
    const exists = this.addressBooks.some(
      (book) => book.name.toLowerCase() === name.toLowerCase()
    )
    if (exists) throw new Error('同名地址簿已存在。')
    const password = String(payload.password || '').trim()
    const now = new Date().toISOString()
    const book = {
      guid: `ab-${Date.now().toString(36)}`,
      name,
      owner: 'demo',
      note: String(payload.note || '').trim(),
      info: password ? { password } : {},
      created_at: now,
      updated_at: now,
      tags: [],
      shares: [],
      peers: [],
    }
    this.addressBooks.push(book)
    this.record('demo', 'address_book.create', book.guid, book.name)
    this.persist()
    return this.bookSummary(book)
  }

  async updateAddressBook(guid, payload) {
    await wait()
    const book = this.findBook(guid)
    if (payload.name !== undefined) {
      const name = String(payload.name).trim()
      if (!name) throw new Error('请填写地址簿名称。')
      const exists = this.addressBooks.some(
        (item) =>
          item.guid !== guid &&
          item.name.toLowerCase() === name.toLowerCase()
      )
      if (exists) throw new Error('同名地址簿已存在。')
      book.name = name
    }
    if (payload.note !== undefined) book.note = String(payload.note).trim()
    if (payload.password !== undefined) {
      const password = String(payload.password).trim()
      book.info = password ? { ...book.info, password } : {}
    }
    book.updated_at = new Date().toISOString()
    this.record('demo', 'address_book.update', guid, book.name)
    this.persist()
    return { success: true }
  }

  async deleteAddressBook(guid) {
    await wait()
    const book = this.findBook(guid)
    this.addressBooks = this.addressBooks.filter((item) => item.guid !== guid)
    this.record('demo', 'address_book.delete', guid, book.name)
    this.persist()
    return { success: true }
  }

  async getAddressBookShares(guid) {
    await wait(80)
    const book = this.findBook(guid)
    return book.shares
      .map((share) => {
        const user = this.users.find((item) => item.id === share.user_id)
        return {
          user_id: share.user_id,
          rule: share.rule,
          name: user?.name || '',
          display_name: user?.display_name || user?.name || '',
        }
      })
      .sort((a, b) => a.name.localeCompare(b.name))
  }

  async setAddressBookShares(guid, shares) {
    await wait()
    const book = this.findBook(guid)
    const next = []
    for (const share of shares) {
      const userId = Number(share.userId ?? share.user_id)
      const rule = Number(share.rule)
      if (!(rule >= 1 && rule <= 3)) throw new Error('共享权限无效。')
      if (!this.users.some((user) => user.id === userId)) {
        throw new Error('用户不存在。')
      }
      next.push({ user_id: userId, rule })
    }
    book.shares = next
    book.updated_at = new Date().toISOString()
    this.record('demo', 'address_book.share', guid, String(next.length))
    this.persist()
    return { success: true, shares: next.length }
  }

  async getAddressBookPeers(guid) {
    await wait(100)
    return clone(this.findBook(guid).peers)
  }

  async getAddressBookTags(guid) {
    await wait(60)
    return clone(this.findBook(guid).tags)
  }

  async addAddressBookTag(guid, { name, color }) {
    await wait(80)
    const book = this.findBook(guid)
    const tagName = String(name || '').trim()
    if (!tagName) throw new Error('请填写标签名称。')
    if (book.tags.some((tag) => tag.name === tagName)) {
      throw new Error('同名标签已存在。')
    }
    const tag = { name: tagName, color: Number(color || 4280391411) }
    book.tags.push(tag)
    this.persist()
    return clone(tag)
  }

  async addAddressBookPeer(guid, payload) {
    await wait()
    const book = this.findBook(guid)
    const peerId = String(payload.id || '').trim()
    if (!peerId) throw new Error('请填写设备 ID。')
    if (book.peers.some((peer) => peer.id === peerId)) {
      throw new Error('该设备已在地址簿中。')
    }
    const tagNames = new Set(book.tags.map((tag) => tag.name))
    const tags = Array.isArray(payload.tags)
      ? payload.tags.filter((tag) => tagNames.has(tag))
      : []
    book.peers.push({
      id: peerId,
      hash: '',
      password: String(payload.password || ''),
      username: String(payload.username || ''),
      hostname: String(payload.hostname || ''),
      platform: String(payload.platform || ''),
      alias: String(payload.alias || peerId),
      tags,
      forceAlwaysRelay: payload.forceAlwaysRelay === true ? 'true' : 'false',
      rdpPort: String(payload.rdpPort || ''),
      rdpUsername: String(payload.rdpUsername || ''),
      loginName: String(payload.loginName || ''),
      device_group_name: String(payload.device_group_name || ''),
      note: String(payload.note || ''),
      same_server: this.devices.some((device) => device.id === peerId),
    })
    book.updated_at = new Date().toISOString()
    this.persist()
    return { success: true }
  }

  async deleteAddressBookPeer(guid, ids) {
    await wait()
    const book = this.findBook(guid)
    const remove = new Set(ids.map((id) => String(id)))
    book.peers = book.peers.filter((peer) => !remove.has(peer.id))
    book.updated_at = new Date().toISOString()
    this.persist()
    return { success: true }
  }

  findStrategy(guid) {
    const strategy = this.strategies.find((item) => item.guid === guid)
    if (!strategy) throw new Error('未找到策略。')
    return strategy
  }

  strategyAssignments(strategy) {
    const assigned = strategy.assignments || {}
    return {
      devices: (assigned.devices || [])
        .map((guid) => this.devices.find((device) => device.guid === guid))
        .filter(Boolean)
        .map((device) => ({
          guid: device.guid,
          id: device.id,
          device_name: device.device_name,
        })),
      users: (assigned.users || [])
        .map((name) => this.users.find((user) => user.name === name))
        .filter(Boolean)
        .map((user) => ({
          name: user.name,
          display_name: user.display_name,
        })),
      groups: (assigned.groups || [])
        .map((guid) => this.groups.find((group) => group.guid === guid))
        .filter(Boolean)
        .map((group) => ({ guid: group.guid, name: group.name })),
    }
  }

  strategySummary(strategy) {
    const { assignments, ...rest } = strategy
    const resolved = this.strategyAssignments(strategy)
    return {
      ...clone(rest),
      device_count: resolved.devices.length,
      user_count: resolved.users.length,
      group_count: resolved.groups.length,
    }
  }

  async getStrategies() {
    await wait(120)
    return this.strategies
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'))
      .map((strategy) => this.strategySummary(strategy))
  }

  async createStrategy(payload) {
    await wait(150)
    const name = String(payload.name || '').trim()
    if (!name) throw new Error('策略名称不能为空。')
    const exists = this.strategies.some(
      (strategy) => strategy.name.toLowerCase() === name.toLowerCase()
    )
    if (exists) throw new Error('同名策略已存在。')
    const now = new Date()
    const strategy = {
      guid: `st-${Date.now().toString(36)}`,
      name,
      note: String(payload.note || ''),
      enabled: payload.enabled !== false,
      config_options: sanitizeConfig(payload.config_options),
      assignments: { devices: [], users: [], groups: [] },
      created_at: now.toISOString(),
      updated_at: now.toISOString(),
      modified_at: Math.floor(now.getTime() / 1000),
    }
    this.strategies.push(strategy)
    this.record('demo', 'strategy.create', strategy.guid, name)
    this.persist()
    return this.strategySummary(strategy)
  }

  async updateStrategy(guid, payload) {
    await wait(150)
    const strategy = this.findStrategy(guid)
    if (payload.name !== undefined) {
      const name = String(payload.name).trim()
      if (!name) throw new Error('策略名称不能为空。')
      const exists = this.strategies.some(
        (item) =>
          item.guid !== guid && item.name.toLowerCase() === name.toLowerCase()
      )
      if (exists) throw new Error('同名策略已存在。')
      strategy.name = name
    }
    if (payload.note !== undefined) strategy.note = String(payload.note)
    if (payload.enabled !== undefined) strategy.enabled = Boolean(payload.enabled)
    if (payload.config_options !== undefined) {
      strategy.config_options = sanitizeConfig(payload.config_options)
    }
    const previous = strategy.modified_at || 0
    strategy.modified_at = Math.max(Math.floor(Date.now() / 1000), previous + 1)
    strategy.updated_at = new Date().toISOString()
    this.record('demo', 'strategy.update', guid, strategy.name)
    this.persist()
    return this.strategySummary(strategy)
  }

  async setStrategyStatus(guid, enabled) {
    await wait(120)
    const strategy = this.findStrategy(guid)
    strategy.enabled = Boolean(enabled)
    const previous = strategy.modified_at || 0
    strategy.modified_at = Math.max(Math.floor(Date.now() / 1000), previous + 1)
    strategy.updated_at = new Date().toISOString()
    this.record(
      'demo',
      'strategy.status',
      guid,
      strategy.enabled ? 'enabled' : 'disabled'
    )
    this.persist()
    return { success: true, enabled: strategy.enabled }
  }

  async deleteStrategy(guid) {
    await wait()
    const strategy = this.findStrategy(guid)
    this.strategies = this.strategies.filter((item) => item.guid !== guid)
    this.record('demo', 'strategy.delete', guid, strategy.name)
    this.persist()
    return { success: true }
  }

  async getStrategyAssignments(guid) {
    await wait(100)
    return clone(this.strategyAssignments(this.findStrategy(guid)))
  }

  async assignStrategy({ strategy = null, peers = [], users = [], groups = [] }) {
    await wait(140)
    const target = strategy ? this.findStrategy(strategy) : null
    const deviceKeys = []
    for (const raw of peers) {
      const key = String(raw || '').trim()
      const device = this.devices.find(
        (item) => item.guid === key || item.id === key
      )
      if (!device) throw new Error(`设备“${key}”不存在。`)
      if (!deviceKeys.includes(device.guid)) deviceKeys.push(device.guid)
    }
    const userKeys = []
    for (const raw of users) {
      const key = String(raw || '').trim()
      const user = this.users.find(
        (item) => item.name === key || String(item.id) === key
      )
      if (!user) throw new Error(`用户“${key}”不存在。`)
      if (!userKeys.includes(user.name)) userKeys.push(user.name)
    }
    const groupKeys = []
    for (const raw of groups) {
      const key = String(raw || '').trim()
      const group = this.groups.find(
        (item) =>
          item.guid === key || item.name.toLowerCase() === key.toLowerCase()
      )
      if (!group) throw new Error(`设备分组“${key}”不存在。`)
      if (!groupKeys.includes(group.guid)) groupKeys.push(group.guid)
    }

    const attach = (kind, keys) => {
      for (const key of keys) {
        this.detachStrategyTarget(kind, key)
        if (target) {
          target.assignments[kind] ||= []
          if (!target.assignments[kind].includes(key)) {
            target.assignments[kind].push(key)
          }
        }
      }
    }
    attach('devices', deviceKeys)
    attach('users', userKeys)
    attach('groups', groupKeys)

    if (target) {
      target.updated_at = new Date().toISOString()
      const previous = target.modified_at || 0
      target.modified_at = Math.max(Math.floor(Date.now() / 1000), previous + 1)
    }
    this.record(
      'demo',
      'strategy.assign',
      target?.guid || '',
      `peers=${deviceKeys.length},users=${userKeys.length},groups=${groupKeys.length}`
    )
    this.persist()
    return {
      success: true,
      assigned: {
        peers: deviceKeys.length,
        users: userKeys.length,
        groups: groupKeys.length,
      },
    }
  }

  async testConnection() {
    await wait(220)
    return true
  }

  async reset() {
    await wait(120)
    this.devices = seedDevices()
    this.groups = clone(GROUPS)
    this.users = clone(USERS)
    this.userGroups = clone(USER_GROUPS)
    this.audit = seedAudit()
    this.addressBooks = seedAddressBooks()
    this.strategies = seedStrategies()
    this.tfa = { enabled: false, pendingSecret: '' }
    this.persist()
    return { success: true }
  }
}
