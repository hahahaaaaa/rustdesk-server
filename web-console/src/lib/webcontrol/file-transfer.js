import { decompress } from 'fzstd'
import { WebControlClient } from './client'
import { initSodium } from './secure'

// Matches the native client's block size. Uploads are sent uncompressed.
const BLOCK_SIZE = 128 * 1024
// Pause uploading while this much data is still queued in the WebSocket.
const MAX_BUFFERED = 4 * 1024 * 1024
const JOB_NOTIFY_INTERVAL = 200

export const FILE_TYPE = {
  Dir: 0,
  DirLink: 2,
  DirDrive: 3,
  File: 4,
  FileLink: 5,
}

const FINAL_STATES = ['done', 'error', 'canceled', 'skipped']

const FILE_ERRORS = {
  'one-way-file-transfer-tip': '对方只允许单向文件传输，不能向其写入文件。',
  'Permission denied': '没有权限访问该路径。',
  skipped: '已跳过。',
}

function has(object, field) {
  return Boolean(object) && Object.prototype.hasOwnProperty.call(object, field)
}

function toNumber(value) {
  if (value === null || value === undefined) return 0
  if (typeof value === 'number') return value
  if (typeof value === 'object' && typeof value.toNumber === 'function') return value.toNumber()
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function toBytes(value) {
  if (!value) return new Uint8Array(0)
  if (value instanceof Uint8Array) return value
  return new Uint8Array(value)
}

function fileError(text) {
  const value = String(text || '').trim()
  if (!value) return '操作失败。'
  return FILE_ERRORS[value] || value
}

function abortWritable(writable) {
  if (!writable) return
  try {
    writable.abort()
  } catch {
    // The stream may already be closed.
  }
}

export class FileTransferClient extends WebControlClient {
  constructor({ session, handlers = {}, password = '' }) {
    super({ session, handlers })
    this.connType = 1
    this.autoPassword = String(password || '')
    this.autoTried = false
    this.showHidden = false
    this.currentPath = ''
    this.jobs = new Map()
    this.nextJobId = 1
    this.jobsTimer = 0
  }

  get isWindows() {
    return this.peerPlatform === 'Windows'
  }

  get separator() {
    return this.isWindows ? '\\' : '/'
  }

  async start() {
    if (this.started) return
    this.started = true
    if (!this.deviceId) {
      this.fail('缺少设备 ID，无法发起文件传输。')
      return
    }
    try {
      this.setPhase('preparing', '正在准备文件传输…')
      await initSodium()
      if (!window.isSecureContext) {
        throw new Error('文件传输需要 HTTPS 安全上下文（或通过 localhost 访问）。')
      }
      const relayResponse = await this.connectRendezvous()
      await this.connectRelay(relayResponse)
      this.setPhase('auth')
      this.loop()
    } catch (error) {
      this.fail(error?.message || '文件传输初始化失败。')
    }
  }

  handleMessage(message) {
    this.lastPeerAt = Date.now()
    if (message.hash) {
      this.hash = {
        salt: message.hash.salt || '',
        challenge: message.hash.challenge || '',
      }
      // Reuse the password from the remote-control session once; ask the user if it fails.
      if (!this.autoTried && this.autoPassword) {
        this.autoTried = true
        this.setPhase('auth', '正在验证设备身份…')
        this.submitPassword(this.autoPassword)
        return
      }
      this.setPhase('auth', '请输入设备连接密码。')
      this.emit('onPasswordRequired', { wrong: false })
      return
    }
    if (message.login_response) {
      this.handleLoginResponse(message.login_response)
      return
    }
    if (message.file_response) {
      this.handleFileResponse(message.file_response)
      return
    }
    if (message.file_action) {
      this.handleFileAction(message.file_action)
      return
    }
    if (message.test_delay) {
      if (!message.test_delay.from_client) {
        this.send({ test_delay: message.test_delay })
      }
      return
    }
    if (message.misc && has(message.misc, 'close_reason')) {
      this.emit('onPeerClosed', {
        reason: String(message.misc.close_reason) || '对方已结束文件传输会话。',
      })
      this.close()
      return
    }
    if (message.message_box) {
      const box = message.message_box
      this.emit('onMessageBox', {
        title: box.title || '对方消息',
        text: box.text || '',
        type: box.msgtype || 'info',
      })
    }
  }

  // File-transfer sessions carry no video, so the remote-control watchdog is not started.
  handlePeerInfo(info) {
    this.peer = info
    this.loggedIn = true
    this.peerPlatform = info.platform || ''
    const sessions = info.windows_sessions
    if (sessions && Array.isArray(sessions.sessions) && sessions.sessions.length > 1) {
      // The peer waits for a session choice before listing files; keep the current one.
      this.send({ misc: { selected_sid: Number(sessions.current_sid || 0) } })
    }
    this.setPhase('connected', '已连接')
    this.emit('onPeer', { platform: this.peerPlatform, separator: this.separator })
  }

  sendLogin(password) {
    this.send({
      login_request: {
        username: this.deviceId,
        password: password || new Uint8Array(0),
        my_id: 'web-console',
        my_name: '网页控制台',
        my_platform: 'Web',
        file_transfer: { dir: '', show_hidden: this.showHidden },
        version: '1.5.1',
      },
    })
  }

  readDir(path) {
    if (!this.loggedIn || this.closed) return
    this.send({
      file_action: {
        read_dir: { path: String(path ?? ''), include_hidden: this.showHidden },
      },
    })
  }

  setShowHidden(value) {
    this.showHidden = Boolean(value)
  }

  handleFileResponse(response) {
    if (response.dir) {
      const dir = response.dir
      const job = this.jobs.get(Number(dir.id || 0))
      if (job && job.kind === 'download') {
        const entries = dir.entries || []
        if (entries.length > 1) {
          this.cancelJob(job.id, 'error', '暂不支持下载文件夹，请逐个下载文件。')
          return
        }
        if (entries[0]) job.total = toNumber(entries[0].size)
        this.notifyJobs()
        return
      }
      this.currentPath = String(dir.path || '')
      this.emit('onDirectory', {
        path: this.currentPath,
        entries: (dir.entries || []).map((entry) => ({
          name: String(entry.name || ''),
          type: Number(entry.entry_type || 0),
          hidden: entry.is_hidden === true,
          size: toNumber(entry.size),
          modified: toNumber(entry.modified_time),
        })),
      })
      return
    }
    if (response.digest) {
      this.handleDigest(response.digest)
      return
    }
    if (response.block) {
      this.handleBlock(response.block)
      return
    }
    if (response.done) {
      const job = this.jobs.get(Number(response.done.id || 0))
      if (!job) return
      if (job.kind === 'download') this.finishDownload(job)
      else this.finishJob(job, 'done', '')
      return
    }
    if (response.error) {
      const id = Number(response.error.id || 0)
      const text = fileError(response.error.error)
      const job = this.jobs.get(id)
      if (job && !FINAL_STATES.includes(job.state)) {
        this.finishJob(job, text === FILE_ERRORS.skipped ? 'skipped' : 'error', text)
        return
      }
      this.emit('onDirectoryError', text)
    }
  }

  handleFileAction(action) {
    const confirm = action.send_confirm
    if (!confirm) return
    const job = this.jobs.get(Number(confirm.id || 0))
    if (!job || job.kind !== 'upload' || job.state !== 'waiting') return
    if (has(confirm, 'skip') && confirm.skip) {
      // The peer already has an identical file.
      this.cancelJob(job.id, 'skipped', '对方已有相同文件，已跳过。')
      return
    }
    this.sendBlocks(job)
  }

  handleDigest(digest) {
    const job = this.jobs.get(Number(digest.id || 0))
    if (!job || FINAL_STATES.includes(job.state)) return
    if (job.kind === 'upload') {
      // The target file already exists; let the user decide.
      job.state = 'confirm'
      job.identical = digest.is_identical === true
      job.message = job.identical ? '对方已存在相同的文件。' : '对方已存在同名文件。'
      this.notifyJobs()
      return
    }
    job.total = toNumber(digest.file_size) || job.total
    job.state = 'running'
    this.send({
      file_action: {
        send_confirm: {
          id: job.id,
          file_num: Number(digest.file_num || 0),
          offset_blk: 0,
        },
      },
    })
    this.notifyJobs()
  }

  handleBlock(block) {
    const job = this.jobs.get(Number(block.id || 0))
    if (!job || job.kind !== 'download' || FINAL_STATES.includes(job.state)) return
    job.state = 'running'
    let data = toBytes(block.data)
    if (block.compressed) {
      try {
        data = decompress(data)
      } catch {
        this.cancelJob(job.id, 'error', '数据解压失败。')
        return
      }
    }
    // Copy, because the decoded buffer may be a view into a reused message buffer.
    const chunk = data.slice()
    job.transferred += chunk.length
    if (job.writable) {
      job.writeChain = job.writeChain
        .then(() => job.writable.write(chunk))
        .catch((error) => {
          job.writeError = job.writeError || error
        })
    } else {
      job.chunks.push(chunk)
    }
    this.notifyJobs()
  }

  uploadFile(file, dir) {
    if (!this.loggedIn || this.closed) return null
    const target = String(dir ?? '')
    if (!target || (target === '/' && this.isWindows)) {
      this.emit('onNotice', '请先进入一个具体的文件夹再上传。')
      return null
    }
    const id = this.nextJobId++
    const path = joinPath(target, file.name, this.separator)
    const modified = Math.floor((file.lastModified || Date.now()) / 1000)
    const job = {
      id,
      kind: 'upload',
      name: file.name,
      path,
      dir: target,
      total: file.size,
      transferred: 0,
      state: 'waiting',
      message: '',
      identical: false,
      file,
      modified,
    }
    this.jobs.set(id, job)
    this.send({
      file_action: {
        receive: {
          id,
          path,
          files: [{ entry_type: FILE_TYPE.File, name: '', size: file.size, modified_time: modified }],
          file_num: 0,
          total_size: file.size,
        },
      },
    })
    // Ask the peer whether the file already exists before sending data.
    this.send({
      file_response: {
        digest: {
          id,
          file_num: 0,
          last_modified: modified,
          file_size: file.size,
          is_upload: true,
          is_resume: false,
        },
      },
    })
    this.notifyJobs()
    return id
  }

  confirmOverwrite(id, overwrite) {
    const job = this.jobs.get(id)
    if (!job || job.state !== 'confirm') return
    if (overwrite) this.sendBlocks(job)
    else this.cancelJob(id, 'skipped', '已跳过。')
  }

  async sendBlocks(job) {
    if (job.state === 'running') return
    job.state = 'running'
    job.message = ''
    this.notifyJobs()
    const file = job.file
    let offset = 0
    try {
      do {
        if (job.state !== 'running' || this.closed) return
        while (this.wire?.ws && this.wire.ws.bufferedAmount > MAX_BUFFERED) {
          await new Promise((resolve) => setTimeout(resolve, 30))
          if (job.state !== 'running' || this.closed) return
        }
        const end = Math.min(offset + BLOCK_SIZE, file.size)
        const data = new Uint8Array(await file.slice(offset, end).arrayBuffer())
        if (job.state !== 'running' || this.closed) return
        // An empty file still needs one block so the peer creates it.
        this.send({
          file_response: {
            block: { id: job.id, file_num: 0, data, compressed: false },
          },
        })
        offset = end
        job.transferred = offset
        this.notifyJobs()
      } while (offset < file.size)
      job.state = 'finishing'
      this.send({ file_response: { done: { id: job.id, file_num: 0 } } })
      this.notifyJobs()
    } catch (error) {
      this.cancelJob(job.id, 'error', error?.message || '读取本地文件失败。')
    }
  }

  async downloadFile(entry) {
    if (!this.loggedIn || this.closed) return null
    const name = String(entry?.name || '')
    if (!name) return null
    let writable = null
    if (typeof window.showSaveFilePicker === 'function') {
      try {
        const handle = await window.showSaveFilePicker({ suggestedName: name })
        writable = await handle.createWritable()
      } catch (error) {
        if (error?.name === 'AbortError') return null
        writable = null
      }
    }
    if (this.closed) {
      abortWritable(writable)
      return null
    }
    const id = this.nextJobId++
    const path = joinPath(this.currentPath, name, this.separator)
    const job = {
      id,
      kind: 'download',
      name,
      path,
      dir: this.currentPath,
      total: Number(entry.size || 0),
      transferred: 0,
      state: 'waiting',
      message: '',
      writable,
      writeChain: Promise.resolve(),
      writeError: null,
      chunks: [],
    }
    this.jobs.set(id, job)
    this.send({
      file_action: {
        send: { id, path, include_hidden: false, file_num: 0, file_type: 0 },
      },
    })
    this.notifyJobs()
    return id
  }

  async finishDownload(job) {
    if (FINAL_STATES.includes(job.state)) return
    job.state = 'finishing'
    this.notifyJobs()
    try {
      if (job.writable) {
        await job.writeChain
        if (job.writeError) throw job.writeError
        await job.writable.close()
        job.writable = null
      } else {
        const blob = new Blob(job.chunks)
        const url = URL.createObjectURL(blob)
        const link = document.createElement('a')
        link.href = url
        link.download = job.name
        document.body.appendChild(link)
        link.click()
        link.remove()
        setTimeout(() => URL.revokeObjectURL(url), 60000)
      }
      this.finishJob(job, 'done', '')
    } catch (error) {
      this.finishJob(job, 'error', error?.message || '保存文件失败。')
    }
  }

  cancelJob(id, state = 'canceled', message = '已取消。') {
    const job = this.jobs.get(id)
    if (!job || FINAL_STATES.includes(job.state)) return
    if (this.wire && !this.closed) {
      this.send({ file_action: { cancel: { id } } })
    }
    this.finishJob(job, state, message)
  }

  finishJob(job, state, message) {
    job.state = state
    job.message = message
    job.file = null
    job.chunks = []
    if (state !== 'done') abortWritable(job.writable)
    job.writable = null
    this.notifyJobs(true)
    if (state === 'done' && job.kind === 'upload') {
      this.emit('onUploadDone', { dir: job.dir })
    }
  }

  clearFinished() {
    for (const [id, job] of this.jobs) {
      if (FINAL_STATES.includes(job.state)) this.jobs.delete(id)
    }
    this.notifyJobs(true)
  }

  notifyJobs(immediate = false) {
    const emit = () => {
      this.jobsTimer = 0
      this.emit(
        'onJobs',
        [...this.jobs.values()].map((job) => ({
          id: job.id,
          kind: job.kind,
          name: job.name,
          total: job.total,
          transferred: job.transferred,
          state: job.state,
          message: job.message,
          identical: job.identical === true,
        }))
      )
    }
    if (immediate) {
      if (this.jobsTimer) clearTimeout(this.jobsTimer)
      emit()
      return
    }
    if (!this.jobsTimer) this.jobsTimer = setTimeout(emit, JOB_NOTIFY_INTERVAL)
  }

  teardown() {
    for (const job of this.jobs.values()) {
      if (!FINAL_STATES.includes(job.state)) {
        job.state = 'error'
        job.message = '连接已断开。'
        job.file = null
        job.chunks = []
        abortWritable(job.writable)
        job.writable = null
      }
    }
    if (this.jobsTimer) {
      clearTimeout(this.jobsTimer)
      this.jobsTimer = 0
    }
    if (this.jobs.size) this.notifyJobs(true)
    super.teardown()
  }
}

export function joinPath(dir, name, separator) {
  const base = String(dir || '')
  if (!base) return name
  if (separator === '\\' && /^[A-Za-z]:$/.test(base)) return `${base}\\${name}`
  if (base.endsWith('/') || base.endsWith('\\')) return `${base}${name}`
  return `${base}${separator}${name}`
}

// Returns the parent directory, or '/' for the Windows drive list.
export function parentPath(path, separator) {
  const value = String(path || '')
  if (separator === '\\') {
    const trimmed = value.replace(/[\\/]+$/, '')
    if (!trimmed || trimmed === '/' || /^[A-Za-z]:$/.test(trimmed)) return '/'
    const index = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'))
    if (index < 0) return '/'
    const parent = trimmed.slice(0, index)
    return /^[A-Za-z]:$/.test(parent) ? `${parent}\\` : parent || '/'
  }
  const trimmed = value.replace(/\/+$/, '')
  if (!trimmed) return '/'
  const index = trimmed.lastIndexOf('/')
  if (index <= 0) return '/'
  return trimmed.slice(0, index)
}
