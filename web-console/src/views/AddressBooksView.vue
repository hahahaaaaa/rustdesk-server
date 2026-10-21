<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  BookOpen,
  FolderPlus,
  LibraryBig,
  MonitorSmartphone,
  Pencil,
  RefreshCw,
  Search,
  Share2,
  Trash2,
  Users,
  WifiOff,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import AddressBookModal from '../components/AddressBookModal.vue'
import AddressBookPeersModal from '../components/AddressBookPeersModal.vue'
import AddressBookShareModal from '../components/AddressBookShareModal.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import { exactTime, timeAgo } from '../lib/format'
import { clearSession } from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const books = ref([])
const users = ref([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const search = ref('')
const applied = ref('')

const modalOpen = ref(false)
const editingBook = ref(null)
const modalBusy = ref(false)
const modalError = ref('')

const deleteTarget = ref(null)
const deleteBusy = ref(false)

const peersOpen = ref(false)
const peersBook = ref(null)
const peers = ref([])
const tags = ref([])
const peersLoading = ref(false)
const peersBusy = ref(false)
const peersError = ref('')

const sharesOpen = ref(false)
const sharesBook = ref(null)
const shares = ref([])
const sharesBusy = ref(false)
const sharesError = ref('')

const rows = computed(() =>
  [...books.value].sort((a, b) => a.name.localeCompare(b.name))
)
const hasFilters = computed(() => Boolean(applied.value))
const stats = computed(() => ({
  books: books.value.length,
  peers: books.value.reduce(
    (sum, book) => sum + Number(book.peer_count || 0),
    0
  ),
  shares: books.value.reduce(
    (sum, book) => sum + Number(book.share_count || 0),
    0
  ),
}))

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  else refreshing.value = true
  error.value = ''
  try {
    const [bookRows, userRows] = await Promise.all([
      api.getAddressBooks({ q: applied.value }),
      api.getUsers(),
    ])
    books.value = bookRows
    users.value = userRows
  } catch (e) {
    error.value = e.message || '地址簿加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function applySearch() {
  applied.value = search.value.trim()
  load(false)
}

function clearSearch() {
  search.value = ''
  applied.value = ''
  load(false)
}

function openCreate() {
  editingBook.value = null
  modalError.value = ''
  modalOpen.value = true
}

function openEdit(book) {
  editingBook.value = book
  modalError.value = ''
  modalOpen.value = true
}

async function submitBook(payload) {
  modalBusy.value = true
  modalError.value = ''
  try {
    if (editingBook.value) {
      await api.updateAddressBook(editingBook.value.guid, payload)
      notify('地址簿已更新。')
    } else {
      await api.createAddressBook(payload)
      notify('地址簿已创建。')
    }
    modalOpen.value = false
    await load(false)
  } catch (e) {
    modalError.value = e.message || '地址簿保存失败。'
  } finally {
    modalBusy.value = false
  }
}

async function deleteBook() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await api.deleteAddressBook(deleteTarget.value.guid)
    notify('地址簿已删除。')
    deleteTarget.value = null
    await load(false)
  } catch (e) {
    notify(e.message || '地址簿删除失败。', 'error')
  } finally {
    deleteBusy.value = false
  }
}

async function openPeers(book) {
  peersBook.value = book
  peers.value = []
  tags.value = []
  peersError.value = ''
  peersOpen.value = true
  await loadPeerDetails()
}

async function loadPeerDetails() {
  if (!peersBook.value) return
  peersLoading.value = true
  peersError.value = ''
  try {
    const [peerRows, tagRows] = await Promise.all([
      api.getAddressBookPeers(peersBook.value.guid),
      api.getAddressBookTags(peersBook.value.guid),
    ])
    peers.value = peerRows
    tags.value = tagRows
  } catch (e) {
    peersError.value = e.message || '地址簿设备加载失败。'
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    peersLoading.value = false
  }
}

