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
} from '../lib/api'
import { saveSession } from '../lib/session'

const route = useRoute()
const router = useRouter()

const mode = ref('password')
const username = ref('')
const password = ref('')
const token = ref('')
const showPassword = ref(false)
const busy = ref(false)
const error = ref('')
const tfaChallenge = ref('')
const tfaType = ref('totp')
const tfaUser = ref(null)
const tfaCode = ref('')

const tfaStep = computed(() => Boolean(tfaChallenge.value))

const canSubmit = computed(() => {
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
  if (busy.value) return
  if (tfaStep.value) {
    await verifyCode()
    return
  }
  if (!canSubmit.value) return
  busy.value = true
  error.value = ''
  try {
    const baseUrl = window.location.origin
    if (mode.value === 'password') {
      const result = await loginToServer({
        baseUrl,
        username: username.value.trim(),
        password: password.value,
      })
      if (result.tfaRequired) {
        tfaChallenge.value = result.challenge
        tfaType.value = result.tfaType
        tfaUser.value = result.user
        tfaCode.value = ''
        busy.value = false
        return
      }
      saveSession({
        token: result.token,
        user: result.user,
      })
    } else {
      const client = new ApiClient(baseUrl, token.value.trim())
      await client.testConnection()
      saveSession({
        token: token.value.trim(),
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

async function verifyCode() {
  const code = tfaCode.value.trim()
  if (busy.value || code.length < 6) return
  busy.value = true
  error.value = ''
  try {
    const baseUrl = window.location.origin
    const result = await loginToServer({
      baseUrl,
      username: username.value.trim(),
      password: password.value,
      tfaCode: code,
      tfaSecret: tfaChallenge.value,
    })
    if (result.tfaRequired) {
      tfaChallenge.value = result.challenge
      error.value = '验证码未通过，请重试。'
      return
    }
    saveSession({
      token: result.token,
      user: result.user,
    })
    finish()
  } catch (e) {
    error.value = e.message || '验证码验证失败，请重试。'
  } finally {
    busy.value = false
  }
}

function cancelTfa() {
  tfaChallenge.value = ''
  tfaCode.value = ''
  tfaUser.value = null
  error.value = ''
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
        <div class="mono-label">RustDesk 自托管服务</div>
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
          使用管理员账号登录当前 RustDesk 服务，无需填写服务器地址。
        </p>

        <form class="login-form" @submit.prevent="signIn">
          <template v-if="tfaStep">
            <div class="tfa-card">
              <div class="tfa-icon"><ShieldCheck :size="18" /></div>
              <div>
                <h2>两步验证</h2>
                <p>
                  账号
                  <strong>{{ tfaUser?.name || username }}</strong>
                  已启用双重认证，请打开身份验证器应用并输入当前的 6 位动态验证码。
                </p>
              </div>
            </div>

            <div class="field">
              <label class="label" for="tfa-code">
                验证码
                <span class="hint">{{ tfaType === 'totp' ? '30 秒刷新一次' : '查收邮件或短信' }}</span>
              </label>
              <input
                id="tfa-code"
                v-model="tfaCode"
                class="input tfa-input"
                inputmode="numeric"
                autocomplete="one-time-code"
                maxlength="8"
                placeholder="000000"
                autofocus
              />
            </div>

            <div v-if="error" class="login-error">{{ error }}</div>

            <button
              class="btn btn-primary"
              type="submit"
              :disabled="busy || tfaCode.trim().length < 6"
            >
              <span v-if="busy" class="spinner" />
              <template v-else>
                验证并登录
                <ArrowRight :size="15" />
              </template>
            </button>

            <button
              class="btn btn-ghost btn-sm tfa-back"
              type="button"
              :disabled="busy"
              @click="cancelTfa"
            >
              返回重新登录
            </button>
          </template>

          <template v-else>
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
          </template>
        </form>

        <template v-if="!tfaStep">
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
            登录凭据只会发送到当前访问的服务器，会话令牌保存在当前浏览器中。
          </p>
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.tfa-card {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 13px 14px;
  margin-bottom: 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--wash);
}

.tfa-icon {
  display: grid;
  place-items: center;
  flex: none;
  width: 34px;
  height: 34px;
  border-radius: 9999px;
  background: #fff;
  box-shadow: inset 0 0 0 1px var(--line);
  color: var(--ink);
}

.tfa-card h2 {
  margin: 0 0 4px;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--ink);
}

.tfa-card p {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--muted);
}

.tfa-card strong {
  color: var(--ink);
  font-weight: 600;
}

.tfa-input {
  height: 46px;
  font-family: var(--font-mono);
  font-size: 20px;
  letter-spacing: 8px;
  text-align: center;
}

.tfa-back {
  width: 100%;
  margin-top: 8px;
  justify-content: center;
}
</style>
