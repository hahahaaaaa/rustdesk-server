<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  FolderLock,
  KeyRound,
  Pencil,
  RefreshCw,
  ShieldCheck,
  Trash2,
  UserPlus,
  Users,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import UserAccessModal from '../components/UserAccessModal.vue'
import UserModal from '../components/UserModal.vue'
import { getApi } from '../lib/api'
import { can, clearSession, session } from '../lib/session'
import { initials } from '../lib/format'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const users = ref([])
const groups = ref([])
const roles = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')

const modalOpen = ref(false)
const editingUser = ref(null)
const modalBusy = ref(false)
const modalError = ref('')

const accessTarget = ref(null)
const accessSelected = ref([])
const accessBusy = ref(false)
const accessError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)

const tfaResetTarget = ref(null)
const tfaResetBusy = ref(false)

const rows = computed(() =>
  [...users.value].sort((a, b) => {
    if (a.is_admin !== b.is_admin) return a.is_admin ? -1 : 1
    return String(a.name).localeCompare(String(b.name))
  })
)

const canAssignRoles = computed(() => session.user?.is_admin === true)

function canManageTarget(user) {
  return canAssignRoles.value || !user.is_admin
}

const stats = computed(() => ({
  total: users.value.length,
  admins: users.value.filter((user) => user.is_admin).length,
  disabled: users.value.filter((user) => user.status !== 1).length,
}))

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [userRows, groupRows] = await Promise.all([
      api.getUsers(),
      api.getGroups(),
    ])
    users.value = userRows
    groups.value = groupRows
    if (can('role.manage')) {
      api
        .getRoles()
        .then((roleRows) => {
          roles.value = roleRows
        })
        .catch(() => {})
    }
  } catch (e) {
    error.value = e.message || '用户列表加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function isSelf(user) {
  return Boolean(session.user?.name) && user.name === session.user.name
}

function displayName(user) {
  return user.display_name || user.name
}

function userRoles(user) {
  return Array.isArray(user.roles) ? user.roles : []
}

function openCreate() {
  editingUser.value = null
  modalError.value = ''
  modalOpen.value = true
}

function openEdit(user) {
  editingUser.value = user
  modalError.value = ''
  modalOpen.value = true
}

function closeModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

async function submitUser(payload) {
  modalBusy.value = true
  modalError.value = ''
  try {
    if (editingUser.value) {
      await api.updateUser(editingUser.value.id, payload)
      notify('用户资料已更新。')
    } else {
      await api.createUser(payload)
      notify('用户已创建。')
    }
    modalOpen.value = false
    await load(false)
  } catch (e) {
    modalError.value = e.message || '用户保存失败。'
  } finally {
    modalBusy.value = false
  }
}

async function openAccess(user) {
  accessTarget.value = user
  accessSelected.value = []
  accessError.value = ''
  accessBusy.value = true
  try {
    accessSelected.value = await api.getUserGroups(user.id)
  } catch (e) {
    accessError.value = e.message || '分组授权加载失败。'
  } finally {
    accessBusy.value = false
  }
}

function closeAccess() {
  if (accessBusy.value) return
  accessTarget.value = null
}

async function submitAccess(selected) {
  if (!accessTarget.value) return
  accessBusy.value = true
  accessError.value = ''
  try {
    await api.setUserGroups(accessTarget.value.id, selected)
    notify('设备分组授权已保存。')
    accessTarget.value = null
  } catch (e) {
    accessError.value = e.message || '分组授权保存失败。'
  } finally {
    accessBusy.value = false
  }
}

