<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  CheckCircle2,
  LogOut,
  RefreshCw,
  Server,
  ShieldCheck,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import TfaDisableModal from '../components/TfaDisableModal.vue'
import TfaSetupModal from '../components/TfaSetupModal.vue'
import { getApi } from '../lib/api'
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

const tfa = ref({ enabled: false })
const tfaLoading = ref(true)
const tfaError = ref('')
const tfaSetupOpen = ref(false)
const tfaSetup = ref(null)
const tfaSetupBusy = ref(false)
const tfaSetupError = ref('')
const tfaDisableOpen = ref(false)
const tfaDisableBusy = ref(false)
const tfaDisableError = ref('')

const myRoles = computed(() =>
  Array.isArray(session.user?.roles) ? session.user.roles : []
)

const permissionSummary = computed(() => {
  if (session.user?.is_admin) return '全部权限（管理员）'
  const list = Array.isArray(session.user?.permissions)
    ? session.user.permissions
    : []
  if (!list.length) return '仅基础访问权限'
  return `${list.length} 项权限`
})

onMounted(loadTfa)

async function loadTfa() {
  try {
    tfa.value = await api.getTfaStatus()
  } catch (e) {
    tfaError.value = e.message || '无法读取双重认证状态。'
  } finally {
    tfaLoading.value = false
  }
}

async function startTfaSetup() {
  tfaSetupError.value = ''
  try {
    tfaSetup.value = await api.setupTfa()
    tfaSetupOpen.value = true
  } catch (e) {
    notify(e.message || '无法生成双重认证密钥。', 'error')
  }
}

async function enableTfa(code) {
  tfaSetupBusy.value = true
  tfaSetupError.value = ''
  try {
    await api.enableTfa(code)
    tfa.value = { enabled: true }
    tfaSetupOpen.value = false
    notify('双重认证已启用。')
  } catch (e) {
    tfaSetupError.value = e.message || '验证码校验失败。'
  } finally {
    tfaSetupBusy.value = false
  }
}

async function disableTfa(payload) {
  tfaDisableBusy.value = true
  tfaDisableError.value = ''
  try {
    await api.disableTfa(payload)
    tfa.value = { enabled: false }
    tfaDisableOpen.value = false
    notify('双重认证已关闭。')
  } catch (e) {
    tfaDisableError.value = e.message || '无法关闭双重认证。'
  } finally {
    tfaDisableBusy.value = false
  }
}

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
            查看一体化服务的运行信息与账号安全设置。
          </p>
        </div>
      </div>

      <div class="settings-stack">
        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>服务</h3>
              <p class="desc">
                控制台、管理 API 与 RustDesk ID/中继服务运行在同一实例中，自动使用当前访问地址，无需填写服务器地址。
              </p>
            </div>
            <Server :size="16" class="faint" />
          </div>

          <dl class="detail-list">
            <div class="detail-row">
              <dt>访问地址</dt>
              <dd class="mono" style="font-size: 11.5px">
                {{ session.baseUrl }}
              </dd>
            </div>
            <div class="detail-row">
              <dt>模式</dt>
              <dd>一体化服务</dd>
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
              <dd>
                <div class="role-list">
                  <span v-if="session.user?.is_admin" class="tag tag-blue">
                    管理员
                  </span>
                  <span v-for="role in myRoles" :key="role" class="tag">
                    {{ role }}
                  </span>
                  <span
                    v-if="!session.user?.is_admin && !myRoles.length"
                    class="tag"
                  >
                    成员
                  </span>
                </div>
              </dd>
            </div>
            <div class="detail-row">
              <dt>权限</dt>
              <dd>{{ permissionSummary }}</dd>
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
              <h3>账号安全</h3>
              <p class="desc">
                为当前账号启用基于时间的一次性密码（TOTP）。启用后，网页控制台与
                RustDesk 客户端登录都需要额外输入身份验证器中的 6 位动态码。
              </p>
            </div>
            <ShieldCheck :size="16" class="faint" />
          </div>

          <dl class="detail-list">
            <div class="detail-row">
              <dt>双重认证</dt>
              <dd>
                <span
                  class="pill"
                  :class="
                    tfaLoading
                      ? 'pill-offline'
                      : tfa.enabled
                        ? 'pill-online'
                        : 'pill-disabled'
                  "
                >
                  <span class="dot" />
                  {{
                    tfaLoading
                      ? '读取中'
                      : tfa.enabled
                        ? '已启用'
                        : '未启用'
                  }}
                </span>
              </dd>
            </div>
            <div class="detail-row">
              <dt>验证方式</dt>
              <dd>身份验证器应用 · TOTP（30 秒）</dd>
            </div>
          </dl>

          <p v-if="tfaError" class="desc" style="color: var(--red)">
            {{ tfaError }}
          </p>

          <div class="settings-actions">
            <button
              v-if="tfa.enabled"
              class="btn btn-danger-ghost btn-sm"
              type="button"
              :disabled="tfaLoading"
              @click="tfaDisableError = ''; tfaDisableOpen = true"
            >
              关闭双重认证
            </button>
            <button
              v-else
              class="btn btn-primary btn-sm"
              type="button"
              :disabled="tfaLoading"
              @click="startTfaSetup"
            >
              <ShieldCheck :size="14" />
              启用双重认证
            </button>
          </div>
        </section>

        <section class="settings-card">
          <div class="section-head">
            <div>
              <h3>API 接口</h3>
              <p class="desc">
                控制台使用以下内置接口，所有操作都会遵循当前账号的权限。
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

    <TfaSetupModal
      v-if="tfaSetupOpen && tfaSetup"
      :setup="tfaSetup"
      :busy="tfaSetupBusy"
      :error="tfaSetupError"
      @cancel="!tfaSetupBusy && (tfaSetupOpen = false)"
      @submit="enableTfa"
    />

    <TfaDisableModal
      v-if="tfaDisableOpen"
      :busy="tfaDisableBusy"
      :error="tfaDisableError"
      @cancel="!tfaDisableBusy && (tfaDisableOpen = false)"
      @submit="disableTfa"
    />
  </AppShell>
</template>

<style scoped>
.role-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
</style>