async function addPeer(payload) {
  if (!peersBook.value) return
  peersBusy.value = true
  peersError.value = ''
  try {
    await api.addAddressBookPeer(peersBook.value.guid, payload)
    notify('设备已添加到地址簿。')
    await loadPeerDetails()
    await load(false)
  } catch (e) {
    peersError.value = e.message || '添加设备失败。'
  } finally {
    peersBusy.value = false
  }
}

async function removePeer(peerId) {
  if (!peersBook.value) return
  peersBusy.value = true
  peersError.value = ''
  try {
    await api.deleteAddressBookPeer(peersBook.value.guid, [peerId])
    notify('设备已从地址簿移除。')
    await loadPeerDetails()
    await load(false)
  } catch (e) {
    peersError.value = e.message || '移除设备失败。'
  } finally {
    peersBusy.value = false
  }
}

async function addTag(name) {
  if (!peersBook.value) return
  peersBusy.value = true
  peersError.value = ''
  try {
    await api.addAddressBookTag(peersBook.value.guid, { name })
    notify('标签已创建。')
    tags.value = await api.getAddressBookTags(peersBook.value.guid)
  } catch (e) {
    peersError.value = e.message || '创建标签失败。'
  } finally {
    peersBusy.value = false
  }
}

async function openShares(book) {
  sharesBook.value = book
  shares.value = []
  sharesError.value = ''
  sharesBusy.value = true
  try {
    const [shareRows, userRows] = await Promise.all([
      api.getAddressBookShares(book.guid),
      api.getUsers(),
    ])
    shares.value = shareRows
    users.value = userRows
    sharesOpen.value = true
  } catch (e) {
    notify(e.message || '共享授权加载失败。', 'error')
    if (e.status === 401) {
      clearSession()
      router.replace('/login')
    }
  } finally {
    sharesBusy.value = false
  }
}

async function submitShares(nextShares) {
  if (!sharesBook.value) return
  sharesBusy.value = true
  sharesError.value = ''
  try {
    await api.setAddressBookShares(sharesBook.value.guid, nextShares)
    notify('共享授权已保存。')
    sharesOpen.value = false
    await load(false)
  } catch (e) {
    sharesError.value = e.message || '共享授权保存失败。'
  } finally {
    sharesBusy.value = false
  }
}

function closeBookModal() {
  if (modalBusy.value) return
  modalOpen.value = false
}

function closePeers() {
  if (peersBusy.value) return
  peersOpen.value = false
}

