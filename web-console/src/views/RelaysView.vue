<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Network,
  Pencil,
  Plus,
  RefreshCw,
  RotateCw,
  Server,
  Trash2,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime } from '../lib/format'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const rows = ref([])
const builtin = ref('')
const consoleAddr = ref('')
const loading = ref(true)
const refreshing = ref(false)
const syncing = ref(false)
const error = ref('')
const syncResult = ref('')

const modalOpen = ref(false)
const editing = ref(null)
const form = ref({ name: '', host: '', port: 21117, note: '', enabled: true })
const modalBusy = ref(false)
const modalError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)

const enabledCount = computed(
  () => rows.value.filter((relay) => relay.enabled).length
)

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const data = await api.getRelays()
    rows.value = Array.isArray(data?.data) ? data.data : []
    builtin.value = data?.builtin || ''
    consoleAddr.value = data?.console || ''
  } catch (e) {
    error.value = e.message || '中继服务器加载失败。'
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
  form.value = { name: '', host: '', port: 21117, note: '', enabled: true }
  modalError.value = ''
  modalOpen.value = true
}

function openEdit(relay) {
  editing.value = relay
  form.value = {
    name: relay.name || '',
    host: relay.host || '',
    port: relay.port || 21117,
    note: relay.note || '',
    enabled: relay.enabled !== false,
  }
  modalError.value = ''
  modalOpen.value = true
}

function closeModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

async function submitRelay() {
  modalBusy.value = true
  modalError.value = ''
  const payload = {
    name: form.value.name.trim(),
    host: form.value.host.trim(),
    port: Number(form.value.port) || 21117,
    note: form.value.note.trim(),
    enabled: form.value.enabled,
  }
  try {
    if (editing.value) {
      await api.updateRelay(editing.value.guid, payload)
      notify('中继服务器已更新。')
    } else {
      await api.createRelay(payload)
      notify('中继服务器已添加。')
    }
    modalOpen.value = false
    await load(false)
    syncResult.value = '配置已提交，hbbs 会立即使用新的中继列表。'
  } catch (e) {
    modalError.value = e.message || '保存中继服务器失败。'
  } finally {
    modalBusy.value = false
  }
}

