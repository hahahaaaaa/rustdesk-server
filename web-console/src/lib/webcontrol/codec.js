const VP9_CODEC = 'vp09.00.10.08'

export async function checkVp9Support() {
  if (
    typeof VideoDecoder === 'undefined' ||
    typeof EncodedVideoChunk === 'undefined'
  ) {
    return false
  }
  try {
    const result = await VideoDecoder.isConfigSupported({
      codec: VP9_CODEC,
      optimizeForLatency: true,
    })
    return result.supported === true
  } catch {
    return false
  }
}

export class Vp9Decoder {
  constructor({ onFrame, onError, onNeedKeyFrame }) {
    this.onFrame = onFrame
    this.onError = onError
    this.onNeedKeyFrame = onNeedKeyFrame
    this.decoder = null
    this.width = 0
    this.height = 0
    this.timestamp = 0
    this.waitingForKey = true
    this.closed = false
    this.handleOutput = this.handleOutput.bind(this)
    this.handleError = this.handleError.bind(this)
  }

  configure(width = 0, height = 0) {
    if (this.closed) return
    this.closeDecoder()
    this.width = Math.floor(width) || 0
    this.height = Math.floor(height) || 0
    const config = { codec: VP9_CODEC, optimizeForLatency: true }
    this.decoder = new VideoDecoder({
      output: this.handleOutput,
      error: this.handleError,
    })
    this.decoder.configure(config)
    this.waitingForKey = true
  }

  decode(data, key) {
    if (this.closed) return
    if (!this.decoder || this.decoder.state !== 'configured') {
      if (!key) return
      this.configure(this.width, this.height)
    }
    if (this.waitingForKey && !key) return
    if (key) this.waitingForKey = false
    const chunk = new EncodedVideoChunk({
      type: key ? 'key' : 'delta',
      timestamp: this.timestamp,
      data,
    })
    this.timestamp += 1000
    try {
      this.decoder.decode(chunk)
    } catch (error) {
      this.handleError(error)
    }
  }

  handleOutput(frame) {
    if (this.closed) {
      frame.close()
      return
    }
    try {
      this.onFrame?.(frame)
    } finally {
      frame.close()
    }
  }

  handleError(error) {
    if (this.closed) return
    this.waitingForKey = true
    this.closeDecoder()
    this.onError?.(error)
    this.onNeedKeyFrame?.()
  }

  closeDecoder() {
    if (!this.decoder) return
    try {
      if (this.decoder.state !== 'closed') this.decoder.close()
    } catch {
      // Decoder already torn down by a decode error.
    }
    this.decoder = null
  }

  close() {
    this.closed = true
    this.closeDecoder()
  }
}
