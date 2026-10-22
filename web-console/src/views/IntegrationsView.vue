<script setup>
import { computed, onMounted, ref } from 'vue'
import {
  CheckCircle2,
  Fingerprint,
  Mail,
  Network,
  RefreshCw,
  Save,
  Send,
  TestTube2,
  WifiOff,
  XCircle,
} from 'lucide-vue-next'
import AppShell from '../components/AppShell.vue'
import { getApi } from '../lib/api'
import { can } from '../lib/session'
import { notify } from '../lib/toast'

const api = getApi()

const TABS = [
  {
    key: 'oidc',
    label: 'OIDC 单点登录',
    hint: '对接 Keycloak、Authentik、Azure AD 等',
    icon: Fingerprint,
  },
  {
    key: 'smtp',
    label: 'SMTP 邮件',
    hint: '邮箱验证与安全告警通知',
    icon: Mail,
  },
  {
    key: 'ldap',
    label: 'LDAP 目录',
    hint: '使用企业目录账号登录',
    icon: Network,
  },
]

const tab = ref('oidc')
const loading = ref(true)
const error = ref('')

const oidc = ref({
  enabled: false,
  name: '',
  issuer: '',
  client_id: '',
  client_secret: '',
  client_secret_set: false,
  scopes: '',
  redirect_url: '',
  auto_create: true,
  username_claim: '',
})
const smtp = ref({
  enabled: false,
  host: '',
  port: 587,
  username: '',
  password: '',
  password_set: false,
  from: '',
  from_name: '',
  encryption: 'starttls',
})
const ldap = ref({
  enabled: false,
  host: '',
  port: 389,
  encryption: 'none',
  bind_dn: '',
  bind_password: '',
  bind_password_set: false,
  base_dn: '',
  user_filter: '',
  auto_create: true,
})

const clearOidcSecret = ref(false)
const clearSmtpPassword = ref(false)
const clearLdapPassword = ref(false)

const oidcBusy = ref(false)
const oidcTestBusy = ref(false)
const oidcResult = ref(null)
const smtpBusy = ref(false)
const smtpTestBusy = ref(false)
const smtpResult = ref(null)
const smtpTestTo = ref('')
const ldapBusy = ref(false)
const ldapTestBusy = ref(false)
const ldapResult = ref(null)
const ldapTestUser = ref('')
const ldapTestPassword = ref('')

const writable = computed(() => can('settings.write'))
const callbackUrl = computed(
  () => `${window.location.origin}/api/oidc/callback`
)

onMounted(load)

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true
  error.value = ''
  try {
    const [oidcData, smtpData, ldapData] = await Promise.all([
      api.getOidcSettings(),
      api.getSmtpSettings(),
      api.getLdapSettings(),
    ])
    oidc.value = {
      ...oidc.value,
      ...oidcData,
      client_secret: '',
      client_secret_set: oidcData?.client_secret_set === true,
    }
    smtp.value = {
      ...smtp.value,
      ...smtpData,
      password: '',
      password_set: smtpData?.password_set === true,
    }
    ldap.value = {
      ...ldap.value,
      ...ldapData,
      bind_password: '',
      bind_password_set: ldapData?.bind_password_set === true,
    }
  } catch (e) {
    error.value = e.message || '集成设置加载失败。'
  } finally {
    loading.value = false
  }
}

function resultTone(result) {
  if (!result) return ''
  return result.success ? 'result-ok' : 'result-error'
}

function resultText(result) {
  if (!result) return ''
  if (result.success) {
    if (result.message) return result.message
    if (result.dn) return `认证成功：${result.dn}`
    if (result.issuer) {
      return `发现配置成功：${result.issuer}${
        result.authorization_endpoint
          ? ` · 授权端点 ${result.authorization_endpoint}`
          : ''
      }`
    }
    return '测试成功。'
  }
  return result.error || '测试失败。'
}

