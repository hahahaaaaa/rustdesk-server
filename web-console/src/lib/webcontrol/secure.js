import _sodium from 'libsodium-wrappers'

let sodium = null

export async function initSodium() {
  if (!sodium) {
    await _sodium.ready
    sodium = _sodium
  }
  return sodium
}

function requireSodium() {
  if (!sodium) throw new Error('加密模块尚未初始化。')
  return sodium
}

export function decodeBase64(value) {
  return requireSodium().from_base64(value, requireSodium().base64_variants.ORIGINAL)
}

export function decodeServerKey(publicKey) {
  const bytes = decodeBase64(String(publicKey || '').trim())
  if (bytes.length !== 32) throw new Error('服务器公钥格式无效。')
  return bytes
}

export function verifySigned(signed, serverKey) {
  return requireSodium().crypto_sign_open(signed, serverKey)
}

export function createKeyExchange(theirPublicKey) {
  const api = requireSodium()
  const pair = api.crypto_box_keypair()
  const key = api.crypto_secretbox_keygen()
  const nonce = new Uint8Array(api.crypto_box_NONCEBYTES)
  const sealed = api.crypto_box_easy(key, nonce, theirPublicKey, pair.privateKey)
  return { publicKey: pair.publicKey, sealedKey: sealed, secretKey: key }
}

export function nonceFor(counter) {
  const api = requireSodium()
  const nonce = new Uint8Array(api.crypto_secretbox_NONCEBYTES)
  let value = Number(counter)
  for (let index = 0; index < nonce.length && value > 0; index += 1) {
    nonce[index] = value & 0xff
    value = Math.floor(value / 256)
  }
  return nonce
}

export function encryptMessage(bytes, counter, key) {
  return requireSodium().crypto_secretbox_easy(bytes, nonceFor(counter), key)
}

export function decryptMessage(bytes, counter, key) {
  return requireSodium().crypto_secretbox_open_easy(bytes, nonceFor(counter), key)
}
