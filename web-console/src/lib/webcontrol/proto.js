import { parse } from 'protobufjs'
import messageSource from './proto/message.proto?raw'
import rendezvousSource from './proto/rendezvous.proto?raw'

const { root } = parse(messageSource, { keepCase: true })
parse(rendezvousSource, root, { keepCase: true })

export const Message = root.lookupType('hbb.Message')
export const RendezvousMessage = root.lookupType('hbb.RendezvousMessage')
export const IdPk = root.lookupType('hbb.IdPk')

export function encodeMessage(payload) {
  return Message.encode(Message.fromObject(payload)).finish()
}

export function encodeRendezvous(payload) {
  return RendezvousMessage.encode(RendezvousMessage.fromObject(payload)).finish()
}

export function decodeMessage(bytes) {
  return Message.decode(bytes)
}

export function decodeRendezvous(bytes) {
  return RendezvousMessage.decode(bytes)
}