async function saveOidc() {
  oidcBusy.value = true
  oidcResult.value = null
  try {
    await api.saveOidcSettings({
      enabled: oidc.value.enabled,
      name: oidc.value.name,
      issuer: oidc.value.issuer,
      client_id: oidc.value.client_id,
      client_secret: oidc.value.client_secret,
      clear_secret: clearOidcSecret.value,
      scopes: oidc.value.scopes,
      redirect_url: oidc.value.redirect_url,
      auto_create: oidc.value.auto_create,
      username_claim: oidc.value.username_claim,
    })
    notify('OIDC 设置已保存。')
    clearOidcSecret.value = false
    await load(false)
  } catch (e) {
    notify(e.message || 'OIDC 设置保存失败。', 'error')
  } finally {
    oidcBusy.value = false
  }
}

async function testOidc() {
  oidcTestBusy.value = true
  oidcResult.value = null
  try {
    oidcResult.value = await api.testOidc()
  } catch (e) {
    oidcResult.value = { success: false, error: e.message }
  } finally {
    oidcTestBusy.value = false
  }
}

async function saveSmtp() {
  smtpBusy.value = true
  smtpResult.value = null
  try {
    await api.saveSmtpSettings({
      enabled: smtp.value.enabled,
      host: smtp.value.host,
      port: Number(smtp.value.port) || 587,
      username: smtp.value.username,
      password: smtp.value.password,
      clear_password: clearSmtpPassword.value,
      from: smtp.value.from,
      from_name: smtp.value.from_name,
      encryption: smtp.value.encryption,
    })
    notify('SMTP 设置已保存。')
    clearSmtpPassword.value = false
    await load(false)
  } catch (e) {
    notify(e.message || 'SMTP 设置保存失败。', 'error')
  } finally {
    smtpBusy.value = false
  }
}

async function testSmtp() {
  smtpTestBusy.value = true
  smtpResult.value = null
  try {
    smtpResult.value = await api.testSmtp(smtpTestTo.value.trim(), {
      enabled: smtp.value.enabled,
      host: smtp.value.host,
      port: Number(smtp.value.port) || 587,
      username: smtp.value.username,
      password: smtp.value.password,
      from: smtp.value.from,
      from_name: smtp.value.from_name,
      encryption: smtp.value.encryption,
    })
  } catch (e) {
    smtpResult.value = { success: false, error: e.message }
  } finally {
    smtpTestBusy.value = false
  }
}

async function saveLdap() {
  ldapBusy.value = true
  ldapResult.value = null
  try {
    await api.saveLdapSettings({
      enabled: ldap.value.enabled,
      host: ldap.value.host,
      port: Number(ldap.value.port) || 389,
      encryption: ldap.value.encryption,
      bind_dn: ldap.value.bind_dn,
      bind_password: ldap.value.bind_password,
      clear_password: clearLdapPassword.value,
      base_dn: ldap.value.base_dn,
      user_filter: ldap.value.user_filter,
      auto_create: ldap.value.auto_create,
    })
    notify('LDAP 设置已保存。')
    clearLdapPassword.value = false
    await load(false)
  } catch (e) {
    notify(e.message || 'LDAP 设置保存失败。', 'error')
  } finally {
    ldapBusy.value = false
  }
}

async function testLdap() {
  ldapTestBusy.value = true
  ldapResult.value = null
  try {
    ldapResult.value = await api.testLdap(
      ldapTestUser.value.trim(),
      ldapTestPassword.value,
      {
        enabled: ldap.value.enabled,
        host: ldap.value.host,
        port: Number(ldap.value.port) || 389,
        encryption: ldap.value.encryption,
        bind_dn: ldap.value.bind_dn,
        bind_password: ldap.value.bind_password,
        base_dn: ldap.value.base_dn,
        user_filter: ldap.value.user_filter,
        auto_create: ldap.value.auto_create,
      }
    )
  } catch (e) {
    ldapResult.value = { success: false, error: e.message }
  } finally {
    ldapTestBusy.value = false
  }
}
</script>

