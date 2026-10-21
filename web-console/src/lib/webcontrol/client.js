import { decompress } from 'fzstd'
import { checkVp9Support, Vp9Decoder } from './codec'
import {
  CONTROL_KEY,
  MOUSE_TYPE_DOWN,
  MOUSE_TYPE_MOVE,
  MOUSE_TYPE_UP,
  MOUSE_TYPE_WHEEL,
  buttonMask,
  isModifierCode,
  keyDescriptor,
  modifierList,
} from './keys'
import { IdPk } from './proto'
import { createKeyExchange, decodeServerKey, initSodium, verifySigned } from './secure'
import { Wire, WireTimeoutError } from './wire'

const CONNECT_TIMEOUT = 15000
const RENDEZVOUS_TIMEOUT = 30000
const SECURE_TIMEOUT = 20000

const PUNCH_FAILURES = {
  0: '设备 ID 不存在，请确认设备已在服务器注册。',
  2: '对方设备离线。',
  3: '服务器 Key 不匹配，请检查控制台与 hbbs/hbbr 的 Key 配置。',
  4: '服务器 Key 授权数已超限。',
}

const LOGIN_ERRORS = {
  'Wrong Password': '密码错误，请重新输入。',
  'No Password Access': '对方未开放密码登录，已请求对方在设备上确认连接。',
  '2FA Required': '对方设备启用了双重认证，网页远控暂不支持。',
  'Wrong 2FA Code': '对方设备双重认证码错误。',
  Offline: '对方设备离线。',
  'Your ip is blocked by the peer': '当前 IP 已被对方阻止。',
  'Your ID is blocked by the peer': '当前账号已被对方阻止。',
  'Connection not allowed': '对方不允许此连接。',
  'Please try 1 minute later': '尝试过于频繁，请一分钟后再试。',
  'Too many wrong attempts': '密码错误次数过多，请稍后再试。',
  'The main window is not open': '对方 RustDesk 主窗口未打开，无法发起连接。',
  'No permission of keyboard': '对方未授予键盘控制权限。',
  'No permission of clipboard': '对方未授予剪贴板权限。',
}

const PHASE_TEXT = {
  preparing: '正在准备远控环境…',
  rendezvous: '正在连接信令服务器…',
  relay: '正在建立中继连接…',
  secure: '正在建立加密通道…',
  auth: '正在验证设备身份…',
  approving: '等待对方在设备上确认连接…',
  connected: '已连接',
  error: '连接失败',
  closed: '连接已断开',
}

function has(object, field) {
  return Object.prototype.hasOwnProperty.call(object, field)
}

function toBytes(value) {
  if (!value) return new Uint8Array(0)
  if (value instanceof Uint8Array) return value
  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
  }
  if (value instanceof ArrayBuffer) return new Uint8Array(value)
  return new Uint8Array(value)
}

