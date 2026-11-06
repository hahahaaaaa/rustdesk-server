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

// Windows set-1 scan codes by KeyboardEvent.code. Extended keys carry the 0xE0 prefix
// in the high byte, which is the format RustDesk's Map keyboard mode expects on Windows.
// Sending physical scan codes lets the controlled side's IME process the keystrokes.
const WINDOWS_SCAN_CODES = {
  Escape: 0x01,
  Digit1: 0x02,
  Digit2: 0x03,
  Digit3: 0x04,
  Digit4: 0x05,
  Digit5: 0x06,
  Digit6: 0x07,
  Digit7: 0x08,
  Digit8: 0x09,
  Digit9: 0x0a,
  Digit0: 0x0b,
  Minus: 0x0c,
  Equal: 0x0d,
  Backspace: 0x0e,
  Tab: 0x0f,
  KeyQ: 0x10,
  KeyW: 0x11,
  KeyE: 0x12,
  KeyR: 0x13,
  KeyT: 0x14,
  KeyY: 0x15,
  KeyU: 0x16,
  KeyI: 0x17,
  KeyO: 0x18,
  KeyP: 0x19,
  BracketLeft: 0x1a,
  BracketRight: 0x1b,
  Enter: 0x1c,
  ControlLeft: 0x1d,
  KeyA: 0x1e,
  KeyS: 0x1f,
  KeyD: 0x20,
  KeyF: 0x21,
  KeyG: 0x22,
  KeyH: 0x23,
  KeyJ: 0x24,
  KeyK: 0x25,
  KeyL: 0x26,
  Semicolon: 0x27,
  Quote: 0x28,
  Backquote: 0x29,
  ShiftLeft: 0x2a,
  Backslash: 0x2b,
  KeyZ: 0x2c,
  KeyX: 0x2d,
  KeyC: 0x2e,
  KeyV: 0x2f,
  KeyB: 0x30,
  KeyN: 0x31,
  KeyM: 0x32,
  Comma: 0x33,
  Period: 0x34,
  Slash: 0x35,
  ShiftRight: 0x36,
  NumpadMultiply: 0x37,
  AltLeft: 0x38,
  Space: 0x39,
  F1: 0x3b,
  F2: 0x3c,
  F3: 0x3d,
  F4: 0x3e,
  F5: 0x3f,
  F6: 0x40,
  F7: 0x41,
  F8: 0x42,
  F9: 0x43,
  F10: 0x44,
  ScrollLock: 0x46,
  Numpad7: 0x47,
  Numpad8: 0x48,
  Numpad9: 0x49,
  NumpadSubtract: 0x4a,
  Numpad4: 0x4b,
  Numpad5: 0x4c,
  Numpad6: 0x4d,
  NumpadAdd: 0x4e,
  Numpad1: 0x4f,
  Numpad2: 0x50,
  Numpad3: 0x51,
  Numpad0: 0x52,
  NumpadDecimal: 0x53,
  IntlBackslash: 0x56,
  F11: 0x57,
  F12: 0x58,
  NumpadEnter: 0xe01c,
  ControlRight: 0xe01d,
  NumpadDivide: 0xe035,
  PrintScreen: 0xe037,
  AltRight: 0xe038,
  Home: 0xe047,
  ArrowUp: 0xe048,
  PageUp: 0xe049,
  ArrowLeft: 0xe04b,
  ArrowRight: 0xe04d,
  End: 0xe04f,
  ArrowDown: 0xe050,
  PageDown: 0xe051,
  Insert: 0xe052,
  Delete: 0xe053,
  MetaLeft: 0xe05b,
  MetaRight: 0xe05c,
  ContextMenu: 0xe05d,
}

export function windowsScanCode(code) {
  const value = WINDOWS_SCAN_CODES[code || '']
  return value === undefined ? null : value
}

// Map mode does not sync Shift/Ctrl/Alt from the modifier list (those travel as their
// own key events), but the controlled side still aligns Caps Lock and Num Lock with it.
export function lockModifiers(event) {
  const modifiers = []
  if (event.getModifierState?.('CapsLock')) modifiers.push(CONTROL_KEYS.CapsLock)
  if (event.getModifierState?.('NumLock')) modifiers.push(CONTROL_KEYS.NumLock)
  return modifiers
}

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
