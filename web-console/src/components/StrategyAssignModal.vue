<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  FolderTree,
  MonitorSmartphone,
  Search,
  Users,
} from 'lucide-vue-next'

const props = defineProps({
  strategy: { type: Object, required: true },
  devices: { type: Array, default: () => [] },
  users: { type: Array, default: () => [] },
  groups: { type: Array, default: () => [] },
  assignments: { type: Object, default: () => ({}) },
  busy: { type: Boolean, default: false },
  error: { type: String, default: '' },
})

const emit = defineEmits(['cancel', 'submit'])

const active = ref('devices')
const search = ref('')
const selectedDevices = ref(
  new Set((props.assignments?.devices || []).map((device) => device.guid))
)
const selectedUsers = ref(
  new Set((props.assignments?.users || []).map((user) => user.name))
)
const selectedGroups = ref(
  new Set((props.assignments?.groups || []).map((group) => group.guid))
)

const term = computed(() => search.value.trim().toLowerCase())

const tabs = computed(() => [
  {
    key: 'devices',
    label: '设备',
    icon: MonitorSmartphone,
    selected: selectedDevices.value.size,
    total: props.devices.length,
  },
  {
    key: 'users',
    label: '用户',
    icon: Users,
    selected: selectedUsers.value.size,
    total: props.users.length,
  },
  {
    key: 'groups',
    label: '设备分组',
    icon: FolderTree,
    selected: selectedGroups.value.size,
    total: props.groups.length,
  },
])

const filteredDevices = computed(() =>
  props.devices.filter((device) =>
    [device.device_name, device.id, device.device_group_name, device.note]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(term.value)
  )
)

const filteredUsers = computed(() =>
  props.users.filter((user) =>
    [user.display_name, user.name, user.email]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(term.value)
  )
)

const filteredGroups = computed(() =>
  props.groups.filter((group) =>
    [group.name, group.note]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(term.value)
  )
)

const selectedTotal = computed(
  () =>
    selectedDevices.value.size +
    selectedUsers.value.size +
    selectedGroups.value.size
)

const placeholder = computed(() => {
  if (active.value === 'users') return '搜索姓名、账号或邮箱'
  if (active.value === 'groups') return '搜索分组名称或备注'
  return '搜索设备名称、ID 或分组'
})

function toggleDevice(guid) {
  const next = new Set(selectedDevices.value)
  if (next.has(guid)) next.delete(guid)
  else next.add(guid)
  selectedDevices.value = next
}

function toggleUser(name) {
  const next = new Set(selectedUsers.value)
  if (next.has(name)) next.delete(name)
  else next.add(name)
  selectedUsers.value = next
}

function toggleGroup(guid) {
  const next = new Set(selectedGroups.value)
  if (next.has(guid)) next.delete(guid)
  else next.add(guid)
  selectedGroups.value = next
}

function groupDeviceCount(group) {
  if (Number.isFinite(Number(group.device_count))) {
    return Number(group.device_count)
  }
  return props.devices.filter(
    (device) => device.device_group_name === group.name
  ).length
}

