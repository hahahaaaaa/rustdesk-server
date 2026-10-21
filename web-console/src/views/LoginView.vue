<script setup>
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  ArrowRight,
  Check,
  Eye,
  EyeOff,
  KeyRound,
  ShieldCheck,
} from 'lucide-vue-next'
import {
  ApiClient,
  ApiError,
  loginToServer,
  normalizeBaseUrl,
  startDemoSession,
} from '../lib/api'
import { saveSession } from '../lib/session'

const route = useRoute()
const router = useRouter()

const DEFAULT_SERVER = 'https://desktop.voltmonkey.com.cn'

const mode = ref('password')
const server = ref(
  localStorage.getItem('rd-console-last-server') || DEFAULT_SERVER
)
const username = ref('')
const password = ref('')
const token = ref('')
const showPassword = ref(false)
const busy = ref(false)
const error = ref('')

const canSubmit = computed(() => {
  if (!server.value.trim()) return false
  if (mode.value === 'password') {
    return Boolean(username.value.trim() && password.value)
  }
  return Boolean(token.value.trim())
})

function finish() {
  const redirect =
    typeof route.query.redirect === 'string' ? route.query.redirect : '/devices'
  router.replace(redirect)
}

async function signIn() {
  if (busy.value || !canSubmit.value) return
  busy.value = true
  error.value = ''
  try {
    const baseUrl = normalizeBaseUrl(server.value)
    localStorage.setItem('rd-console-last-server', baseUrl)
    if (mode.value === 'password') {
      const result = await loginToServer({
        baseUrl,
        username: username.value.trim(),
        password: password.value,
      })
      saveSession({
        baseUrl,
        token: result.token,
        user: result.user,
        demo: false,
      })
    } else {
      const client = new ApiClient(baseUrl, token.value.trim())
      await client.testConnection()
      saveSession({
        baseUrl,
        token: token.value.trim(),
        demo: false,
        user: {
          name: 'api-token',
          display_name: 'API 令牌会话',
          email: '',
        },
      })
    }
    finish()
  } catch (e) {
    error.value =
      e instanceof ApiError || e?.message
        ? e.message
        : '登录失败，请检查服务器信息后重试。'
    busy.value = false
    return
  }
  busy.value = false
}

function useDemo() {
  startDemoSession()
  finish()
}

function switchMode(next) {
  mode.value = next
  error.value = ''
}
</script>

<template>
  <div class="login">
    <section class="login-left">
      <div class="login-brand">
        <div class="brand-mark">R</div>
        <div>
          <div class="brand-name">RustDesk</div>
          <span class="brand-tag">设备控制台</span>
        </div>
      </div>

      <div class="login-hero">
        <div class="mono-label">RustDesk Server Pro</div>
        <h1 class="login-title">
          管理整个设备集群，<br />
          <span class="quiet">一切从容有序。</span>
        </h1>
        <p class="login-copy">
          无需打开客户端，即可查看所有已注册设备、控制远程访问，并整理分组与设备归属。
        </p>
        <div class="login-points">
          <div class="login-point">
            <Check :size="15" />
            一键启用或禁用远程访问
          </div>
          <div class="login-point">
            <Check :size="15" />
            将设备分组并分配负责人
          </div>
          <div class="login-point">
            <Check :size="15" />
            直接连接你自建的服务器
          </div>
        </div>
      </div>

      <div class="login-foot">
        <ShieldCheck :size="14" />
        自托管 · 你的服务器，你的数据
      </div>
    </section>

    <section class="login-right">
      <div class="login-card">
        <h1>登录</h1>
        <p class="sub">
          连接到你的 RustDesk Server Pro 实例管理 API。
        </p>

        <form class="login-form" @submit.prevent="signIn">
          <div class="field">
            <label class="label" for="server">服务器地址</label>
            <input
              id="server"
              v-model="server"
              class="input"
              autocomplete="url"
              placeholder="https://rd.example.com:21114"
            />
          </div>

          <template v-if="mode === 'password'">
            <div class="field">
              <label class="label" for="username">用户名</label>
              <input
                id="username"
                v-model="username"
                class="input"
                autocomplete="username"
                placeholder="admin"
              />
            </div>
            <div class="field">
              <label class="label" for="password">密码</label>
              <div style="position: relative">
                <input
                  id="password"
                  v-model="password"
                  class="input"
                  :type="showPassword ? 'text' : 'password'"
                  autocomplete="current-password"
                  placeholder="••••••••"
                  style="padding-right: 38px"
                />
                <button
                  class="icon-btn"
                  type="button"
                  style="position: absolute; right: 3px; top: 3px"
                  :aria-label="showPassword ? '隐藏密码' : '显示密码'"
                  @click="showPassword = !showPassword"
                >
                  <EyeOff v-if="showPassword" :size="15" />
                  <Eye v-else :size="15" />
                </button>
              </div>
            </div>
          </template>

          <div v-else class="field">
            <label class="label" for="token">
              访问令牌
              <span class="hint">由服务器管理员提供</span>
            </label>
            <textarea
              id="token"
              v-model="token"
              class="textarea"
              style="min-height: 84px; font-family: var(--font-mono); font-size: 11.5px"
              placeholder="eyJhbGciOi…"
            />
          </div>

          <div v-if="error" class="login-error">{{ error }}</div>

          <button
            class="btn btn-primary"
            type="submit"
            :disabled="busy || !canSubmit"
          >
            <span v-if="busy" class="spinner" />
            <template v-else>
              登录
              <ArrowRight :size="15" />
            </template>
          </button>
        </form>

        <div class="login-divider">或</div>

        <button
          class="btn login-demo"
          type="button"
          :disabled="busy"
          @click="useDemo"
        >
          体验演示控制台
        </button>

        <p class="login-fine">
          <button
            class="btn btn-ghost btn-sm"
            type="button"
            style="height: auto; padding: 0; font-size: 10.5px; font-weight: 400"
            @click="switchMode(mode === 'password' ? 'token' : 'password')"
          >
            <KeyRound :size="11" />
            {{
              mode === 'password'
                ? '改用访问令牌登录'
                : '改用用户名和密码登录'
            }}
          </button>
          <br /><br />
          凭据会从当前浏览器直接发送到你的 RustDesk 服务器。如果控制台部署在其他域名下，服务器需要通过
          CORS 允许该来源。会话令牌仅保存在当前浏览器中。
        </p>
      </div>
    </section>
  </div>
</template>