function closeShares() {
  if (sharesBusy.value && sharesOpen.value) return
  sharesOpen.value = false
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">协作与共享</div>
          <h1 class="page-title">共享地址簿</h1>
          <p class="page-sub">
            维护团队共享的设备地址簿，授权给成员后在客户端中直接同步使用。
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
            <FolderPlus :size="15" />
            新建地址簿
          </button>
        </div>
      </div>

      <div class="stat-grid cols-3">
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">地址簿</span>
            <LibraryBig :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.books }}</div>
          <div class="stat-note">{{ hasFilters ? '符合搜索条件' : '当前共享地址簿' }}</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">收录设备</span>
            <MonitorSmartphone :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.peers }}</div>
          <div class="stat-note">所有地址簿中的设备条目</div>
        </div>
        <div class="stat-card">
          <div class="stat-top">
            <span class="stat-label">共享授权</span>
            <Share2 :size="15" class="stat-icon" />
          </div>
          <div class="stat-value">{{ stats.shares }}</div>
          <div class="stat-note">用户与地址簿的授权关系</div>
        </div>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载地址簿
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载地址簿</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else class="panel">
        <div class="toolbar">
          <div class="search">
            <Search :size="14" />
            <input
              v-model="search"
              class="input"
              type="search"
              placeholder="搜索名称、所有者或备注"
              aria-label="搜索地址簿"
              @keyup.enter="applySearch"
            />
          </div>
          <button
            class="btn btn-secondary btn-sm"
            type="button"
            @click="applySearch"
          >
            搜索
          </button>
          <button
            v-if="hasFilters"
            class="btn btn-ghost btn-sm"
            type="button"
            @click="clearSearch"
          >
            重置
          </button>
          <div class="toolbar-spacer" />
          <span class="faint" style="font-size: 11.5px">
            共 {{ rows.length }} 个地址簿
          </span>
        </div>

        <div v-if="!rows.length" class="empty">
          <div class="empty-icon"><BookOpen :size="18" /></div>
          <h3>{{ hasFilters ? '没有匹配的地址簿' : '暂无共享地址簿' }}</h3>
          <p>
            {{
              hasFilters
                ? '尝试调整搜索关键词。'
                : '创建地址簿并添加设备，然后授权给团队成员。'
            }}
          </p>
          <button
            v-if="hasFilters"
            class="btn btn-secondary"
            type="button"
            @click="clearSearch"
          >
            清除搜索
          </button>
          <button v-else class="btn btn-primary" type="button" @click="openCreate">
            <FolderPlus :size="15" />
            新建地址簿
          </button>
        </div>

        <div v-else class="table-wrap">
          <table class="table ab-table">
            <thead>
              <tr>
                <th>地址簿</th>
                <th style="width: 120px">所有者</th>
                <th style="width: 86px">设备</th>
                <th style="width: 86px">共享</th>
                <th style="width: 92px">访问密码</th>
                <th style="width: 150px">更新时间</th>
                <th style="width: 168px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="book in rows" :key="book.guid">
                <td>
                  <div class="book-cell">
                    <span class="book-mark"><BookOpen :size="14" /></span>
                    <div class="book-copy">
                      <div class="book-name">{{ book.name }}</div>
                      <div class="book-note" :title="book.note">
                        {{ book.note || '暂无备注' }}
                      </div>
                    </div>
                  </div>
                </td>
                <td>
                  <span class="owner-name">{{ book.owner || '—' }}</span>
                </td>
                <td>
                  <span class="count-pill">
                    <MonitorSmartphone :size="11" />
                    {{ book.peer_count || 0 }}
                  </span>
                </td>
                <td>
                  <span class="count-pill">
                    <Users :size="11" />
                    {{ book.share_count || 0 }}
                  </span>
                </td>
                <td>
                  <span v-if="book.info && book.info.password" class="tag tag-blue">
                    已设置
                  </span>
                  <span v-else class="faint">—</span>
                </td>
                <td>
                  <div class="time-cell">{{ timeAgo(book.updated_at) }}</div>
                  <div class="time-full">{{ exactTime(book.updated_at) }}</div>
                </td>
                <td>
                  <div class="row-actions">
                    <button class="row-link" type="button" @click="openPeers(book)">
                      <MonitorSmartphone :size="13" />
                      设备
                    </button>
                    <button class="row-link" type="button" @click="openShares(book)">
                      <Share2 :size="13" />
                      共享
                    </button>
                    <button
                      class="icon-btn"
                      type="button"
                      title="编辑地址簿"
                      @click="openEdit(book)"
                    >
                      <Pencil :size="14" />
                    </button>
                    <button
                      class="icon-btn danger"
                      type="button"
                      title="删除地址簿"
                      @click="deleteTarget = book"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="rows.length" class="book-cards">
          <article v-for="book in rows" :key="book.guid" class="book-card">
            <div class="book-card-head">
              <span class="book-mark"><BookOpen :size="14" /></span>
              <div class="book-copy">
                <div class="book-name">{{ book.name }}</div>
                <div class="book-note" :title="book.note">
                  {{ book.note || '暂无备注' }}
                </div>
              </div>
              <span v-if="book.info && book.info.password" class="tag tag-blue">
                已设密码
              </span>
            </div>
            <div class="book-card-meta">
              <span class="count-pill">
                <MonitorSmartphone :size="11" />
                {{ book.peer_count || 0 }} 台设备
              </span>
              <span class="count-pill">
                <Users :size="11" />
                {{ book.share_count || 0 }} 个共享
              </span>
              <span class="owner-name">所有者 {{ book.owner || '—' }}</span>
            </div>
            <div class="book-card-foot">
              <span class="time-cell">{{ timeAgo(book.updated_at) }}</span>
              <div class="row-actions">
                <button class="row-link" type="button" @click="openPeers(book)">
                  <MonitorSmartphone :size="13" />
                  设备
                </button>
                <button class="row-link" type="button" @click="openShares(book)">
                  <Share2 :size="13" />
                  共享
                </button>
                <button
                  class="icon-btn"
                  type="button"
                  title="编辑地址簿"
                  @click="openEdit(book)"
                >
                  <Pencil :size="14" />
                </button>
                <button
                  class="icon-btn danger"
                  type="button"
                  title="删除地址簿"
                  @click="deleteTarget = book"
                >
                  <Trash2 :size="14" />
                </button>
              </div>
            </div>
          </article>
        </div>
      </div>
    </main>

    <AddressBookModal
      v-if="modalOpen"
      :book="editingBook"
      :busy="modalBusy"
      :error="modalError"
      @cancel="closeBookModal"
      @submit="submitBook"
    />

    <AddressBookPeersModal
      v-if="peersOpen"
      :book="peersBook"
      :peers="peers"
      :tags="tags"
      :loading="peersLoading"
      :busy="peersBusy"
      :error="peersError"
      @cancel="closePeers"
      @refresh="loadPeerDetails"
      @add="addPeer"
      @remove="removePeer"
      @add-tag="addTag"
    />

    <AddressBookShareModal
      v-if="sharesOpen"
      :book="sharesBook"
      :users="users"
      :shares="shares"
      :busy="sharesBusy"
      :error="sharesError"
      @cancel="closeShares"
      @submit="submitShares"
    />

    <ConfirmDialog
      v-if="deleteTarget"
      title="删除此地址簿？"
      :message="`${deleteTarget.name} 及其中的设备条目、共享授权将被永久删除，授权用户将无法再同步该地址簿。`"
      confirm-label="删除地址簿"
      danger
      :busy="deleteBusy"
      @cancel="deleteTarget = null"
      @confirm="deleteBook"
    />
  </AppShell>
