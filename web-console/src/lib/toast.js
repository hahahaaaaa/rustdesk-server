import { reactive } from 'vue'

export const toasts = reactive([])

let nextId = 1

export function notify(message, type = 'success') {
  const id = nextId++
  toasts.push({ id, message, type })
  window.setTimeout(() => dismiss(id), 3400)
}

export function dismiss(id) {
  const index = toasts.findIndex((toast) => toast.id === id)
  if (index !== -1) toasts.splice(index, 1)
}
