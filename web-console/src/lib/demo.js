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
  { name: 'maya', display_name: '陈雨桐', email: 'maya@acme.com', status: 1, is_admin: true },
  { name: 'alex', display_name: '李昂', email: 'alex@acme.com', status: 1, is_admin: false },
  { name: 'priya', display_name: '王思远', email: 'priya@acme.com', status: 1, is_admin: false },
  { name: 'noah', display_name: '周诺', email: 'noah@acme.com', status: 1, is_admin: false },
  { name: 'lena', display_name: '赵琳', email: 'lena@acme.com', status: 1, is_admin: false },
]

function isoAgo(minutes) {
  return new Date(Date.now() - minutes * 60 * 1000).toISOString()
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

export class DemoApi {
  constructor() {
    const stored = readStored()
    if (stored) {
      this.devices = stored.devices
      this.groups = stored.groups
    } else {
      this.devices = seedDevices()
      this.groups = clone(GROUPS)
      this.persist()
    }
  }

  persist() {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ devices: this.devices, groups: this.groups })
    )
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
    return clone(USERS)
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
    this.persist()
    return { success: true }
  }

  async deleteDevice(guid) {
    await wait()
    this.devices = this.devices.filter((item) => item.guid !== guid)
    this.persist()
    return { success: true }
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
    this.persist()
    return { success: true }
  }

  async deleteGroup(guid) {
    await wait()
    const group = this.groups.find((item) => item.guid === guid)
    if (!group) throw new Error('未找到分组。')
    this.groups = this.groups.filter((item) => item.guid !== guid)
    for (const device of this.devices) {
      if (device.device_group_name === group.name) {
        device.device_group_name = ''
      }
    }
    this.persist()
    return { success: true }
  }

  async testConnection() {
    await wait(220)
    return true
  }

  async reset() {
    await wait(120)
    this.devices = seedDevices()
    this.groups = clone(GROUPS)
    this.persist()
    return { success: true }
  }
}
