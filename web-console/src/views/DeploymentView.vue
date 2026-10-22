<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Copy,
  Eye,
  EyeOff,
  KeyRound,
  Plus,
  Power,
  RefreshCw,
  Rocket,
  Terminal,
  Trash2,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime, timeAgo } from '../lib/format'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const tokens = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const revealed = ref({})

const createOpen = ref(false)
const createName = ref('')
const createBusy = ref(false)
const createError = ref('')

const commandToken = ref(null)
const deleteTarget = ref(null)
const deleteBusy = ref(false)

const stats = computed(() => ({
  total: tokens.value.length,
  enabled: tokens.value.filter((token) => token.enabled).length,
  used: tokens.value.filter((token) => token.last_used_at).length,
}))

const commands = computed(() => {
  const token = commandToken.value?.token || '<部署令牌>'
  return [
    {
      label: '部署到当前设备',
      note: '在已安装 RustDesk 的设备上以管理员身份运行，使用设备当前的 ID。',
      value: `rustdesk --deploy --token ${token}`,
    },
    {
      label: '使用指定 ID 部署',
      note: '将 <设备ID> 替换为需要预注册的 RustDesk ID，适合批量装机。',
      value: `rustdesk --deploy --token ${token} --id <设备ID>`,
    },
    {
      label: '分配设备归属',
      note: '将 <用户名> 替换为控制台中的账号，可同时追加 --strategy_name、--device_group_name 等参数。',
      value: `rustdesk --assign --token ${token} --user_name <用户名>`,
    },
  ]
})

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    tokens.value = await api.getDeploymentTokens()
  } catch (e) {
    error.value = e.message || '部署令牌加载失败。'
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
  createName.value = ''
  createError.value = ''
  createOpen.value = true
}

function closeCreate() {
  if (createBusy.value) return
  createOpen.value = false
}

async function createToken() {
  createBusy.value = true
  createError.value = ''
  try {
    const created = await api.createDeploymentToken(createName.value.trim())
    createOpen.value = false
    if (created?.token) {
      revealed.value[created.id] = true
      const copied = await copy(created.token, false)
      notify(
        copied
          ? '部署令牌已创建，令牌内容已复制到剪贴板。'
          : '部署令牌已创建，请点击显示按钮后手动复制。'
      )
    } else {
      notify('部署令牌已创建。')
    }
    await load(false)
  } catch (e) {
    createError.value = e.message || '部署令牌创建失败。'
  } finally {
    createBusy.value = false
  }
}

async function toggleToken(token) {
  try {
    await api.updateDeploymentToken(token.id, { enabled: !token.enabled })
    token.enabled = !token.enabled
    notify(token.enabled ? '部署令牌已启用。' : '部署令牌已停用。')
  } catch (e) {
    notify(e.message || '部署令牌更新失败。', 'error')
  }
}

async function removeToken() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteDeploymentToken(deleteTarget.value.id)
    notify('部署令牌已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '部署令牌删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

function maskToken(value) {
  const text = String(value || '')
  if (text.length <= 10) return '••••••••'
  return `${text.slice(0, 6)}••••••${text.slice(-4)}`
}

async function copy(text, notifyUser = true) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
    } else {
      const area = document.createElement('textarea')
      area.value = text
      area.style.position = 'fixed'
      area.style.opacity = '0'
      document.body.appendChild(area)
      area.select()
      document.execCommand('copy')
      document.body.removeChild(area)
    }
    if (notifyUser) notify('已复制到剪贴板。')
    return true
  } catch {
    if (notifyUser) notify('复制失败，请手动选择文本。', 'error')
    return false
  }
}

