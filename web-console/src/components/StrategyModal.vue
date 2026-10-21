<script setup>
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { Plus, SlidersHorizontal, Trash2 } from 'lucide-vue-next'

const ACCESS_FIELDS = [
  {
    key: 'access-mode',
    label: '访问模式',
    choices: [
      ['full', '完全控制'],
      ['view', '仅查看'],
    ],
  },
  {
    key: 'verification-method',
    label: '密码验证方式',
    choices: [
      ['use-both-passwords', '双密码（永久 + 临时）'],
      ['use-temporary-password', '仅临时密码'],
      ['use-permanent-password', '仅永久密码'],
    ],
  },
  {
    key: 'approve-mode',
    label: '连接确认方式',
    choices: [
      ['both', '双方确认'],
      ['password', '密码确认'],
      ['click', '点击确认'],
    ],
  },
  {
    key: 'temporary-password-length',
    label: '临时密码位数',
    choices: [
      ['6', '6 位'],
      ['8', '8 位'],
      ['10', '10 位'],
    ],
  },
]

const FEATURE_FIELDS = [
  { key: 'enable-file-transfer', label: '文件传输' },
  { key: 'enable-clipboard', label: '剪贴板同步' },
  { key: 'enable-keyboard', label: '键盘输入' },
  { key: 'enable-audio', label: '音频' },
  { key: 'enable-camera', label: '摄像头' },
  { key: 'enable-terminal', label: '终端' },
  { key: 'enable-remote-restart', label: '远程重启' },
  { key: 'enable-privacy-mode', label: '隐私模式' },
  { key: 'enable-tunnel', label: '网络隧道' },
  { key: 'enable-trusted-devices', label: '可信设备' },
  { key: 'enable-lan-discovery', label: '局域网发现' },
  { key: 'allow-auto-update', label: '自动更新' },
  { key: 'allow-auto-disconnect', label: '空闲自动断开' },
  { key: 'allow-remote-config-modification', label: '允许修改远端配置' },
]

const TIMEOUT_KEY = 'auto-disconnect-timeout'
const KNOWN_KEYS = new Set([
  ...ACCESS_FIELDS.map((field) => field.key),
  ...FEATURE_FIELDS.map((field) => field.key),
  TIMEOUT_KEY,
])