function wsUrl(baseUrl, path, ticket) {
  const base = baseUrl ? new URL(baseUrl) : new URL(window.location.href)
  const protocol = base.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${base.host}${path}?ticket=${encodeURIComponent(ticket)}`
}

function formatLoginError(message) {
  const text = String(message || '').trim()
  if (!text) return '连接被对方拒绝。'
  return LOGIN_ERRORS[text] || `连接被对方拒绝：${text}`
}

export class WebControlClient {
  constructor({ session, handlers = {} }) {
    this.session = session
    this.handlers = handlers
    this.deviceId = session?.device?.id || ''
    this.phase = 'idle'
    this.error = ''
    this.closed = false
    this.started = false
    this.loggedIn = false
    this.rendezvous = null
    this.wire = null
    this.decoder = null
    this.canvas = null
    this.context = null
    this.hash = null
    this.peer = null
    this.peerPlatform = ''
    this.displays = []
    this.currentDisplay = 0
    this.quality = 3
    this.cursorCache = new Map()
    this.currentCursorId = ''
    this.videoSize = { width: 0, height: 0 }
    this.pressedKeys = new Map()
    this.pendingMove = null
    this.moveFrame = 0
    this.lastClipboard = ''
  }

  emit(event, payload) {
    const handler = this.handlers[event]
    if (typeof handler === 'function') handler(payload)
  }

  setPhase(phase, text) {
    this.phase = phase
    this.emit('onPhase', { phase, text: text || PHASE_TEXT[phase] || phase })
  }

  attachCanvas(canvas) {
    this.canvas = canvas
    this.context = canvas ? canvas.getContext('2d', { alpha: false, desynchronized: true }) : null
  }

  async start() {
    if (this.started) return
    this.started = true
    if (!this.deviceId) {
      this.fail('缺少设备 ID，无法发起远程控制。')
      return
    }
    try {
      this.setPhase('preparing')
      await initSodium()
      if (!window.isSecureContext) {
        throw new Error(
          '网页远控需要 HTTPS 安全上下文（或通过 localhost 访问）。请为控制台配置 HTTPS 后重试。'
        )
      }
      const vp9 = await checkVp9Support()
      if (!vp9) {
        throw new Error(
          '当前浏览器不支持 VP9 视频解码（WebCodecs）。请使用最新版 Chrome 或 Edge，并确认已启用硬件加速。'
        )
      }
      this.decoder = new Vp9Decoder({
        onFrame: (frame) => this.drawFrame(frame),
        onError: () => this.onDecoderError(),
        onNeedKeyFrame: () => this.requestKeyFrame(),
      })
      const relayResponse = await this.connectRendezvous()
      await this.connectRelay(relayResponse)
      this.setPhase('auth')
      this.loop()
    } catch (error) {
      this.fail(error?.message || '远程控制初始化失败。')
    }
  }

  async connectRendezvous() {
    this.setPhase('rendezvous')
    const wire = new Wire(wsUrl(this.session.baseUrl, this.session.ws_path, this.session.ticket))
    wire.useRendezvous()
    this.rendezvous = wire
    await wire.open(CONNECT_TIMEOUT)
    wire.sendRendezvous({
      punch_hole_request: {
        id: this.deviceId,
        nat_type: 2,
        force_relay: true,
        conn_type: 0,
        licence_key: this.session.license_key || '',
      },
    })
    this.setPhase('rendezvous', '正在等待对方设备响应…')
    const deadline = Date.now() + RENDEZVOUS_TIMEOUT
    while (Date.now() < deadline) {
      let message
      try {
        message = await wire.next(Math.max(1000, deadline - Date.now()))
      } catch (error) {
        if (error instanceof WireTimeoutError) break
        throw error
      }
      if (message.relay_response) {
        const response = message.relay_response
        if (!response.uuid) throw new Error('对方返回的中继标识无效。')
        if (!response.pk || !response.pk.length) {
          throw new Error('服务器未返回设备身份签名，请为 hbbs 配置 Key 后重试。')
        }
        this.rendezvous = null
        wire.close()
        return response
      }
      if (message.punch_hole_response) {
        const response = message.punch_hole_response
        if (response.other_failure) throw new Error(`连接失败：${response.other_failure}`)
        if (!response.socket_addr || !response.socket_addr.length) {
          const failure = Number(response.failure || 0)
          throw new Error(PUNCH_FAILURES[failure] || '对方设备拒绝连接。')
        }
      }
    }
    throw new Error(
      '等待对方响应超时。请确认设备在线、版本支持网页远控，并在设备上允许连接。'
    )
  }

  verifySignedId(signedBytes, key) {
    try {
      return verifySigned(toBytes(signedBytes), key)
    } catch {
      return null
    }
  }

  async connectRelay(relayResponse) {
    this.setPhase('secure')
    let serverKey
    try {
      serverKey = decodeServerKey(this.session.public_key)
    } catch {
      throw new Error('控制台未配置有效的服务器公钥，无法验证设备身份。')
    }
    const serverIdentity = this.verifySignedId(relayResponse.pk, serverKey)
    if (!serverIdentity) {
      throw new Error('设备身份签名校验失败，请检查控制台与 hbbs 是否使用同一套 Key。')
    }
    let identity
    try {
      identity = IdPk.decode(serverIdentity)
    } catch {
      throw new Error('设备身份数据损坏，无法继续连接。')
    }
    if (identity.id !== this.deviceId) {
      throw new Error('设备身份校验失败：设备 ID 不一致。')
    }
    const signKey = toBytes(identity.pk)
    if (signKey.length !== 32) {
      throw new Error('设备身份校验失败：公钥格式无效。')
    }

    const wire = new Wire(
      wsUrl(this.session.baseUrl, this.session.relay_path, this.session.relay_ticket)
    )
    this.wire = wire
    await wire.open(CONNECT_TIMEOUT)
    wire.sendRendezvous({
      request_relay: {
        id: this.deviceId,
        uuid: relayResponse.uuid,
        licence_key: this.session.license_key || '',
      },
    })

    const deadline = Date.now() + SECURE_TIMEOUT
    let signedId = null
    while (Date.now() < deadline && !signedId) {
      let message
      try {
        message = await wire.next(Math.max(1000, deadline - Date.now()))
      } catch (error) {
        if (error instanceof WireTimeoutError) break
        throw error
      }
      if (message.signed_id) signedId = message.signed_id
    }
    if (!signedId) {
      throw new Error(
        '对方未返回身份信息。请确认对方 RustDesk 版本支持网页远控，并检查中继服务是否正常。'
      )
    }
    const peerIdentity = this.verifySignedId(signedId.id, signKey)
    if (!peerIdentity) {
      throw new Error('对方身份签名校验失败，连接可能被篡改，已中止。')
    }
    let peer
    try {
      peer = IdPk.decode(peerIdentity)
    } catch {
      throw new Error('对方身份数据损坏，连接已中止。')
    }
    if (peer.id !== this.deviceId) {
      throw new Error('对方身份校验失败：设备 ID 不一致。')
    }
    const boxKey = toBytes(peer.pk)
    if (boxKey.length !== 32) {
      throw new Error('对方身份校验失败：加密公钥无效。')
    }

    const exchange = createKeyExchange(boxKey)
    wire.sendMessage({
      public_key: {
        asymmetric_value: exchange.publicKey,
        symmetric_value: exchange.sealedKey,
        kx_version: 0,
      },
    })
    wire.setSecretKey(exchange.secretKey)
  }

  async loop() {
    while (!this.closed) {
      let message
      try {
        message = await this.wire.next(0)
      } catch (error) {
        if (!this.closed) this.fail(error?.message || '连接已断开。')
        return
      }
      try {
        this.handleMessage(message)
      } catch (error) {
        this.fail(error?.message || '处理服务器消息失败。')
        return
      }
    }
  }

  handleMessage(message) {
    if (message.video_frame) {
      this.handleVideoFrame(message.video_frame)
      return
    }
    if (message.hash) {
      this.hash = {
        salt: message.hash.salt || '',
        challenge: message.hash.challenge || '',
      }
      this.setPhase('auth', '请输入设备连接密码。')
      this.emit('onPasswordRequired', { wrong: false })
      return
    }
    if (message.login_response) {
      this.handleLoginResponse(message.login_response)
      return
    }
    if (message.peer_info) {
      this.handlePeerInfo(message.peer_info)
      return
    }
    if (message.clipboard) {
      this.handleClipboard(message.clipboard)
      return
    }
    if (message.multi_clipboards) {
      for (const clipboard of message.multi_clipboards.clipboards || []) {
        this.handleClipboard(clipboard)
      }
      return
    }
    if (message.cursor_data) {
      this.handleCursorData(message.cursor_data)
      return
    }
    if (has(message, 'cursor_id')) {
      this.currentCursorId = String(message.cursor_id)
      this.applyCursor()
      return
    }
    if (message.cursor_position) {
      this.emit('onCursorPosition', {
        x: Number(message.cursor_position.x || 0),
        y: Number(message.cursor_position.y || 0),
      })
      return
    }
    if (message.test_delay) {
      if (!message.test_delay.from_client) {
        this.send({ test_delay: message.test_delay })
      }
      return
    }
    if (message.misc) {
      this.handleMisc(message.misc)
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

  handleLoginResponse(response) {
    if (response.error) {
      const text = String(response.error)
      if (text === 'Wrong Password') {
        this.setPhase('auth', '密码错误，请重试。')
        this.emit('onPasswordRequired', { wrong: true })
        return
      }
      if (text === 'No Password Access') {
        this.setPhase('approving')
        this.emit('onNotice', LOGIN_ERRORS[text])
        return
      }
      this.fail(formatLoginError(text))
      return
    }
    if (response.peer_info) {
      this.handlePeerInfo(response.peer_info)
    }
  }

  handlePeerInfo(info) {
    this.peer = info
    this.loggedIn = true
    this.peerPlatform = info.platform || ''
    this.displays = Array.isArray(info.displays) ? info.displays : []
    this.currentDisplay = Number(info.current_display || 0)
    this.setPhase('connected')
    this.emit('onPeer', info)
    this.emit('onDisplays', {
      displays: this.displays,
      current: this.currentDisplay,
    })
  }

  handleVideoFrame(frame) {
    const display = Number(frame.display || 0)
    if (display !== this.currentDisplay) {
      this.currentDisplay = display
      this.emit('onDisplays', {
        displays: this.displays,
        current: this.currentDisplay,
      })
    }
    const stream = frame.vp9s
    if (!stream) {
      if (frame.rgb || frame.yuv) {
        this.fail('对方发送了浏览器不支持的原始视频格式，请升级对方 RustDesk 客户端。')
      }
      return
    }
    if (this.decoder) {
      for (const encoded of stream.frames || []) {
        this.decoder.decode(toBytes(encoded.data), encoded.key === true)
      }
    }
    this.sendVideoAck()
  }

  sendVideoAck() {
    this.send({ misc: { video_received: true } })
  }

  requestKeyFrame() {
    if (this.closed) return
    this.send({ misc: { refresh_video: true } })
  }

  onDecoderError() {
    this.emit('onNotice', '视频解码出现问题，正在请求关键帧…')
    this.requestKeyFrame()
  }

  drawFrame(frame) {
    if (!this.canvas || !this.context) return
    const width = frame.displayWidth
    const height = frame.displayHeight
    if (!width || !height) return
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width
      this.canvas.height = height
      this.videoSize = { width, height }
      this.emit('onVideoSize', this.videoSize)
    }
    this.context.drawImage(frame, 0, 0, width, height)
    if (!this.loggedIn) this.loggedIn = true
    this.emit('onFrame')
  }

  handleMisc(misc) {
    if (has(misc, 'close_reason')) {
      this.emit('onPeerClosed', {
        reason: String(misc.close_reason) || '对方已结束远程会话。',
      })
      this.close()
      return
    }
    if (misc.switch_display) {
      const display = Number(misc.switch_display.display || 0)
      this.currentDisplay = display
      this.decoder?.configure(0, 0)
      this.emit('onDisplays', {
        displays: this.displays,
        current: display,
        width: misc.switch_display.width,
        height: misc.switch_display.height,
      })
      this.sendVideoAck()
      return
    }
    if (misc.permission_info) {
      this.emit('onPermission', {
        permission: Number(misc.permission_info.permission || 0),
        enabled: misc.permission_info.enabled === true,
      })
      return
    }
    if (misc.option) {
      this.emit('onRemoteOption', misc.option)
    }
  }

  handleClipboard(clipboard) {
    if (Number(clipboard.format || 0) !== 0) return
    let bytes = toBytes(clipboard.content)
    if (!bytes.length) return
    if (clipboard.compress) {
      try {
        bytes = decompress(bytes)
      } catch {
        return
      }
    }
    let text = ''
    try {
      text = new TextDecoder().decode(bytes)
    } catch {
      return
    }
    if (!text || text === this.lastClipboard) return
    this.lastClipboard = text
    this.emit('onClipboard', text)
  }

  handleCursorData(data) {
    const width = Number(data.width || 0)
    const height = Number(data.height || 0)
    if (width < 1 || height < 1 || width > 512 || height > 512) return
    const expected = width * height * 4
    let colors = toBytes(data.colors)
    if (colors.length !== expected) {
      try {
        colors = decompress(colors, new Uint8Array(expected))
      } catch {
        return
      }
    }
    if (colors.length !== expected) return
    const cursorCanvas = document.createElement('canvas')
    cursorCanvas.width = width
    cursorCanvas.height = height
    const context = cursorCanvas.getContext('2d')
    if (!context) return
    const image = new ImageData(new Uint8ClampedArray(colors), width, height)
    context.putImageData(image, 0, 0)
    const id = String(data.id ?? '')
    if (!id) return
    this.cursorCache.set(id, {
      url: cursorCanvas.toDataURL('image/png'),
      hotx: Number(data.hotx || 0),
      hoty: Number(data.hoty || 0),
    })
    if (this.cursorCache.size > 64) {
      const oldest = this.cursorCache.keys().next().value
      if (oldest !== id) this.cursorCache.delete(oldest)
    }
    if (!this.currentCursorId || this.currentCursorId === id) {
      this.currentCursorId = this.currentCursorId || id
      this.applyCursor()
    }
  }

  applyCursor() {
    if (!this.canvas) return
    const shape = this.cursorCache.get(String(this.currentCursorId))
    if (!shape) {
      this.canvas.style.cursor = 'default'
      return
    }
    this.canvas.style.cursor = `url(${shape.url}) ${shape.hotx} ${shape.hoty}, default`
  }

  async submitPassword(password) {
    if (!this.wire || this.closed) return
    if (!this.hash) {
      this.emit('onNotice', '尚未收到设备的验证信息，请稍候。')
      return
    }
    const value = String(password ?? '')
    if (!value) {
      this.requestApproval()
      return
    }
    const h1 = await sha256(concatBytes(new TextEncoder().encode(value), new TextEncoder().encode(this.hash.salt)))
    const h2 = await sha256(concatBytes(h1, new TextEncoder().encode(this.hash.challenge)))
    this.sendLogin(h2)
  }

  requestApproval() {
    if (!this.wire || this.closed) return
    this.setPhase('approving')
    this.sendLogin(null)
  }

  resendLogin() {
    if (!this.wire || this.closed) return
    this.sendLogin(null)
  }

  sendLogin(password) {
    this.send({
      login_request: {
        username: this.deviceId,
        password: password || new Uint8Array(0),
        my_id: 'web-console',
        my_name: '网页控制台',
        my_platform: 'Web',
        option: this.buildOption(),
        video_ack_required: true,
        version: '1.5.1',
      },
    })
  }

  buildOption() {
    return {
      image_quality: this.quality,
      show_remote_cursor: 2,
      disable_audio: 2,
      disable_clipboard: 1,
      supported_decoding: {
        ability_vp9: 1,
        ability_h264: 0,
        ability_h265: 0,
        ability_vp8: 0,
        ability_av1: 0,
        prefer: 1,
        prefer_chroma: 0,
      },
    }
  }

  setQuality(quality) {
    const value = Number(quality)
    if (![2, 3, 4].includes(value)) return
    this.quality = value
    this.send({ misc: { option: { image_quality: value } } })
  }

  switchDisplay(index) {
    const value = Number(index)
    if (!Number.isInteger(value) || value < 0) return
    this.currentDisplay = value
    this.decoder?.configure(0, 0)
    this.send({ misc: { message_query: { switch_display: value } } })
    this.emit('onDisplays', { displays: this.displays, current: value })
  }

  refresh() {
    this.decoder?.configure(0, 0)
    this.send({ misc: { refresh_video: true } })
  }

  ctrlAltDel() {
    if (this.peerPlatform === 'Windows') {
      this.send({
        key_event: {
          control_key: 100,
          down: true,
          mode: 0,
          modifiers: [],
        },
      })
      return
    }
    this.send({
      key_event: {
        control_key: CONTROL_KEY.Delete,
        down: true,
        press: true,
        mode: 0,
        modifiers: [CONTROL_KEY.ControlLeft, CONTROL_KEY.AltLeft],
      },
    })
  }

  lockScreen() {
    this.send({
      key_event: {
        control_key: 101,
        down: true,
        mode: 0,
        modifiers: [],
      },
    })
  }

  sendClipboardText(text) {
    const value = String(text ?? '')
    if (!value) return
    this.lastClipboard = value
    this.send({
      clipboard: {
        compress: false,
        format: 0,
        content: new TextEncoder().encode(value),
      },
    })
  }

  async pasteLocalClipboard() {
    if (!navigator.clipboard?.readText) {
      throw new Error('当前浏览器不支持读取本地剪贴板。')
    }
    const text = await navigator.clipboard.readText()
    if (!text) throw new Error('本地剪贴板为空。')
    this.sendClipboardText(text)
    return text
  }

  sendKeyboardEvent(event, down) {
    if (!this.loggedIn || !this.wire || this.closed) return false
    if (event.isComposing || event.keyCode === 229) return false
    const code = event.code || ''
    const descriptor = keyDescriptor(event)
    if (descriptor) {
      if (down) {
        this.pressedKeys.set(code, {
          descriptor,
          modifiers: modifierList(event, code),
        })
      } else {
        this.pressedKeys.delete(code)
      }
      this.send({
        key_event: {
          ...descriptor,
          down,
          press: false,
          mode: 0,
          modifiers: modifierList(event, code),
        },
      })
      return true
    }
    if (down && !isModifierCode(code)) {
      const text = event.key || ''
      if ([...text].length === 1) {
        const point = text.codePointAt(0)
        if (point >= 0x20) {
          this.send({
            key_event: {
              unicode: point,
              down: true,
              press: true,
              mode: 0,
              modifiers: [],
            },
          })
          return true
        }
      }
    }
    return false
  }

  sendComposition(text) {
    if (!this.loggedIn || !this.wire || this.closed) return
    for (const char of String(text || '')) {
      const point = char.codePointAt(0)
      if (point === undefined) continue
      this.send({
        key_event: {
          unicode: point,
          down: true,
          press: true,
          mode: 0,
          modifiers: [],
        },
      })
    }
  }

  releaseAllKeys() {
    const wire = this.wire
    for (const [code, pressed] of this.pressedKeys) {
      if (wire && !wire.closed) {
        try {
          wire.sendMessage({
            key_event: {
              ...pressed.descriptor,
              down: false,
              press: false,
              mode: 0,
              modifiers: pressed.modifiers,
            },
          })
        } catch {
          // The connection is already gone; there is nothing left to release.
        }
      }
      this.pressedKeys.delete(code)
    }
  }

  canvasPoint(event) {
    if (!this.canvas) return { x: 0, y: 0 }
    const rect = this.canvas.getBoundingClientRect()
    if (!rect.width || !rect.height) return { x: 0, y: 0 }
    const x = Math.round(
      ((event.clientX - rect.left) / rect.width) * this.canvas.width
    )
    const y = Math.round(
      ((event.clientY - rect.top) / rect.height) * this.canvas.height
    )
    return {
      x: Math.max(0, Math.min(this.canvas.width - 1, x)),
      y: Math.max(0, Math.min(this.canvas.height - 1, y)),
    }
  }

  sendMouse(point, mask) {
    if (!this.loggedIn || !this.wire || this.closed) return
    this.send({
      mouse_event: {
        mask,
        x: Math.round(point.x),
        y: Math.round(point.y),
        modifiers: [],
      },
    })
  }

  mouseMove(event) {
    this.queueMove(event)
  }

  queueMove(event) {
    this.pendingMove = event
    if (this.moveFrame) return
    this.moveFrame = requestAnimationFrame(() => {
      this.moveFrame = 0
      const pending = this.pendingMove
      this.pendingMove = null
      if (!pending) return
      this.sendMouse(this.canvasPoint(pending), MOUSE_TYPE_MOVE)
    })
  }

  mouseDown(event) {
    this.sendMouse(
      this.canvasPoint(event),
      MOUSE_TYPE_DOWN | buttonMask(event.button)
    )
  }

  mouseUp(event) {
    this.sendMouse(
      this.canvasPoint(event),
      MOUSE_TYPE_UP | buttonMask(event.button)
    )
  }

  mouseWheel(event) {
    const x = -Math.sign(event.deltaX)
    const y = -Math.sign(event.deltaY)
    if (!x && !y) return
    this.sendMouse(
      { x, y },
      MOUSE_TYPE_WHEEL
    )
  }

  send(payload) {
    if (!this.wire || this.wire.closed || this.closed) return
    try {
      this.wire.sendMessage(payload)
    } catch (error) {
      this.fail(error?.message || '发送数据失败。')
    }
  }

  fail(message) {
    if (this.closed) return
    this.closed = true
    this.setPhase('error', message)
    this.teardown()
    this.emit('onError', message)
  }

  close(reason = '已断开远程控制。') {
    if (this.closed) return
    this.closed = true
    this.setPhase('closed', reason)
    this.teardown()
    this.emit('onClosed', reason)
  }

  teardown() {
    this.releaseAllKeys()
    if (this.moveFrame) {
      cancelAnimationFrame(this.moveFrame)
      this.moveFrame = 0
    }
    this.pendingMove = null
    for (const wire of [this.rendezvous, this.wire]) {
      try {
        wire?.close()
      } catch {
        // Ignore close failures.
      }
    }
    this.rendezvous = null
    this.wire = null
    try {
      this.decoder?.close()
    } catch {
      // Ignore decoder close failures.
    }
    this.decoder = null
  }
}

async function sha256(bytes) {
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return new Uint8Array(digest)
}

function concatBytes(a, b) {
  const out = new Uint8Array(a.length + b.length)
  out.set(a, 0)
  out.set(b, a.length)
  return out
}