<template>
  <AppShell>
    <main class="page">
      <div class="page-head">
        <div>
          <div class="page-eyebrow">系统</div>
          <h1 class="page-title">身份与邮件</h1>
          <p class="page-sub">
            配置单点登录、邮件服务与 LDAP 目录，用于账号登录、邮箱验证和安全告警通知。
          </p>
        </div>
        <div class="page-actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="loading"
            @click="load()"
          >
            <RefreshCw :size="15" />
            重新读取
          </button>
        </div>
      </div>

      <div class="int-tabs" role="tablist" aria-label="集成类型">
        <button
          v-for="item in TABS"
          :key="item.key"
          class="int-tab"
          :class="{ active: tab === item.key }"
          type="button"
          role="tab"
          :aria-selected="tab === item.key"
          @click="tab = item.key"
        >
          <component :is="item.icon" :size="16" />
          <span class="int-tab-text">
            <strong>{{ item.label }}</strong>
            <small>{{ item.hint }}</small>
          </span>
        </button>
      </div>

      <div v-if="loading" class="panel">
        <div class="loading-state">
          <span class="spinner-dark" />
          正在加载集成设置
        </div>
      </div>

      <div v-else-if="error" class="panel">
        <div class="empty">
          <div class="empty-icon"><WifiOff :size="18" /></div>
          <h3>无法加载集成设置</h3>
          <p>{{ error }}</p>
          <button class="btn btn-secondary" type="button" @click="load()">
            重试
          </button>
        </div>
      </div>

      <div v-else class="settings-stack">
        <section v-if="tab === 'oidc'" class="settings-card">
          <div class="section-head">
            <div>
              <h3>OIDC 单点登录</h3>
              <p class="desc">
                支持标准 OpenID Connect 提供方。网页控制台与 RustDesk 客户端会使用同一个回调地址完成登录。
              </p>
            </div>
            <Fingerprint :size="16" class="faint" />
          </div>

          <label class="check-row">
            <input v-model="oidc.enabled" type="checkbox" :disabled="!writable" />
            <span>
              启用 OIDC 登录
              <small>启用后，登录页会显示单点登录按钮。</small>
            </span>
          </label>

          <div class="form-grid">
            <div class="field">
              <label class="label" for="oidc-name">显示名称</label>
              <input
                id="oidc-name"
                v-model="oidc.name"
                class="input"
                placeholder="例如：企业单点登录"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="oidc-issuer">签发者地址（Issuer）</label>
              <input
                id="oidc-issuer"
                v-model="oidc.issuer"
                class="input mono"
                placeholder="https://auth.example.com/realms/main"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="oidc-client-id">Client ID</label>
              <input
                id="oidc-client-id"
                v-model="oidc.client_id"
                class="input mono"
                placeholder="rustdesk"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="oidc-secret">Client Secret</label>
              <input
                id="oidc-secret"
                v-model="oidc.client_secret"
                class="input"
                type="password"
                autocomplete="new-password"
                :placeholder="
                  oidc.client_secret_set
                    ? '已保存，留空保持不变'
                    : '填写客户端密钥'
                "
                :disabled="!writable"
              />
              <label
                v-if="oidc.client_secret_set"
                class="secret-clear"
              >
                <input
                  v-model="clearOidcSecret"
                  type="checkbox"
                  :disabled="!writable"
                />
                清除已保存的密钥
              </label>
            </div>
            <div class="field">
              <label class="label" for="oidc-scopes">
                Scopes <span class="hint">可选</span>
              </label>
              <input
                id="oidc-scopes"
                v-model="oidc.scopes"
                class="input mono"
                placeholder="openid profile email"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="oidc-claim">
                用户名 Claim <span class="hint">可选</span>
              </label>
              <input
                id="oidc-claim"
                v-model="oidc.username_claim"
                class="input mono"
                placeholder="默认使用 preferred_username"
                :disabled="!writable"
              />
            </div>
            <div class="field form-span">
              <label class="label" for="oidc-redirect">
                回调地址 <span class="hint">留空自动使用当前域名</span>
              </label>
              <input
                id="oidc-redirect"
                v-model="oidc.redirect_url"
                class="input mono"
                :placeholder="callbackUrl"
                :disabled="!writable"
              />
              <span class="field-hint">
                请在 OIDC 提供方中登记回调地址：
                <code>{{ oidc.redirect_url || callbackUrl }}</code>
              </span>
            </div>
          </div>

          <label class="check-row">
            <input
              v-model="oidc.auto_create"
              type="checkbox"
              :disabled="!writable"
            />
            <span>
              自动创建本地账号
              <small>
                关闭后，只有已经存在且邮箱匹配的账号才能通过 OIDC 登录。
              </small>
            </span>
          </label>

          <div v-if="oidcResult" class="test-result" :class="resultTone(oidcResult)">
            <CheckCircle2 v-if="oidcResult.success" :size="14" />
            <XCircle v-else :size="14" />
            <span>{{ resultText(oidcResult) }}</span>
          </div>

          <div class="settings-actions">
            <button
              class="btn btn-secondary"
              type="button"
              :disabled="oidcTestBusy || !writable"
              @click="testOidc"
            >
              <TestTube2 :size="14" />
              {{ oidcTestBusy ? '测试中…' : '测试发现配置' }}
            </button>
            <button
              class="btn btn-primary"
              type="button"
              :disabled="oidcBusy || !writable"
              @click="saveOidc"
            >
              <Save :size="14" />
              {{ oidcBusy ? '保存中…' : '保存 OIDC 设置' }}
            </button>
          </div>
          <p class="desc" style="margin-top: 10px">
            测试使用已保存的配置。修改后请先保存，再点击测试。
          </p>
        </section>

        <section v-else-if="tab === 'smtp'" class="settings-card">
          <div class="section-head">
            <div>
              <h3>SMTP 邮件</h3>
              <p class="desc">
                用于发送邮箱验证码、安全告警邮件与测试邮件。密码仅保存在服务器数据库中，不会回显。
              </p>
            </div>
            <Mail :size="16" class="faint" />
          </div>

          <label class="check-row">
            <input v-model="smtp.enabled" type="checkbox" :disabled="!writable" />
            <span>
              启用邮件发送
              <small>关闭后，邮箱验证步骤会自动跳过，告警邮件也不会发送。</small>
            </span>
          </label>

          <div class="form-grid">
            <div class="field">
              <label class="label" for="smtp-host">SMTP 服务器</label>
              <input
                id="smtp-host"
                v-model="smtp.host"
                class="input mono"
                placeholder="smtp.example.com"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="smtp-port">端口</label>
              <input
                id="smtp-port"
                v-model.number="smtp.port"
                class="input mono"
                type="number"
                min="1"
                max="65535"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="smtp-username">
                用户名 <span class="hint">可选</span>
              </label>
              <input
                id="smtp-username"
                v-model="smtp.username"
                class="input mono"
                placeholder="no-reply@example.com"
                autocomplete="off"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="smtp-password">密码</label>
              <input
                id="smtp-password"
                v-model="smtp.password"
                class="input"
                type="password"
                autocomplete="new-password"
                :placeholder="
                  smtp.password_set ? '已保存，留空保持不变' : '填写邮箱密码或授权码'
                "
                :disabled="!writable"
              />
              <label v-if="smtp.password_set" class="secret-clear">
                <input
                  v-model="clearSmtpPassword"
                  type="checkbox"
                  :disabled="!writable"
                />
                清除已保存的密码
              </label>
            </div>
            <div class="field">
              <label class="label" for="smtp-from">发件人地址</label>
              <input
                id="smtp-from"
                v-model="smtp.from"
                class="input mono"
                type="email"
                placeholder="no-reply@example.com"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="smtp-from-name">
                发件人名称 <span class="hint">可选</span>
              </label>
              <input
                id="smtp-from-name"
                v-model="smtp.from_name"
                class="input"
                placeholder="RustDesk 服务"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="smtp-encryption">加密方式</label>
              <select
                id="smtp-encryption"
                v-model="smtp.encryption"
                class="select"
                :disabled="!writable"
              >
                <option value="starttls">STARTTLS（常用，587）</option>
                <option value="tls">TLS（465）</option>
                <option value="none">不加密</option>
              </select>
            </div>
            <div class="field">
              <label class="label" for="smtp-test-to">
                测试收件人 <span class="hint">留空发送给当前账号</span>
              </label>
              <div class="test-row">
                <input
                  id="smtp-test-to"
                  v-model="smtpTestTo"
                  class="input"
                  type="email"
                  placeholder="you@example.com"
                />
                <button
                  class="btn btn-secondary"
                  type="button"
                  :disabled="smtpTestBusy || !writable"
                  @click="testSmtp"
                >
                  <Send :size="14" />
                  {{ smtpTestBusy ? '发送中…' : '发送测试' }}
                </button>
              </div>
            </div>
          </div>

          <div v-if="smtpResult" class="test-result" :class="resultTone(smtpResult)">
            <CheckCircle2 v-if="smtpResult.success" :size="14" />
            <XCircle v-else :size="14" />
            <span>{{ resultText(smtpResult) }}</span>
          </div>

          <div class="settings-actions">
            <button
              class="btn btn-primary"
              type="button"
              :disabled="smtpBusy || !writable"
              @click="saveSmtp"
            >
              <Save :size="14" />
              {{ smtpBusy ? '保存中…' : '保存 SMTP 设置' }}
            </button>
          </div>
        </section>

        <section v-else class="settings-card">
          <div class="section-head">
            <div>
              <h3>LDAP 目录</h3>
              <p class="desc">
                使用企业 LDAP 或 Active Directory 账号登录。支持服务账号搜索后绑定用户，首次登录可自动创建本地账号。
              </p>
            </div>
            <Network :size="16" class="faint" />
          </div>

          <label class="check-row">
            <input v-model="ldap.enabled" type="checkbox" :disabled="!writable" />
            <span>
              启用 LDAP 登录
              <small>本地账号仍可正常登录；LDAP 用户由目录服务校验密码。</small>
            </span>
          </label>

          <div class="form-grid">
            <div class="field">
              <label class="label" for="ldap-host">LDAP 服务器</label>
              <input
                id="ldap-host"
                v-model="ldap.host"
                class="input mono"
                placeholder="ldap.example.com"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="ldap-port">端口</label>
              <input
                id="ldap-port"
                v-model.number="ldap.port"
                class="input mono"
                type="number"
                min="1"
                max="65535"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="ldap-encryption">加密方式</label>
              <select
                id="ldap-encryption"
                v-model="ldap.encryption"
                class="select"
                :disabled="!writable"
              >
                <option value="none">不加密（389）</option>
                <option value="ldaps">LDAPS（636）</option>
              </select>
              <span class="field-hint">
                当前版本仅支持 LDAP 与 LDAPS，不支持 StartTLS。
              </span>
            </div>
            <div class="field">
              <label class="label" for="ldap-filter">
                用户过滤器
                <span class="hint">使用 {username} 占位</span>
              </label>
              <input
                id="ldap-filter"
                v-model="ldap.user_filter"
                class="input mono"
                placeholder="(uid={username})"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="ldap-bind-dn">
                绑定 DN <span class="hint">可选</span>
              </label>
              <input
                id="ldap-bind-dn"
                v-model="ldap.bind_dn"
                class="input mono"
                placeholder="cn=admin,dc=example,dc=com"
                :disabled="!writable"
              />
            </div>
            <div class="field">
              <label class="label" for="ldap-bind-password">绑定密码</label>
              <input
                id="ldap-bind-password"
                v-model="ldap.bind_password"
                class="input"
                type="password"
                autocomplete="new-password"
                :placeholder="
                  ldap.bind_password_set
                    ? '已保存，留空保持不变'
                    : '填写绑定账号密码'
                "
                :disabled="!writable"
              />
              <label v-if="ldap.bind_password_set" class="secret-clear">
                <input
                  v-model="clearLdapPassword"
                  type="checkbox"
                  :disabled="!writable"
                />
                清除已保存的密码
              </label>
            </div>
            <div class="field form-span">
              <label class="label" for="ldap-base-dn">基准 DN</label>
              <input
                id="ldap-base-dn"
                v-model="ldap.base_dn"
                class="input mono"
                placeholder="ou=users,dc=example,dc=com"
                :disabled="!writable"
              />
              <span class="field-hint">
                例如 Active Directory 可填写
                <code>DC=example,DC=com</code>，过滤器使用
                <code>(sAMAccountName={username})</code>。
              </span>
            </div>
          </div>

          <label class="check-row">
            <input
              v-model="ldap.auto_create"
              type="checkbox"
              :disabled="!writable"
            />
            <span>
              首次登录自动创建账号
              <small>关闭后，需要先在用户页创建同名账号才能使用 LDAP 登录。</small>
            </span>
          </label>

          <div class="form-grid" style="margin-top: 14px">
            <div class="field">
              <label class="label" for="ldap-test-user">
                测试用户名 <span class="hint">可选</span>
              </label>
              <input
                id="ldap-test-user"
                v-model="ldapTestUser"
                class="input mono"
                placeholder="alice"
              />
            </div>
            <div class="field">
              <label class="label" for="ldap-test-password">
                测试密码 <span class="hint">填写后验证用户认证</span>
              </label>
              <div class="test-row">
                <input
                  id="ldap-test-password"
                  v-model="ldapTestPassword"
                  class="input"
                  type="password"
                  autocomplete="off"
                  placeholder="留空仅测试服务绑定"
                />
                <button
                  class="btn btn-secondary"
                  type="button"
                  :disabled="ldapTestBusy || !writable"
                  @click="testLdap"
                >
                  <TestTube2 :size="14" />
                  {{ ldapTestBusy ? '测试中…' : '测试连接' }}
                </button>
              </div>
            </div>
          </div>

          <div v-if="ldapResult" class="test-result" :class="resultTone(ldapResult)">
            <CheckCircle2 v-if="ldapResult.success" :size="14" />
            <XCircle v-else :size="14" />
            <span>{{ resultText(ldapResult) }}</span>
          </div>

          <div class="settings-actions">
            <button
              class="btn btn-primary"
              type="button"
              :disabled="ldapBusy || !writable"
              @click="saveLdap"
            >
              <Save :size="14" />
              {{ ldapBusy ? '保存中…' : '保存 LDAP 设置' }}
            </button>
          </div>
        </section>

        <p v-if="!writable" class="desc">
          当前账号只有查看权限，修改设置需要「修改系统设置」权限。
        </p>
      </div>
    </main>
  </AppShell>
