const CONTROL_KEYS = {
  AltLeft: 1,
  Backspace: 2,
  CapsLock: 3,
  ControlLeft: 4,
  Delete: 5,
  ArrowDown: 6,
  End: 7,
  Escape: 8,
  F1: 9,
  F10: 10,
  F11: 11,
  F12: 12,
  F2: 13,
  F3: 14,
  F4: 15,
  F5: 16,
  F6: 17,
  F7: 18,
  F8: 19,
  F9: 20,
  Home: 21,
  ArrowLeft: 22,
  MetaLeft: 23,
  PageDown: 25,
  PageUp: 26,
  Enter: 27,
  ArrowRight: 28,
  ShiftLeft: 29,
  Space: 30,
  Tab: 31,
  ArrowUp: 32,
  Numpad0: 33,
  Numpad1: 34,
  Numpad2: 35,
  Numpad3: 36,
  Numpad4: 37,
  Numpad5: 38,
  Numpad6: 39,
  Numpad7: 40,
  Numpad8: 41,
  Numpad9: 42,
  Pause: 46,
  PrintScreen: 57,
  Insert: 58,
  NumpadMultiply: 66,
  NumpadAdd: 67,
  NumpadSubtract: 68,
  NumpadDecimal: 69,
  NumpadDivide: 70,
  NumpadEnter: 72,
  ShiftRight: 73,
  ControlRight: 74,
  AltRight: 75,
  NumLock: 63,
  ScrollLock: 62,
  ContextMenu: 65,
  MetaRight: 64,
}

const ASCII_KEYS = {
  Backquote: '`',
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  IntlBackslash: '\\',
  Semicolon: ';',
  Quote: "'",
  Comma: ',',
  Period: '.',
  Slash: '/',
}

export const CONTROL_KEY = CONTROL_KEYS

export function keyDescriptor(event) {
  const code = event.code || ''
  if (CONTROL_KEYS[code] !== undefined) {
    return { control_key: CONTROL_KEYS[code] }
  }
  if (/^Key[A-Z]$/.test(code)) {
    return { chr: code.charCodeAt(3) + 32 }
  }
  if (/^Digit[0-9]$/.test(code)) {
    return { chr: code.charCodeAt(5) }
  }
  if (ASCII_KEYS[code] !== undefined) {
    return { chr: ASCII_KEYS[code].charCodeAt(0) }
  }
  if (event.key && event.key.length === 1) {
    const value = event.key.charCodeAt(0)
    if (value >= 0x20 && value <= 0x7e) return { chr: value }
  }
  return null
}

export function isModifierCode(code) {
  return [
    'AltLeft',
    'AltRight',
    'ControlLeft',
    'ControlRight',
    'MetaLeft',
    'MetaRight',
    'ShiftLeft',
    'ShiftRight',
  ].includes(code)
}

export function modifierList(event, selfCode = '') {
  const modifiers = []
  if (event.altKey && selfCode !== 'AltLeft' && selfCode !== 'AltRight') {
    modifiers.push(CONTROL_KEYS.AltLeft)
  }
  if (event.ctrlKey && selfCode !== 'ControlLeft' && selfCode !== 'ControlRight') {
    modifiers.push(CONTROL_KEYS.ControlLeft)
  }
  if (event.shiftKey && selfCode !== 'ShiftLeft' && selfCode !== 'ShiftRight') {
    modifiers.push(CONTROL_KEYS.ShiftLeft)
  }
  if (event.metaKey && selfCode !== 'MetaLeft' && selfCode !== 'MetaRight') {
    modifiers.push(CONTROL_KEYS.MetaLeft)
  }
  return modifiers
}

export const MOUSE_TYPE_MOVE = 0
export const MOUSE_TYPE_DOWN = 1
export const MOUSE_TYPE_UP = 2
export const MOUSE_TYPE_WHEEL = 3

export const MOUSE_BUTTON_LEFT = 1 << 3
export const MOUSE_BUTTON_RIGHT = 2 << 3
export const MOUSE_BUTTON_MIDDLE = 4 << 3

export function buttonMask(button) {
  if (button === 1) return MOUSE_BUTTON_MIDDLE
  if (button === 2) return MOUSE_BUTTON_RIGHT
  return MOUSE_BUTTON_LEFT
}