async function removeRelay() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteRelay(deleteTarget.value.guid)
    notify('中继服务器已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '删除中继服务器失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

async function syncNow() {
  syncing.value = true
  syncResult.value = ''
  try {
    const result = await api.syncRelays()
    if (result?.success) {
      syncResult.value = result.response
        ? `同步成功：${result.response}`
        : '同步成功，hbbs 已接受中继列表。'
      notify('中继列表已同步到 hbbs。')
    } else {
      syncResult.value = result?.error || '同步失败。'
      notify(syncResult.value, 'error')
    }
  } catch (e) {
    syncResult.value = e.message || '同步失败。'
    notify(syncResult.value, 'error')
  } finally {
    syncing.value = false
  }
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">运维</div>
          <h1 class="page-title">中继服务器</h1>
          <p class="page-sub">
            配置客户端可用的中继节点，列表会自动下发到 hbbs，并在设备心跳时生效。
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
            class="btn btn-secondary"
            type="button"
            :disabled="syncing"
            @click="syncNow"
          >
            <RotateCw
              :size="15"
              :style="syncing ? { animation: 'spin .8s linear infinite' } : null"
            />
            {{ syncing ? '同步中…' : '立即同步' }}
          </button>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <Plus :size="15" />
            添加中继
          </button>
        </div>
      </div>

      <div class="relay-overview">
        <div class="relay-card">
          <div class="relay-card-top">
            <span>内置中继</span>
            <Server :size="15" class="faint" />
          </div>
          <div class="relay-address mono">{{ builtin || '—' }}</div>
          <p>随一体化镜像启动的 hbbr，未配置外部中继时作为默认节点。</p>
        </div>
        <div class="relay-card">
          <div class="relay-card-top">
            <span>外部中继</span>
            <Network :size="15" class="faint" />
          </div>
          <div class="relay-address">{{ enabledCount }} / {{ rows.length }}</div>
          <p>已启用 / 已配置。禁用全部外部中继后，hbbs 会恢复使用内置中继。</p>
        </div>
        <div class="relay-card">
          <div class="relay-card-top">
            <span>hbbs 控制台</span>
            <WifiOff :size="15" class="faint" />
          </div>
          <div class="relay-address mono">{{ consoleAddr || '—' }}</div>
          <p>控制台通过该本地端口即时更新 hbbs 的中继列表，无需重启服务。</p>
        </div>
      </div>

      <div v-if="syncResult" class="relay-result">
        <Network :size="14" />
        <span>{{ syncResult }}</span>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载中继服务器
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载中继服务器</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else-if="!rows.length" class="panel">
        <div class="empty">
          <div class="empty-icon"><Network :size="18" /></div>
          <h3>还没有外部中继</h3>
          <p>
            当前使用内置中继。如需扩容或跨地域加速，可添加 hbbr 节点的地址与端口。
          </p>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <Plus :size="15" />
            添加中继
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="table-wrap">
          <table class="table relays-table">
            <thead>
              <tr>
                <th>名称</th>
                <th>地址</th>
                <th>备注</th>
                <th style="width: 110px">状态</th>
                <th style="width: 158px">更新时间</th>
                <th style="width: 88px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="relay in rows" :key="relay.guid">
                <td>
                  <div class="relay-name">{{ relay.name || relay.host }}</div>
                  <div class="relay-guid mono">{{ relay.guid }}</div>
                </td>
                <td>
                  <span class="mono relay-host">
                    {{ relay.host }}:{{ relay.port }}
                  </span>
                </td>
                <td>
                  <span class="relay-note" :title="relay.note">
                    {{ relay.note || '—' }}
                  </span>
                </td>
                <td>
                  <span
                    class="pill"
                    :class="relay.enabled ? 'pill-online' : 'pill-disabled'"
                  >
                    <span class="dot" />
                    {{ relay.enabled ? '已启用' : '已停用' }}
                  </span>
                </td>
                <td class="relay-time">{{ exactTime(relay.updated_at) }}</td>
                <td>
                  <div class="row-actions">
                    <button
                      class="icon-btn"
                      type="button"
                      title="编辑中继"
                      @click="openEdit(relay)"
                    >
                      <Pencil :size="14" />
                    </button>
                    <button
                      class="icon-btn danger"
                      type="button"
                      title="删除中继"
                      @click="deleteTarget = relay"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="relay-cards">
          <article
            v-for="relay in rows"
            :key="`card-${relay.guid}`"
            class="relay-card"
          >
            <div class="relay-card-head">
              <div class="relay-card-title">
                <div class="relay-name">{{ relay.name || relay.host }}</div>
                <div class="relay-guid mono">{{ relay.guid }}</div>
              </div>
              <span
                class="pill"
                :class="relay.enabled ? 'pill-online' : 'pill-disabled'"
              >
                <span class="dot" />
                {{ relay.enabled ? '已启用' : '已停用' }}
              </span>
            </div>
            <div class="relay-card-host mono">
              {{ relay.host }}:{{ relay.port }}
            </div>
            <div v-if="relay.note" class="relay-card-note">{{ relay.note }}</div>
            <div class="relay-card-meta">
              更新于 {{ exactTime(relay.updated_at) }}
            </div>
            <div class="relay-card-actions">
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="openEdit(relay)"
              >
                <Pencil :size="13" />
                编辑
              </button>
              <button
                class="btn btn-ghost btn-sm relay-card-delete"
                type="button"
                @click="deleteTarget = relay"
              >
                <Trash2 :size="13" />
                删除
              </button>
            </div>
          </article>
        </div>

        <div class="relay-foot">
          <Network :size="14" />
          <span>
            修改后会自动尝试同步 hbbs；若同步失败，后台任务会每 5 分钟重试一次。
          </span>
        </div>
      </div>
    </main>

    <div v-if="modalOpen" class="modal-scrim" @click.self="closeModal">
      <form class="modal" role="dialog" aria-modal="true" @submit.prevent="submitRelay">
        <h2 class="modal-title">
          {{ editing ? '编辑中继服务器' : '添加中继服务器' }}
        </h2>
        <p class="modal-sub">
          填写 hbbr 对外可访问的地址与中继端口，客户端将通过 hbbs 获取该列表。
        </p>
        <div class="modal-body">
          <div class="field">
            <label class="label" for="relay-name">
              名称 <span class="hint">可选</span>
            </label>
            <input
              id="relay-name"
              v-model="form.name"
              class="input"
              maxlength="64"
              placeholder="例如：华东中继"
              autofocus
            />
          </div>
          <div class="field">
            <label class="label" for="relay-host">中继地址</label>
            <input
              id="relay-host"
              v-model="form.host"
              class="input mono"
              placeholder="relay.example.com 或 10.0.0.12"
            />
          </div>
          <div class="field">
            <label class="label" for="relay-port">端口</label>
            <input
              id="relay-port"
              v-model.number="form.port"
              class="input mono"
              type="number"
              min="1"
              max="65535"
              placeholder="21117"
            />
          </div>
          <div class="field">
            <label class="label" for="relay-note">
              备注 <span class="hint">可选</span>
            </label>
            <textarea
              id="relay-note"
              v-model="form.note"
              class="textarea"
              maxlength="240"
              placeholder="地域、带宽或其他说明"
            />
          </div>
          <label class="check-row">
            <input v-model="form.enabled" type="checkbox" />
            <span>
              启用该中继
              <small>停用后，hbbs 会在下一次同步时移除该节点。</small>
            </span>
          </label>
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
            :disabled="modalBusy || !form.host.trim()"
          >
            <span v-if="modalBusy" class="spinner" />
            {{ modalBusy ? '保存中…' : editing ? '保存中继' : '添加中继' }}
          </button>
        </div>
      </form>
    </div>

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除这个中继？"
      :message="`${deleteTarget.name || deleteTarget.host} 将从 hbbs 的中继列表中移除，已建立的会话不受影响。`"
      confirm-label="删除中继"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="removeRelay"
    />
  </AppShell>
</template>

<style scoped>
.relay-overview {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}

.relay-card {
  padding: 15px 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-md);
  background: var(--canvas);
}

.relay-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 11.5px;
  font-weight: 500;
  color: var(--muted);
}