</template>

<style scoped>
.int-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-bottom: 16px;
}

.int-tab {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 13px 15px;
  border: 1px solid var(--line);
  border-radius: var(--radius-md);
  background: var(--canvas);
  color: var(--muted);
  text-align: left;
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}

.int-tab:hover {
  background: var(--wash);
  color: var(--ink);
}

.int-tab.active {
  border-color: var(--ink);
  background: var(--wash);
  color: var(--ink);
}

.int-tab-text {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.int-tab-text strong {
  font-size: 12.5px;
  font-weight: 600;
}

.int-tab-text small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10.5px;
  color: var(--faint);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 14px;
}

.form-span {
  grid-column: 1 / -1;
}

.field-hint {
  display: block;
  margin-top: 5px;
  font-size: 10.5px;
  line-height: 1.6;
  color: var(--faint);
}

.field-hint code {
  font-family: var(--font-mono);
  color: var(--muted);
  overflow-wrap: anywhere;
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

.secret-clear {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  font-size: 10.5px;
  color: var(--faint);
  cursor: pointer;
}

.secret-clear input {
  accent-color: var(--ink);
}

.test-row {
  display: flex;
  gap: 8px;
}

.test-row .input {
  flex: 1;
}

.test-result {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  font-size: 11.5px;
  line-height: 1.6;
  overflow-wrap: anywhere;
}

.result-ok {
  background: var(--green-wash);
  color: var(--green);
}

.result-error {
  background: var(--red-wash);
  color: var(--red);
}

@media (max-width: 960px) {
  .int-tabs {
    grid-template-columns: 1fr;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
