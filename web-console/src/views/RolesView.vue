<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Pencil,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  UserCog,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime } from '../lib/format'
import { clearSession, session } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const CATEGORY_LABELS = {
  device: '设备',
  user: '用户',
  group: '设备分组',
  address_book: '地址簿',
  strategy: '策略',
  audit: '日志与审计',
  deployment: '客户端部署',
  recording: '会话录像',
  relay: '中继服务器',
  settings: '系统设置',
  role: '管理员角色',
}

const CATEGORY_ORDER = [
  'device',
  'user',
  'group',
  'address_book',
  'strategy',
  'audit',
  'deployment',
  'recording',
  'relay',
  'settings',
  'role',
]

const roles = ref([])
const permissions = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const isAdmin = computed(() => session.user?.is_admin === true)

const modalOpen = ref(false)
const editing = ref(null)
const form = ref({ name: '', note: '', permissions: [] })
const modalBusy = ref(false)
const modalError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)

const permissionLabels = computed(() => {
  const map = {}
  for (const item of permissions.value) map[item.key] = item.label
  return map
})

const permissionGroups = computed(() => {
  const groups = {}
  for (const item of permissions.value) {
    const category = item.key.split('.')[0]
    if (!groups[category]) groups[category] = []
    groups[category].push(item)
  }
  return CATEGORY_ORDER.filter((category) => groups[category]).map(
    (category) => ({
      key: category,
      label: CATEGORY_LABELS[category] || category,
      items: groups[category],
    })
  )
})

const stats = computed(() => ({
  total: roles.value.length,
  assigned: roles.value.reduce(
    (sum, role) => sum + Number(role.user_count || 0),
    0
  ),
  permissions: permissions.value.length,
}))

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [roleRows, permissionRows] = await Promise.all([
      api.getRoles(),
      api.getPermissions(),
    ])
    roles.value = roleRows
    permissions.value = permissionRows
  } catch (e) {
    error.value = e.message || '角色列表加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function openCreate() {
  editing.value = null
  form.value = { name: '', note: '', permissions: [] }
  modalError.value = ''
  modalOpen.value = true
}

function openEdit(role) {
  editing.value = role
  form.value = {
    name: role.name || '',
    note: role.note || '',
    permissions: Array.isArray(role.permissions)
      ? [...role.permissions]
      : [],
  }
  modalError.value = ''
  modalOpen.value = true
}

function closeModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

function togglePermission(key, checked) {
  const current = new Set(form.value.permissions)
  if (checked) current.add(key)
  else current.delete(key)
  form.value.permissions = [...current]
}

function selectAll() {
  form.value.permissions = permissions.value.map((item) => item.key)
}

function clearAll() {
  form.value.permissions = []
}

async function submitRole() {
  modalBusy.value = true
  modalError.value = ''
  const payload = {
    name: form.value.name.trim(),
    note: form.value.note.trim(),
    permissions: [...form.value.permissions].sort(),
  }
  try {
    if (editing.value) {
      await api.updateRole(editing.value.guid, payload)
      notify('管理员角色已更新。')
    } else {
      await api.createRole(payload)
      notify('管理员角色已创建。')
    }
    modalOpen.value = false
    await load(false)
  } catch (e) {
    modalError.value = e.message || '角色保存失败。'
  } finally {
    modalBusy.value = false
  }
}