const props = defineProps({
  strategy: { type: Object, default: null },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const source = props.strategy?.config_options || {}
const name = ref(props.strategy?.name || '')
const note = ref(props.strategy?.note || '')
const enabled = ref(props.strategy?.enabled !== false)
const values = reactive({})
for (const field of [...ACCESS_FIELDS, ...FEATURE_FIELDS]) {
  values[field.key] = typeof source[field.key] === 'string' ? source[field.key] : ''
}
const timeout = ref(
  typeof source[TIMEOUT_KEY] === 'string' ? source[TIMEOUT_KEY] : ''
)
const custom = ref(
  Object.entries(source)
    .filter(([key]) => !KNOWN_KEYS.has(key))
    .map(([key, value]) => ({
      key,
      value: typeof value === 'string' ? value : String(value),
    }))
)
const localError = ref('')

function addCustom() {
  custom.value.push({ key: '', value: '' })
}

function submit() {
  if (props.busy) return
  const title = name.value.trim()
  if (!title) {
    localError.value = '请填写策略名称。'
    return
  }
  const options = {}
  for (const field of [...ACCESS_FIELDS, ...FEATURE_FIELDS]) {
    if (values[field.key]) options[field.key] = values[field.key]
  }
  const timeoutValue = timeout.value.trim()
  if (timeoutValue) {
    if (!/^\d{1,5}$/.test(timeoutValue)) {
      localError.value = '自动断开超时需填写秒数（数字）。'
      return
    }
    options[TIMEOUT_KEY] = timeoutValue
  }
  for (const row of custom.value) {
    const key = row.key.trim()
    const value = row.value
    if (!key && !value) continue
    if (!key) {
      localError.value = '自定义配置的键不能为空。'
      return
    }
    if (KNOWN_KEYS.has(key)) {
      localError.value = `“${key}”已在上方快速配置中，请直接使用对应选项。`
      return
    }
    if (Object.prototype.hasOwnProperty.call(options, key)) {
      localError.value = `配置键“${key}”重复。`
      return
    }
    options[key] = value
  }
  localError.value = ''
  emit('submit', {
    name: title,
    note: note.value.trim(),
    enabled: enabled.value,
    config_options: options,
  })
}

function onKey(event) {
  if (event.key === 'Escape' && !props.busy) emit('cancel')
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="modal-scrim" @click.self="!busy && emit('cancel')">
    <form
      class="modal strategy-modal"
      role="dialog"
      aria-modal="true"
      @submit.prevent="submit"
    >
      <h2 class="modal-title">
        {{ strategy ? '编辑策略' : '新建策略' }}
      </h2>
      <p class="modal-sub">
        策略会随客户端心跳下发，设备优先于用户，用户优先于设备分组。
      </p>
      <div class="modal-body strategy-modal-body">
        <div class="strategy-form-grid">
          <div class="field">
            <label class="label" for="st-name">策略名称</label>
            <input
              id="st-name"
              v-model="name"
              class="input"
              placeholder="例如：安全基线"
              maxlength="48"
              autofocus
            />
          </div>
          <div class="field">
            <label class="label">策略状态</label>
            <label class="enable-toggle">
              <input v-model="enabled" type="checkbox" />
              <span>
                <strong>{{ enabled ? '已启用' : '已停用' }}</strong>
                <small>停用后停止向客户端下发</small>
              </span>
            </label>
          </div>
        </div>

        <div class="field">
          <label class="label" for="st-note">
            备注 <span class="hint">可选</span>
          </label>
          <textarea
            id="st-note"
            v-model="note"
            class="textarea"
            placeholder="说明这个策略适用的场景。"
            maxlength="240"
          />
        </div>

        <section class="strategy-section">
          <div class="strategy-section-head">
            <SlidersHorizontal :size="14" />
            <h3>访问与会话</h3>
            <span>留空表示不限制</span>
          </div>
          <div class="strategy-form-grid">
            <div v-for="field in ACCESS_FIELDS" :key="field.key" class="field">
              <label class="label" :for="`st-${field.key}`">
                {{ field.label }}
              </label>
              <select
                :id="`st-${field.key}`"
                v-model="values[field.key]"
                class="select"
              >
                <option value="">未设置</option>
                <option
                  v-for="[value, label] in field.choices"
                  :key="value"
                  :value="value"
                >
                  {{ label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label class="label" :for="`st-${TIMEOUT_KEY}`">
                空闲断开超时
              </label>
              <input
                :id="`st-${TIMEOUT_KEY}`"
                v-model="timeout"
                class="input"
                inputmode="numeric"
                placeholder="秒数，留空则使用客户端默认"
              />
            </div>
          </div>
        </section>

        <section class="strategy-section">
          <div class="strategy-section-head">
            <SlidersHorizontal :size="14" />
            <h3>功能开关</h3>
            <span>未设置时沿用客户端默认值</span>
          </div>
          <div class="strategy-form-grid feature-grid">
            <div v-for="field in FEATURE_FIELDS" :key="field.key" class="field">
              <label class="label" :for="`st-${field.key}`">
                {{ field.label }}
              </label>
              <select
                :id="`st-${field.key}`"
                v-model="values[field.key]"
                class="select"
              >
                <option value="">跟随默认</option>
                <option value="Y">启用</option>
                <option value="N">禁用</option>
              </select>
            </div>
          </div>
        </section>

        <section class="strategy-section">
          <div class="strategy-section-head">
            <SlidersHorizontal :size="14" />
            <h3>自定义配置</h3>
            <span>兼容客户端支持的其他键值</span>
          </div>
          <div v-if="custom.length" class="custom-list">
            <div
              v-for="(row, index) in custom"
              :key="index"
              class="custom-row"
            >
              <input
                v-model="row.key"
                class="input"
                placeholder="配置键，例如 enable-file-transfer"
                maxlength="128"
              />
              <input
                v-model="row.value"
                class="input"
                placeholder="值，例如 Y"
                maxlength="1024"
              />
              <button
                class="icon-btn danger"
                type="button"
                title="移除配置项"
                @click="custom.splice(index, 1)"
              >
                <Trash2 :size="14" />
              </button>
            </div>
          </div>
          <button class="btn btn-ghost btn-sm" type="button" @click="addCustom">
            <Plus :size="13" />
            添加配置项
          </button>
        </section>

        <p v-if="localError || error" class="field-error" style="margin-top: 10px">
          {{ localError || error }}
        </p>
      </div>
      <div class="modal-actions">
        <button
          type="button"
          class="btn btn-secondary"
          :disabled="busy"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button
          type="submit"
          class="btn btn-primary"
          :disabled="busy || !name.trim()"
        >
          <span v-if="busy" class="spinner" />
          {{ busy ? '保存中…' : strategy ? '保存策略' : '创建策略' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.strategy-modal {
  width: min(760px, 100%);
}

.strategy-modal-body {
  max-height: min(66vh, 620px);
  overflow-y: auto;
  padding-right: 4px;
}

.strategy-form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.strategy-form-grid .field + .field {
  margin-top: 0;
}

.strategy-modal-body > .field {
  margin-top: 14px;
}

.enable-toggle {
  display: flex;
  align-items: center;
  gap: 9px;
  min-height: 36px;
  padding: 5px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.enable-toggle input {
  accent-color: var(--ink);
}

.enable-toggle strong {
  display: block;
  font-size: 12px;
  font-weight: 500;
}

.enable-toggle small {
  display: block;
  margin-top: 1px;
  font-size: 10.5px;
  color: var(--faint);
}

.strategy-section {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
}

.strategy-section-head {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 12px;
  color: var(--ink-soft);
}

.strategy-section-head h3 {
  font-size: 12.5px;
  font-weight: 600;
}

.strategy-section-head span {
  margin-left: auto;
  font-size: 10.5px;
  color: var(--faint);
}

.feature-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.custom-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 10px;
}

.custom-row {
  display: grid;
  grid-template-columns: 1.15fr 1fr 28px;
  gap: 8px;
  align-items: center;
}

.custom-row .input {
  font-family: var(--font-mono);
  font-size: 11.5px;
}

@media (max-width: 640px) {
  .strategy-form-grid,
  .feature-grid {
    grid-template-columns: 1fr;
  }
}
</style>