</template>

<style scoped>
.ab-table {
  min-width: 900px;
}

.ab-table tbody tr {
  cursor: default;
}

.book-cell {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
}

.book-mark {
  display: grid;
  place-items: center;
  flex: none;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--ink-soft);
}

.book-copy {
  min-width: 0;
}

.book-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
}

.book-note {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
  font-size: 11px;
  color: var(--muted);
}

.owner-name {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
}

.count-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 22px;
  padding: 0 8px;
  border-radius: 9999px;
  background: var(--wash);
  box-shadow: inset 0 0 0 1px var(--line);
  font-size: 11px;
  color: var(--muted);
}

.time-cell {
  font-size: 12px;
  color: var(--ink);
  white-space: nowrap;
}

.time-full {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--faint);
  white-space: nowrap;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 4px;
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

.tag-blue {
  background: var(--blue-wash);
  box-shadow: none;
  color: #0068d6;
}

.book-cards {
  display: none;
  flex-direction: column;
}

.book-card {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.book-card:last-child {
  border-bottom: 0;
}

.book-card-head {
  display: flex;
  align-items: flex-start;
  gap: 9px;
}

.book-card-head .book-copy {
  flex: 1;
}

.book-card-head .book-name,
.book-card-head .book-note {
  max-width: none;
  white-space: normal;
  overflow-wrap: anywhere;
}

.book-card-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
}

.book-card-meta .owner-name {
  margin-left: auto;
}

.book-card-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

.book-card-foot .row-actions {
  margin-left: auto;
}

@media (max-width: 960px) {
  .book-cards {
    display: flex;
  }
}
</style>