function submit() {
  if (props.busy) return
  emit('submit', {
    devices: [...selectedDevices.value],
    users: [...selectedUsers.value],
    groups: [...selectedGroups.value],
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
      class="modal assign-modal"
      role="dialog"
      aria-modal="true"
      @submit.prevent="submit"
    >
      <h2 class="modal-title">分配策略 · {{ strategy.name }}</h2>
      <p class="modal-sub">
        勾选需要应用此策略的设备、用户或分组。取消勾选会解除对应对象的策略绑定。
      </p>
      <div class="modal-body">
        <div class="assign-tabs" role="tablist">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="assign-tab"
            :class="{ active: active === tab.key }"
            type="button"
            role="tab"
            :aria-selected="active === tab.key"
            @click="active = tab.key; search = ''"
          >
            <component :is="tab.icon" :size="14" />
            {{ tab.label }}
            <span class="assign-tab-count">{{ tab.selected }}/{{ tab.total }}</span>
          </button>
        </div>

        <div class="search assign-search">
          <Search :size="14" />
          <input
            v-model="search"
            class="input"
            type="search"
            :placeholder="placeholder"
            :aria-label="placeholder"
          />
        </div>

        <div class="assign-list">
          <template v-if="active === 'devices'">
            <div v-if="!filteredDevices.length" class="assign-empty">
              没有匹配的设备。
            </div>
            <label
              v-for="device in filteredDevices"
              :key="device.guid"
              class="assign-option"
            >
              <input
                type="checkbox"
                :checked="selectedDevices.has(device.guid)"
                @change="toggleDevice(device.guid)"
              />
              <span class="assign-mark"><MonitorSmartphone :size="14" /></span>
              <span class="assign-body">
                <strong>{{ device.device_name || device.id }}</strong>
                <small>
                  {{ device.id }}
                  <template v-if="device.device_group_name">
                    · {{ device.device_group_name }}
                  </template>
                </small>
              </span>
              <span
                v-if="device.disabled"
                class="tag"
                style="font-size: 10px"
              >
                已禁用
              </span>
            </label>
          </template>

          <template v-else-if="active === 'users'">
            <div v-if="!filteredUsers.length" class="assign-empty">
              没有匹配的用户。
            </div>
            <label
              v-for="user in filteredUsers"
              :key="user.name"
              class="assign-option"
            >
              <input
                type="checkbox"
                :checked="selectedUsers.has(user.name)"
                @change="toggleUser(user.name)"
              />
              <span class="assign-mark"><Users :size="14" /></span>
              <span class="assign-body">
                <strong>{{ user.display_name || user.name }}</strong>
                <small>
                  @{{ user.name }}
                  <template v-if="user.email"> · {{ user.email }}</template>
                </small>
              </span>
              <span v-if="user.is_admin" class="tag" style="font-size: 10px">
                管理员
              </span>
            </label>
          </template>

          <template v-else>
            <div v-if="!filteredGroups.length" class="assign-empty">
              没有匹配的设备分组。
            </div>
            <label
              v-for="group in filteredGroups"
              :key="group.guid"
              class="assign-option"
            >
              <input
                type="checkbox"
                :checked="selectedGroups.has(group.guid)"
                @change="toggleGroup(group.guid)"
              />
              <span class="assign-mark"><FolderTree :size="14" /></span>
              <span class="assign-body">
                <strong>{{ group.name }}</strong>
                <small>
                  {{ groupDeviceCount(group) }} 台设备
                  <template v-if="group.note"> · {{ group.note }}</template>
                </small>
              </span>
            </label>
          </template>
        </div>

        <p v-if="error" class="field-error" style="margin-top: 10px">
          {{ error }}
        </p>
      </div>
      <div class="modal-actions">
        <span class="assign-total">已选择 {{ selectedTotal }} 个对象</span>
        <button
          type="button"
          class="btn btn-secondary"
          :disabled="busy"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button type="submit" class="btn btn-primary" :disabled="busy">
          <span v-if="busy" class="spinner" />
          {{ busy ? '下发中…' : '保存分配' }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.assign-modal {
  width: min(620px, 100%);
}

.assign-tabs {
  display: flex;
  gap: 6px;
  padding: 3px;
  border-radius: var(--radius-sm);
  background: var(--wash);
}

.assign-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  justify-content: center;
  height: 32px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  font-weight: 500;
}

.assign-tab.active {
  background: #fff;
  color: var(--ink);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
}

.assign-tab-count {
  padding: 1px 5px;
  border-radius: 9999px;
  background: var(--wash);
  font-family: var(--font-mono);
  font-size: 9.5px;
  color: var(--faint);
}

.assign-tab.active .assign-tab-count {
  background: rgba(0, 0, 0, 0.06);
  color: var(--ink-soft);
}

.assign-search {
  margin-top: 12px;
}

.assign-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  max-height: 300px;
  overflow-y: auto;
  margin-top: 10px;
  padding: 2px;
}

.assign-option {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  box-shadow: inset 0 0 0 1px var(--line);
  cursor: pointer;
}

.assign-option:hover {
  background: var(--wash);
}

.assign-option input {
  accent-color: var(--ink);
}

.assign-mark {
  display: grid;
  place-items: center;
  flex: none;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: var(--wash);
  color: var(--ink-soft);
}

.assign-body {
  min-width: 0;
  flex: 1;
}

.assign-body strong {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  font-weight: 500;
}

.assign-body small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 1px;
  font-size: 10.5px;
  color: var(--faint);
}

.assign-empty {
  padding: 24px 12px;
  text-align: center;
  font-size: 12px;
  color: var(--muted);
}

.assign-total {
  margin-right: auto;
  font-size: 11.5px;
  color: var(--muted);
}
</style>