async function deleteUser() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteUser(deleteTarget.value.id)
    notify('用户已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '用户删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

async function resetUserTfa() {
  if (!tfaResetTarget.value) return
  tfaResetBusy.value = true
  try {
    await api.resetUserTfa(tfaResetTarget.value.id)
    notify('双重认证已重置，用户下次登录无需动态验证码。')
    tfaResetTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '重置双重认证失败。', 'error')
  } finally {
    tfaResetBusy.value = false
  }
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">账号与权限</div>
          <h1 class="page-title">用户</h1>
          <p class="page-sub">
            管理登录账号、管理员角色，以及每个用户可以访问的设备分组。
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
          <button class="btn btn-primary" type="button" @click="openCreate">
            <UserPlus :size="15" />
            新建用户
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">用户总数</span>
            <Users :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">可以登录的账号</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">管理员</span>
            <ShieldCheck :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.admins }}</div>
          <div class="stat-note">拥有完整管理权限</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已停用</span>
            <KeyRound :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.disabled }}</div>
          <div class="stat-note">当前无法登录</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载用户
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法连接服务器</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else-if="!rows.length" class="panel">
        <div class="empty">
          <div class="empty-icon"><Users :size="18" /></div>
          <h3>暂无用户</h3>
          <p>创建第一个账号，并为其分配设备分组。</p>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <UserPlus :size="15" />
            新建用户
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="table-wrap">
          <table class="table users-table">
            <thead>
              <tr>
                <th>用户</th>
                <th>邮箱</th>
                <th>角色</th>
                <th>状态</th>
                <th style="width: 132px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="user in rows" :key="user.id">
                <td>
                  <div class="user-cell">
                    <div class="avatar">{{ initials(displayName(user)) }}</div>
                    <div class="user-cell-meta">
                      <div class="user-cell-name">
                        {{ displayName(user) }}
                        <span v-if="isSelf(user)" class="tag">当前账号</span>
                      </div>
                      <div class="user-cell-handle">@{{ user.name }}</div>
                    </div>
                  </div>
                </td>
                <td>{{ user.email || '—' }}</td>
                <td>
                  <div class="role-tags">
                    <span v-if="user.is_admin" class="tag tag-blue">管理员</span>
                    <span
                      v-for="role in userRoles(user)"
                      :key="role"
                      class="tag"
                    >
                      {{ role }}
                    </span>
                    <span
                      v-if="!user.is_admin && !userRoles(user).length"
                      class="tag"
                    >
                      普通用户
                    </span>
                    <span v-if="user.tfa_enabled" class="tag tag-tfa">
                      双重认证
                    </span>
                  </div>
                </td>
                <td>
                  <span
                    class="pill"
                    :class="user.status === 1 ? 'pill-online' : 'pill-disabled'"
                  >
                    <span class="dot" />
                    {{ user.status === 1 ? '正常' : '已停用' }}
                  </span>
                </td>
                <td>
                  <div class="row-actions">
                    <button
                      v-if="canManageTarget(user)"
                      class="icon-btn"
                      type="button"
                      title="编辑用户"
                      @click="openEdit(user)"
                    >
                      <Pencil :size="14" />
                    </button>
                    <button
                      v-if="canManageTarget(user)"
                      class="icon-btn"
                      type="button"
                      title="分组授权"
                      @click="openAccess(user)"
                    >
                      <FolderLock :size="14" />
                    </button>
                    <button
                      v-if="user.tfa_enabled && !isSelf(user) && canAssignRoles"
                      class="icon-btn"
                      type="button"
                      title="重置双重认证"
                      @click="tfaResetTarget = user"
                    >
                      <KeyRound :size="14" />
                    </button>
                    <button
                      v-if="canManageTarget(user)"
                      class="icon-btn danger"
                      type="button"
                      title="删除用户"
                      :disabled="isSelf(user)"
                      @click="deleteTarget = user"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="user-cards">
          <article v-for="user in rows" :key="user.id" class="user-card">
            <div class="user-card-head">
              <div class="avatar">{{ initials(displayName(user)) }}</div>
              <div class="user-cell-meta">
                <div class="user-cell-name">
                  {{ displayName(user) }}
                  <span v-if="isSelf(user)" class="tag">当前账号</span>
                </div>
                <div class="user-cell-handle">@{{ user.name }}</div>
              </div>
              <span
                class="pill"
                :class="user.status === 1 ? 'pill-online' : 'pill-disabled'"
              >
                <span class="dot" />
                {{ user.status === 1 ? '正常' : '已停用' }}
              </span>
            </div>
            <div class="user-card-meta">
              <span v-if="user.is_admin" class="tag tag-blue">管理员</span>
              <span v-for="role in userRoles(user)" :key="role" class="tag">
                {{ role }}
              </span>
              <span
                v-if="!user.is_admin && !userRoles(user).length"
                class="tag"
              >
                普通用户
              </span>
              <span v-if="user.tfa_enabled" class="tag tag-tfa">双重认证</span>
              <span class="user-card-email">{{ user.email || '未填写邮箱' }}</span>
            </div>
            <div class="user-card-actions">
              <button
                v-if="canManageTarget(user)"
                class="row-link"
                type="button"
                @click="openEdit(user)"
              >
                <Pencil :size="13" />
                编辑
              </button>
              <button
                v-if="canManageTarget(user)"
                class="row-link"
                type="button"
                @click="openAccess(user)"
              >
                <FolderLock :size="13" />
                分组授权
              </button>
              <button
                v-if="user.tfa_enabled && !isSelf(user) && canAssignRoles"
                class="row-link"
                type="button"
                @click="tfaResetTarget = user"
              >
                <KeyRound :size="13" />
                重置 2FA
              </button>
              <button
                v-if="canManageTarget(user)"
                class="icon-btn danger user-card-delete"
                type="button"
                title="删除用户"
                :disabled="isSelf(user)"
                @click="deleteTarget = user"
              >
                <Trash2 :size="14" />
              </button>
            </div>
          </article>
        </div>
      </div>
    </main>

    <UserModal
      v-if="modalOpen"
      :user="editingUser"
      :roles="canAssignRoles ? roles : []"
      :can-assign-roles="canAssignRoles"
      :self="editingUser ? isSelf(editingUser) : false"
      :busy="modalBusy"
      :error="modalError"
      @cancel="closeModal"
      @submit="submitUser"
    />

    <UserAccessModal
      v-if="accessTarget"
      :user="accessTarget"
      :groups="groups"
      :selected="accessSelected"
      :busy="accessBusy"
      :error="accessError"
      @cancel="closeAccess"
      @submit="submitAccess"
    />

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除此用户？"
      :message="`${displayName(deleteTarget)} 将被删除，其设备会变为未分配，分组授权同时移除。`"
      confirm-label="删除用户"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="deleteUser"
    />

    <ConfirmDialog
      v-if="tfaResetTarget"
      title="重置双重认证？"
      :message="`${displayName(tfaResetTarget)} 的双重认证配置将被清除，下次登录只需输入密码，可重新在设置中启用。`"
      confirm-label="重置双重认证"
      danger
      :busy="tfaResetBusy"
      @cancel="tfaResetTarget = null"
      @confirm="resetUserTfa"
    />
  </AppShell>
</template>

<style scoped>
.users-table {
  min-width: 720px;
}

.user-cell {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar {
  display: grid;
  place-items: center;
  flex: none;
  width: 30px;
  height: 30px;
  border-radius: 9999px;
  background: var(--ink);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
}

.user-cell-meta {
  min-width: 0;
}

.user-cell-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.user-cell-handle {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--faint);
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.role-tags {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  max-width: 260px;
}

.tag-tfa {
  background: var(--green-wash);
  box-shadow: none;
  color: var(--green);
}

.icon-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.user-cards {
  display: none;
  flex-direction: column;
}

.user-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.user-card:last-child {
  border-bottom: 0;
}

.user-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.user-card-head .user-cell-meta {
  flex: 1;
}

.user-card-head .user-cell-name {
  flex-wrap: wrap;
}

.user-card-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
}

.user-card-email {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: var(--faint);
}

.user-card-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.user-card-delete {
  margin-left: auto;
}

.row-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 500;
}

.row-link:hover {
  background: var(--wash);
  color: var(--ink);
}

@media (max-width: 960px) {
  .user-cards {
    display: flex;
  }
}
</style>