function openCommands(token) {
  commandToken.value = token
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">运维</div>
          <h1 class="page-title">客户端部署</h1>
          <p class="page-sub">
            使用部署令牌预注册设备、分配用户与分组，适合批量装机与自动化交付。
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
            <Plus :size="15" />
            新建令牌
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">部署令牌</span>
            <KeyRound :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-note">用于客户端命令行部署</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已启用</span>
            <Power :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.enabled }}</div>
          <div class="stat-note">当前可以使用的令牌</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">已使用</span>
            <Rocket :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.used }}</div>
          <div class="stat-note">至少执行过一次部署</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载部署令牌
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载部署令牌</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else-if="!tokens.length" class="panel">
        <div class="empty">
          <div class="empty-icon"><Rocket :size="18" /></div>
          <h3>还没有部署令牌</h3>
          <p>
            创建令牌后，可在设备上运行
            <code>rustdesk --deploy --token …</code>
            完成预注册。
          </p>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <Plus :size="15" />
            新建令牌
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="table-wrap">
          <table class="table deploy-table">
            <thead>
              <tr>
                <th>名称</th>
                <th>令牌</th>
                <th style="width: 118px">状态</th>
                <th style="width: 160px">创建时间</th>
                <th style="width: 150px">最近使用</th>
                <th style="width: 150px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="token in tokens" :key="token.id">
                <td>
                  <div class="token-name">{{ token.name || '未命名令牌' }}</div>
                  <div class="token-id">#{{ token.id }}</div>
                </td>
                <td>
                  <div class="token-value">
                    <code>{{
                      revealed[token.id] ? token.token : maskToken(token.token)
                    }}</code>
                    <button
                      class="icon-btn"
                      type="button"
                      :title="revealed[token.id] ? '隐藏令牌' : '显示令牌'"
                      @click="revealed[token.id] = !revealed[token.id]"
                    >
                      <EyeOff v-if="revealed[token.id]" :size="14" />
                      <Eye v-else :size="14" />
                    </button>
                    <button
                      class="icon-btn"
                      type="button"
                      title="复制令牌"
                      @click="copy(token.token)"
                    >
                      <Copy :size="14" />
                    </button>
                  </div>
                </td>
                <td>
                  <span
                    class="pill"
                    :class="token.enabled ? 'pill-online' : 'pill-disabled'"
                  >
                    <span class="dot" />
                    {{ token.enabled ? '已启用' : '已停用' }}
                  </span>
                </td>
                <td class="deploy-time">{{ exactTime(token.created_at) }}</td>
                <td class="deploy-time">
                  {{ token.last_used_at ? timeAgo(token.last_used_at) : '从未' }}
                </td>
                <td>
                  <div class="row-actions">
                    <button
                      class="btn btn-ghost btn-sm"
                      type="button"
                      title="查看部署命令"
                      @click="openCommands(token)"
                    >
                      <Terminal :size="13" />
                      命令
                    </button>
                    <button
                      class="icon-btn"
                      type="button"
                      :title="token.enabled ? '停用令牌' : '启用令牌'"
                      @click="toggleToken(token)"
                    >
                      <Power :size="14" />
                    </button>
                    <button
                      class="icon-btn danger"
                      type="button"
                      title="删除令牌"
                      @click="deleteTarget = token"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="deploy-cards">
          <article
            v-for="token in tokens"
            :key="`card-${token.id}`"
            class="deploy-card"
          >
            <div class="deploy-card-head">
              <div>
                <div class="token-name">{{ token.name || '未命名令牌' }}</div>
                <div class="token-id">#{{ token.id }}</div>
              </div>
              <span
                class="pill"
                :class="token.enabled ? 'pill-online' : 'pill-disabled'"
              >
                <span class="dot" />
                {{ token.enabled ? '已启用' : '已停用' }}
              </span>
            </div>
            <div class="token-value">
              <code>{{
                revealed[token.id] ? token.token : maskToken(token.token)
              }}</code>
              <button
                class="icon-btn"
                type="button"
                :title="revealed[token.id] ? '隐藏令牌' : '显示令牌'"
                @click="revealed[token.id] = !revealed[token.id]"
              >
                <EyeOff v-if="revealed[token.id]" :size="14" />
                <Eye v-else :size="14" />
              </button>
              <button
                class="icon-btn"
                type="button"
                title="复制令牌"
                @click="copy(token.token)"
              >
                <Copy :size="14" />
              </button>
            </div>
            <div class="deploy-card-meta">
              <span>创建于 {{ exactTime(token.created_at) }}</span>
              <span>
                最近使用：{{
                  token.last_used_at ? timeAgo(token.last_used_at) : '从未'
                }}
              </span>
            </div>
            <div class="deploy-card-actions">
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="openCommands(token)"
              >
                <Terminal :size="13" />
                部署命令
              </button>
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="toggleToken(token)"
              >
                <Power :size="13" />
                {{ token.enabled ? '停用' : '启用' }}
              </button>
              <button
                class="btn btn-ghost btn-sm deploy-card-delete"
                type="button"
                @click="deleteTarget = token"
              >
                <Trash2 :size="13" />
                删除
              </button>
            </div>
          </article>
        </div>

        <div class="deploy-note">
          <Rocket :size="15" />
          <div>
            <strong>部署说明</strong>
            <p>
              在已安装 RustDesk 的设备上以管理员权限运行命令。Windows 客户端通常位于
              <code>C:\Program Files\RustDesk\rustdesk.exe</code>，Linux 与 macOS
              可直接使用 <code>rustdesk</code>。部署令牌等同于管理凭据，请妥善保管。
            </p>
          </div>
        </div>
      </div>
    </main>

    <div v-if="createOpen" class="modal-scrim" @click.self="closeCreate">
      <form class="modal" role="dialog" aria-modal="true" @submit.prevent="createToken">
        <h2 class="modal-title">新建部署令牌</h2>
        <p class="modal-sub">
          为不同的交付批次创建独立令牌，停用或删除令牌不会影响已部署的设备。
        </p>
        <div class="modal-body">
          <div class="field">
            <label class="label" for="token-name">
              令牌名称 <span class="hint">可选</span>
            </label>
            <input
              id="token-name"
              v-model="createName"
              class="input"
              maxlength="64"
              placeholder="例如：华东批次 2026-10"
              autofocus
            />
          </div>
          <p v-if="createError" class="field-error">{{ createError }}</p>
        </div>
        <div class="modal-actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="createBusy"
            @click="closeCreate"
          >
            取消
          </button>
          <button class="btn btn-primary" type="submit" :disabled="createBusy">
            <span v-if="createBusy" class="spinner" />
            {{ createBusy ? '创建中…' : '创建令牌' }}
          </button>
        </div>
      </form>
    </div>

    <div v-if="commandToken" class="modal-scrim" @click.self="commandToken = null">
      <div class="modal modal-wide" role="dialog" aria-modal="true">
        <h2 class="modal-title">部署命令</h2>
        <p class="modal-sub">
          令牌「{{ commandToken.name || `#${commandToken.id}` }}」：
          {{
            commandToken.enabled
              ? '当前已启用，可直接使用。'
              : '当前已停用，命令将无法通过校验。'
          }}
        </p>
        <div class="modal-body">
          <div
            v-for="command in commands"
            :key="command.label"
            class="command-block"
          >
            <div class="command-head">
              <div>
                <strong>{{ command.label }}</strong>
                <p>{{ command.note }}</p>
              </div>
              <button
                class="icon-btn"
                type="button"
                title="复制命令"
                @click="copy(command.value)"
              >
                <Copy :size="14" />
              </button>
            </div>
            <pre class="command-text">{{ command.value }}</pre>
          </div>
          <p class="command-tip">
            可追加 <code>--device_name</code>、<code>--device_group_name</code>、
            <code>--strategy_name</code>、<code>--address_book_name</code>、
            <code>--note</code> 等参数；设备需要先完成部署再执行分配。
          </p>
        </div>
        <div class="modal-actions">
          <button class="btn btn-secondary" type="button" @click="commandToken = null">
            关闭
          </button>
        </div>
      </div>
    </div>

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除部署令牌？"
      :message="`「${deleteTarget.name || deleteTarget.id}」将立即失效，使用该令牌的部署命令会返回未启用。`"
      confirm-label="删除令牌"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="removeToken"
    />
  </AppShell>
