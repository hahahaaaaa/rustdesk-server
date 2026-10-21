import { decodeMessage, decodeRendezvous, encodeMessage, encodeRendezvous } from './proto'
import { decryptMessage, encryptMessage } from './secure'

export class WireTimeoutError extends Error {
  constructor() {
    super('等待服务器响应超时。')
    this.name = 'WireTimeoutError'
  }
}

export class Wire {
  constructor(url) {
    this.url = url
    this.queue = []
    this.waiters = []
    this.closed = false
    this.error = ''
    this.secretKey = null
    this.sendCount = 0
    this.recvCount = 0
    this.rendezvous = false
    this.ws = new WebSocket(url)
    this.ws.binaryType = 'arraybuffer'
    this.ws.onmessage = (event) => this.handleMessage(event)
    this.ws.onclose = () => this.handleClose()
    this.ws.onerror = () => {}
  }

  open(timeout = 15000) {
    return new Promise((resolve, reject) => {
      if (this.ws.readyState === WebSocket.OPEN) {
        resolve(this)
        return
      }
      const timer = setTimeout(() => {
        this.fail('连接服务器超时。')
        reject(new Error('连接服务器超时。'))
      }, timeout)
      this.ws.onopen = () => {
        clearTimeout(timer)
        resolve(this)
      }
      this.ws.onclose = () => {
        clearTimeout(timer)
        this.handleClose()
        if (this.closed) reject(new Error(this.error || '连接已关闭。'))
      }
    })
  }

  useRendezvous() {
    this.rendezvous = true
  }

  setSecretKey(key) {
    this.secretKey = key
    this.sendCount = 0
    this.recvCount = 0
  }

  sendRendezvous(payload) {
    this.sendRaw(encodeRendezvous(payload))
  }

  sendMessage(payload) {
    this.sendRaw(encodeMessage(payload))
  }

  sendRaw(bytes) {
    if (this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('连接尚未建立。')
    }
    let data = bytes
    if (this.secretKey) {
      this.sendCount += 1
      data = encryptMessage(bytes, this.sendCount, this.secretKey)
    }
    this.ws.send(data)
  }

  next(timeout = 20000) {
    if (this.queue.length) {
      return Promise.resolve(this.queue.shift())
    }
    if (this.closed) {
      return Promise.reject(new Error(this.error || '连接已关闭。'))
    }
    return new Promise((resolve, reject) => {
      const waiter = { resolve, reject, timer: null }
      if (timeout > 0) {
        waiter.timer = setTimeout(() => {
          const index = this.waiters.indexOf(waiter)
          if (index >= 0) this.waiters.splice(index, 1)
          reject(new WireTimeoutError())
        }, timeout)
      }
      this.waiters.push(waiter)
    })
  }

  handleMessage(event) {
    if (!(event.data instanceof ArrayBuffer)) return
    let bytes = new Uint8Array(event.data)
    if (this.secretKey) {
      try {
        this.recvCount += 1
        bytes = decryptMessage(bytes, this.recvCount, this.secretKey)
      } catch {
        this.fail('安全通道解密失败。')
        return
      }
    }
    let message
    try {
      message = this.rendezvous
        ? decodeRendezvous(bytes)
        : decodeMessage(bytes)
    } catch {
      this.fail('收到无法解析的服务器消息。')
      return
    }
    const waiter = this.waiters.shift()
    if (waiter) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.resolve(message)
    } else {
      this.queue.push(message)
    }
  }

  handleClose() {
    if (this.closed) return
    this.closed = true
    const reason = this.error || '连接已被服务器关闭。'
    this.error = reason
    for (const waiter of this.waiters.splice(0)) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.reject(new Error(reason))
    }
  }

  fail(message) {
    this.error = message
    this.handleClose()
    try {
      this.ws.close()
    } catch {
      // Ignore close failures.
    }
  }

  close() {
    this.closed = true
    for (const waiter of this.waiters.splice(0)) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.reject(new Error('连接已关闭。'))
    }
    try {
      this.ws.close()
    } catch {
      // Ignore close failures.
    }
  }
}
