<script setup>
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  BookOpen,
  ChevronRight,
  KeyRound,
  LogOut,
  Menu,
  MonitorSmartphone,
  Network,
  Rocket,
  ScrollText,
  Settings,
  ShieldCheck,
  UserCog,
  Users,
  Video,
  Wifi,
} from 'lucide-vue-next'
import {
  can,
  clearSession,
  displayName,
  session,
  userHandle,
} from '../lib/session'
import { initials } from '../lib/format'

const route = useRoute()
const router = useRouter()
const menuOpen = ref(false)

watch(
  () => route.path,
  () => {
    menuOpen.value = false
  }
)

const crumb = computed(() => route.meta.crumb || '设备')
const host = computed(() =>
  String(session.baseUrl || '').replace(/^https?:\/\//, '')
)

function logout() {
  clearSession()
  router.push('/login')
}
</script>

<template>
  <div class="shell">
    <div v-if="menuOpen" class="sidebar-scrim" @click="menuOpen = false" />
    <aside class="sidebar" :class="{ open: menuOpen }">
      <div class="brand">
        <div class="brand-mark">R</div>
        <div>
          <div class="brand-name">RustDesk</div>
          <span class="brand-tag">设备控制台</span>
        </div>
      </div>

      <nav class="nav" aria-label="主导航">
        <div class="nav-label">工作区</div>
        <RouterLink class="nav-link" to="/devices">
          <MonitorSmartphone :size="16" />
          设备
        </RouterLink>
        <RouterLink v-if="can('group.read')" class="nav-link" to="/groups">
          <Users :size="16" />
          分组
        </RouterLink>
        <RouterLink
          v-if="can('address_book.read')"
          class="nav-link"
          to="/address-books"
        >
          <BookOpen :size="16" />
          地址簿
        </RouterLink>

        <div class="nav-label">安全与合规</div>
        <RouterLink v-if="can('user.read')" class="nav-link" to="/users">
          <UserCog :size="16" />
          用户
        </RouterLink>
        <RouterLink v-if="can('role.manage')" class="nav-link" to="/roles">
          <ShieldCheck :size="16" />
          管理员角色
        </RouterLink>
        <RouterLink
          v-if="can('strategy.read')"
          class="nav-link"
          to="/strategies"
        >
          <KeyRound :size="16" />
          策略
        </RouterLink>
        <RouterLink v-if="can('audit.read')" class="nav-link" to="/audit">
          <ScrollText :size="16" />
          审计日志
        </RouterLink>
        <RouterLink v-if="can('audit.read')" class="nav-link" to="/logs">
          <ScrollText :size="16" />
          连接与文件日志
        </RouterLink>

        <div class="nav-label">运维</div>
        <RouterLink
          v-if="can('deployment.manage')"
          class="nav-link"
          to="/deployment"
        >
          <Rocket :size="16" />
          客户端部署
        </RouterLink>
        <RouterLink
          v-if="can('recording.read')"
          class="nav-link"
          to="/recordings"
        >
          <Video :size="16" />
          会话录像
        </RouterLink>
        <RouterLink v-if="can('relay.manage')" class="nav-link" to="/relays">
          <Network :size="16" />
          中继服务器
        </RouterLink>
        <RouterLink
          v-if="can('settings.read')"
          class="nav-link"
          to="/integrations"
        >
          <KeyRound :size="16" />
          身份与邮件
        </RouterLink>

        <div class="nav-label">系统</div>
        <RouterLink class="nav-link" to="/settings">
          <Settings :size="16" />
          设置
        </RouterLink>
      </nav>

      <div class="sidebar-foot">
        <div class="connection-chip">
          <Wifi :size="13" />
          <span class="host">{{ host }}</span>
        </div>
        <div class="user-row">
          <div class="user-avatar">{{ initials(displayName()) }}</div>
          <div class="user-meta">
            <div class="user-name">{{ displayName() }}</div>
            <div class="user-sub">{{ userHandle() }}</div>
          </div>
          <button
            class="icon-btn"
            type="button"
            title="退出登录"
            aria-label="退出登录"
            @click="logout"
          >
            <LogOut :size="15" />
          </button>
        </div>
      </div>
    </aside>

    <div class="main">
      <header class="topbar">
        <button
          class="icon-btn mobile-menu"
          type="button"
          aria-label="打开导航"
          @click="menuOpen = true"
        >
          <Menu :size="18" />
        </button>
        <div class="crumbs">
          <span>控制台</span>
          <ChevronRight :size="12" />
          <strong>{{ crumb }}</strong>
        </div>
      </header>

      <slot />
    </div>
  </div>
</template>