.relay-address {
  margin-top: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--ink);
  overflow-wrap: anywhere;
}

.relay-card p {
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 1.6;
  color: var(--faint);
}

.relay-result {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding: 10px 13px;
  border-radius: var(--radius-sm);
  background: var(--wash);
  font-size: 11.5px;
  color: var(--muted);
}

.relays-table {
  min-width: 820px;
}

.relays-table tbody tr {
  cursor: default;
}

.relay-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.relay-guid {
  margin-top: 2px;
  font-size: 9.5px;
  color: var(--faint);
  overflow-wrap: anywhere;
}

.relay-host {
  font-size: 11.5px;
  color: var(--ink-soft);
}

.relay-note {
  display: block;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11.5px;
  color: var(--muted);
}

.relay-time {
  font-size: 11.5px;
  color: var(--muted);
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.relay-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 11px 16px;
  border-top: 1px solid var(--line);
  background: var(--wash);
  font-size: 11px;
  color: var(--faint);
}

.check-row {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  margin-top: 14px;
  font-size: 12.5px;
  color: var(--ink-soft);
  cursor: pointer;
}

.check-row input {
  margin-top: 2px;
  accent-color: var(--ink);
}

.check-row small {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--faint);
}

.relay-cards {
  display: none;
  flex-direction: column;
}

.relay-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.relay-card:last-child {
  border-bottom: 0;
}

.relay-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.relay-card-title {
  min-width: 0;
}

.relay-card-host {
  margin-top: 8px;
  font-size: 12px;
  color: var(--ink-soft);
  word-break: break-all;
}

.relay-card-note {
  margin-top: 6px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--muted);
}

.relay-card-meta {
  margin-top: 8px;
  font-size: 11px;
  color: var(--faint);
}

.relay-card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.relay-card-delete {
  margin-left: auto;
  color: var(--red);
}

@media (max-width: 960px) {
  .relay-overview {
    grid-template-columns: 1fr;
  }

  .relay-cards {
    display: flex;
  }
}
</style>
