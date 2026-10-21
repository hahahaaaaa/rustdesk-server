<script setup>
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  CheckCircle2,
  Database,
  Globe,
  LogOut,
  RefreshCw,
  RotateCcw,
  Server,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { getApi } from '../lib/api'
import {
  defaultWebClientUrl,
  resetWebClientUrl,
  saveWebClientUrl,
  storedWebClientUrl,
} from '../lib/control'
import {
  clearSession,
  displayName,
  session,
  userHandle,
} from '../lib/session'
import { notify } from '../lib/toast'

const router = useRouter()
const api = getApi()

const testing = ref(false)
const testResult = ref('')
const resetOpen = ref(false)
const resetBusy = ref(false)
const savedWebUrl = ref(storedWebClientUrl())
const webClientInput = ref(savedWebUrl.value)

const effectiveWebUrl = computed(
  () => savedWebUrl.value.trim() || defaultWebClientUrl() || ''
)

async function testConnection() {
  testing.value = true
  testResult.value = ''
  try {
    await api.testConnection()
    testResult.value = '连接成功，服务器 API 响应正常。'
    notify('服务器连接正常。')
  } catch (e) {
    testResult.value = e.message || '服务器没有响应。'
    notify(testResult.value, 'error')
  } finally {
    testing.value = false
  }
}

function saveWebClient() {
  try {
    const normalized = saveWebClientUrl(webClientInput.value)
    webClientInput.value = normalized
    savedWebUrl.value = normalized
    notify(
      normalized ? '网页远控地址已保存。' : '已改为使用默认网页远控地址。'
    )
  } catch {
    notify('请输入有效的网址，例如 https://rd.example.com/web。', 'error')
  }
}

function restoreDefaultWebClient() {
  resetWebClientUrl()
  webClientInput.value = ''
  savedWebUrl.value = ''
  notify('已恢复默认网页远控地址。')
}

async function resetDemo() {
  resetBusy.value = true
  try {
    await api.reset()
    notify('演示数据已恢复。')
    resetOpen.value = false
  } catch (e) {
    notify(e.message || '无法重置演示工作区。', 'error')
  } finally {
    resetBusy.value = false
  }
}

function logout() {
  clearSession()
  router.push('/login')
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">系统</div>
          <h1 class="page-title">设置</h1>
          <p class="page-sub">
            查看当前控制台的连接信息，并配置网页远程控制等选项。
          </p>
        </div>
      </div>

      <div class="settings-stack">
        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>连接</h3>
              <p class="desc">
                控制台会从当前浏览器直接访问 RustDesk Server Pro 的管理 API。
              </p>
            </div>
            <Server :size="16" class="faint" />
          </div>

          <dl class="detail-list">
            <div class="detail-row">
              <dt>服务器</dt>
              <dd class="mono" style="font-size: 11.5px">
                {{ session.demo ? 'demo://local' : session.baseUrl }}
              </dd>
            </div>
            <div class="detail-row">
              <dt>模式</dt>
              <dd>{{ session.demo ? '本地演示工作区' : '在线服务器' }}</dd>
            </div>
            <div class="detail-row">
              <dt>账号</dt>
              <dd>
                {{ displayName() }}
                <span v-if="userHandle()" class="faint">
                  · {{ userHandle() }}
                </span>
              </dd>
            </div>
            <div class="detail-row">
              <dt>角色</dt>
              <dd>{{ session.user?.is_admin ? '管理员' : '成员' }}</dd>
            </div>
          </dl>

          <div class="settings-actions">
            <button
              class="btn btn-secondary"
              type="button"
              :disabled="testing"
              @click="testConnection"
            >
              <RefreshCw
                :size="14"
                :style="
                  testing ? { animation: 'spin .8s linear infinite' } : null
                "
              />
              {{ testing ? '测试中…' : '测试连接' }}
            </button>
          </div>

          <p
            v-if="testResult"
            class="desc"
            style="margin-top: 12px; color: var(--green)"
          >
            <CheckCircle2
              :size="12"
              style="vertical-align: -2px; margin-right: 4px"
            />
            {{ testResult }}
          </p>
        </section>

        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>远程控制</h3>
              <p class="desc">
                设备页的“远程控制”会优先调用本机已安装的 RustDesk 客户端；
                也可以在这里配置 RustDesk Server Pro 网页客户端地址，在浏览器中发起远控。
              </p>
            </div>
            <Globe :size="16" class="faint" />
          </div>

          <div class="field">
            <label class="label" for="web-client-url">网页远控地址</label>
            <div class="inline-form">
              <input
                id="web-client-url"
                v-model="webClientInput"
                class="input"
                type="url"
                :placeholder="defaultWebClientUrl() || 'https://rd.example.com/web'"
                @keyup.enter="saveWebClient"
              />
              <button
                class="btn btn-secondary"
                type="button"
                @click="saveWebClient"
              >
                保存
              </button>
            </div>
          </div>

          <p class="desc" style="margin-top: 0">
            当前生效地址：
            <span class="mono" style="font-size: 11px">
              {{ effectiveWebUrl || '未配置' }}
            </span>
          </p>

          <div class="settings-actions">
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              @click="restoreDefaultWebClient"
            >
              <RotateCcw :size="14" />
              恢复默认
            </button>
          </div>

          <p v-if="session.demo" class="desc" style="margin-top: 12px">
            演示模式不会连接真实服务器，因此网页远控在演示模式下不可用。
          </p>
        </section>

        <section v-if="session.demo" class="settings-card">
          <div class="section-head">
            <div>
              <h3>演示工作区</h3>
              <p class="desc">
                演示设备与分组保存在当前浏览器中，所有改动都只在本机生效，
                不会上传到任何服务器。
              </p>
            </div>
            <Database :size="16" class="faint" />
          </div>
          <div class="settings-actions">
            <button
              class="btn btn-secondary"
              type="button"
              @click="resetOpen = true"
            >
              <RefreshCw :size="14" />
              恢复演示数据
            </button>
          </div>
        </section>

        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>API 接口</h3>
              <p class="desc">
                控制台使用以下 RustDesk Server Pro 接口，所有操作都会遵循当前账号的权限。
              </p>
            </div>
          </div>
          <div class="device-card-meta" style="margin-top: 12px">
            <span class="tag tag-blue">/api/login</span>
            <span class="tag tag-blue">/api/devices</span>
            <span class="tag tag-blue">/api/devices/&#123;guid&#125;/assign</span>
            <span class="tag tag-blue">/api/device-groups</span>
            <span class="tag tag-blue">/api/users</span>
          </div>
        </section>

        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>会话</h3>
              <p class="desc">
                退出登录会从当前浏览器移除访问令牌，服务器上的设备与分组不会受到影响。
              </p>
            </div>
          </div>
          <div class="settings-actions">
            <button class="btn btn-danger-ghost btn-sm" type="button" @click="logout">
              <LogOut :size="14" />
              退出登录
            </button>
          </div>
        </section>
      </div>
    </main>

    <ConfirmDialog
      v-if="resetOpen"
      title="恢复演示数据？"
      message="你对演示设备和分组所做的改动都会被丢弃。"
      confirm-label="恢复演示数据"
      :busy="resetBusy"
      @cancel="resetOpen = false"
      @confirm="resetDemo"
    />
  </AppShell>
</template>