async function removeRole() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteRole(deleteTarget.value.guid)
    notify('管理员角色已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '角色删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

function permissionText(key) {
  return permissionLabels.value[key] || key
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">账号与权限</div>
          <h1 class="page-title">管理员角色</h1>
          <p class="page-sub">
            按职责拆分管理员权限，例如只读审计员、设备管理员或部署运维。
          </p>
        </div>
        <div class="page-actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="refreshing"
            @click="load(false)"
          >
            <RefreshCw
              :size="15"
              :style="
                refreshing ? { animation: 'spin .8s linear infinite' } : null
              "
            />
            刷新
          </button>
          <button
            v-if="isAdmin"
            class="btn btn-primary"
            type="button"
            @click="openCreate"
          >
            <Plus :size="15" />
            新建角色
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">角色总数</span>
            <ShieldCheck :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">可分配给控制台账号</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已分配成员</span>
            <UserCog :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.assigned }}</div>
          <div class="stat-note">所有角色的成员关系数</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">可授权限</span>
            <ShieldCheck :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.permissions }}</div>
          <div class="stat-note">系统定义的权限项</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载管理员角色
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载管理员角色</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else-if="!roles.length" class="panel">
        <div class="empty">
          <div class="empty-icon"><ShieldCheck :size="18" /></div>
          <h3>还没有管理员角色</h3>
          <p>
            创建角色并勾选权限后，可在用户编辑页中将角色分配给账号。系统管理员始终拥有全部权限。
          </p>
          <button
            v-if="isAdmin"
            class="btn btn-primary"
            type="button"
            @click="openCreate"
          >
            <Plus :size="15" />
            新建角色
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="table-wrap">
          <table class="table roles-table">
            <thead>
              <tr>
                <th>角色</th>
                <th>权限</th>
                <th style="width: 100px">成员数</th>
                <th style="width: 158px">更新时间</th>
                <th style="width: 88px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="role in roles" :key="role.guid">
                <td>
                  <div class="role-name">{{ role.name }}</div>
                  <div v-if="role.note" class="role-note">{{ role.note }}</div>
                </td>
                <td>
                  <div v-if="role.permissions?.length" class="role-perms">
                    <span
                      v-for="key in role.permissions.slice(0, 6)"
                      :key="key"
                      class="tag tag-blue"
                    >
                      {{ permissionText(key) }}
                    </span>
                    <span
                      v-if="role.permissions.length > 6"
                      class="tag"
                    >
                      +{{ role.permissions.length - 6 }}
                    </span>
                  </div>
                  <span v-else class="faint" style="font-size: 11.5px">
                    未授予任何权限
                  </span>
                </td>
                <td class="role-count">{{ role.user_count || 0 }}</td>
                <td class="role-time">{{ exactTime(role.updated_at) }}</td>
                <td>
                  <div v-if="isAdmin" class="row-actions">
                    <button
                      class="icon-btn"
                      type="button"
                      title="编辑角色"
                      @click="openEdit(role)"
                    >
                      <Pencil :size="14" />
                    </button>
                    <button
                      class="icon-btn danger"
                      type="button"
                      title="删除角色"
                      @click="deleteTarget = role"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="role-cards">
          <article
            v-for="role in roles"
            :key="`card-${role.guid}`"
            class="role-card"
          >
            <div class="role-card-head">
              <div class="role-card-title">
                <div class="role-name">{{ role.name }}</div>
                <div v-if="role.note" class="role-note">{{ role.note }}</div>
              </div>
              <span class="tag">{{ role.user_count || 0 }} 名成员</span>
            </div>
            <div v-if="role.permissions?.length" class="role-perms">
              <span
                v-for="key in role.permissions.slice(0, 6)"
                :key="key"
                class="tag tag-blue"
              >
                {{ permissionText(key) }}
              </span>
              <span v-if="role.permissions.length > 6" class="tag">
                +{{ role.permissions.length - 6 }}
              </span>
            </div>
            <span v-else class="faint" style="font-size: 11.5px">
              未授予任何权限
            </span>
            <div class="role-card-time">
              更新于 {{ exactTime(role.updated_at) }}
            </div>
            <div v-if="isAdmin" class="role-card-actions">
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="openEdit(role)"
              >
                <Pencil :size="13" />
                编辑
              </button>
              <button
                class="btn btn-ghost btn-sm role-card-delete"
                type="button"
                @click="deleteTarget = role"
              >
                <Trash2 :size="13" />
                删除
              </button>
            </div>
          </article>
        </div>
      </div>
    </main>

    <div v-if="modalOpen" class="modal-scrim" @click.self="closeModal">
      <form
        class="modal role-modal"
        role="dialog"
        aria-modal="true"
        @submit.prevent="submitRole"
      >
        <h2 class="modal-title">
          {{ editing ? '编辑管理员角色' : '新建管理员角色' }}
        </h2>
        <p class="modal-sub">
          勾选该角色可以执行的操作。系统管理员不受角色限制，始终拥有全部权限。
        </p>
        <div class="modal-body">
          <div class="field">
            <label class="label" for="role-name">角色名称</label>
            <input
              id="role-name"
              v-model="form.name"
              class="input"
              maxlength="64"
              placeholder="例如：审计只读"
              autofocus
            />
          </div>
          <div class="field">
            <label class="label" for="role-note">
              说明 <span class="hint">可选</span>
            </label>
            <input
              id="role-note"
              v-model="form.note"
              class="input"
              maxlength="120"
              placeholder="这个角色适用于哪些成员"
            />
          </div>

          <div class="perm-head">
            <span class="label" style="margin: 0">权限</span>
            <div class="perm-actions">
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="selectAll"
              >
                全选
              </button>
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="clearAll"
              >
                清空
              </button>
              <span class="faint" style="font-size: 11px">
                已选 {{ form.permissions.length }} 项
              </span>
            </div>
          </div>

          <div class="perm-groups">
            <div
              v-for="group in permissionGroups"
              :key="group.key"
              class="perm-group"
            >
              <div class="perm-group-label">{{ group.label }}</div>
              <label
                v-for="item in group.items"
                :key="item.key"
                class="perm-item"
              >
                <input
                  type="checkbox"
                  :checked="form.permissions.includes(item.key)"
                  @change="
                    togglePermission(item.key, $event.target.checked)
                  "
                />
                <span>
                  {{ item.label }}
                  <small>{{ item.key }}</small>
                </span>
              </label>
            </div>
          </div>

          <p v-if="modalError" class="field-error" style="margin-top: 10px">
            {{ modalError }}
          </p>
        </div>
        <div class="modal-actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="modalBusy"
            @click="closeModal"
          >
            取消
          </button>
          <button
            class="btn btn-primary"
            type="submit"
            :disabled="modalBusy || !form.name.trim()"
          >
            <span v-if="modalBusy" class="spinner" />
            {{ modalBusy ? '保存中…' : editing ? '保存角色' : '创建角色' }}
          </button>
        </div>
      </form>
    </div>

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除这个角色？"
      :message="`「${deleteTarget.name}」将被删除，已分配该角色的 ${deleteTarget.user_count || 0} 个账号会立即失去对应权限。`"
      confirm-label="删除角色"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="removeRole"
    />
  </AppShell>