</template>

<style scoped>
.deploy-table {
  min-width: 940px;
}

.deploy-table tbody tr {
  cursor: default;
}

.token-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.token-id {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
}

.token-value {
  display: flex;
  align-items: center;
  gap: 4px;
}

.token-value code {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 3px 7px;
  border-radius: 5px;
  background: var(--wash);
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--ink-soft);
}

.deploy-time {
  font-size: 11.5px;
  color: var(--muted);
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.deploy-cards {
  display: none;
  flex-direction: column;
}

.deploy-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.deploy-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.deploy-card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  margin-top: 9px;
  font-size: 11px;
  color: var(--faint);
}

.deploy-card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 11px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.deploy-card-delete {
  margin-left: auto;
  color: var(--red);
}

.deploy-note {
  display: flex;
  gap: 11px;
  align-items: flex-start;
  padding: 14px 16px;
  border-top: 1px solid var(--line);
  background: var(--wash);
  color: var(--muted);
}

.deploy-note strong {
  display: block;
  font-size: 12px;
  color: var(--ink);
}

.deploy-note p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.65;
}

.modal-wide {
  max-width: 720px;
  width: min(720px, calc(100vw - 32px));
}

.command-block {
  margin-bottom: 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.command-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  background: var(--wash);
}

.command-head strong {
  font-size: 12px;
  color: var(--ink);
}

.command-head p {
  margin: 2px 0 0;
  font-size: 11px;
  line-height: 1.55;
  color: var(--faint);
}

.command-text {
  margin: 0;
  padding: 12px;
  overflow-x: auto;
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.6;
  color: var(--ink);
  background: var(--canvas);
  white-space: pre-wrap;
  word-break: break-all;
}

.command-tip {
  margin: 4px 0 0;
  font-size: 11px;
  line-height: 1.7;
  color: var(--faint);
}

code {
  font-family: var(--font-mono);
}

@media (max-width: 960px) {
  .deploy-cards {
    display: flex;
  }
}
</style>