</template>

<style scoped>
.roles-table {
  min-width: 880px;
}

.roles-table tbody tr {
  cursor: default;
}

.role-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.role-note {
  margin-top: 2px;
  font-size: 11px;
  color: var(--faint);
}

.role-perms {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-blue {
  background: var(--blue-wash);
  box-shadow: none;
  color: #0068d6;
}

.role-count {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--ink);
}

.role-time {
  font-size: 11.5px;
  color: var(--muted);
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.role-modal {
  max-width: 720px;
  width: min(720px, calc(100vw - 32px));
}

.perm-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 16px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}

.perm-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.perm-groups {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  margin-top: 10px;
}

.perm-group {
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--canvas);
}

.perm-group-label {
  margin-bottom: 6px;
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--faint);
}

.perm-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
  color: var(--ink-soft);
  cursor: pointer;
}

.perm-item input {
  margin-top: 2px;
  accent-color: var(--ink);
}

.perm-item small {
  display: block;
  margin-top: 1px;
  font-family: var(--font-mono);
  font-size: 9.5px;
  color: var(--faint);
}

.role-cards {
  display: none;
  flex-direction: column;
}

.role-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.role-card:last-child {
  border-bottom: 0;
}

.role-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 9px;
}

.role-card-title {
  min-width: 0;
}

.role-card .role-perms {
  gap: 5px;
}

.role-card-time {
  margin-top: 9px;
  font-size: 11px;
  color: var(--faint);
}

.role-card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.role-card-delete {
  margin-left: auto;
  color: var(--red);
}

@media (max-width: 960px) {
  .role-cards {
    display: flex;
  }
}

@media (max-width: 720px) {
  .perm-groups {
    grid-template-columns: 1fr;
  }
}
</style>
