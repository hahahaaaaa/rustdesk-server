<script setup>
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  BookOpen,
  ChevronRight,
  LogOut,
  Menu,
  MonitorSmartphone,
  ScrollText,
  Settings,
  ShieldCheck,
  UserCog,
  Users,
  Wifi,
} from 'lucide-vue-next'
import {
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
  session.demo
    ? '演示工作区'
    : String(session.baseUrl || '').replace(/^https?:\/\//, '')
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
        <RouterLink class="nav-link" to="/groups">
          <Users :size="16" />
          分组
        </RouterLink>
        <RouterLink
          v-if="session.user?.is_admin"
          class="nav-link"
          to="/address-books"
        >
          <BookOpen :size="16" />
          地址簿
        </RouterLink>
        <div class="nav-label">系统</div>
        <RouterLink v-if="session.user?.is_admin" class="nav-link" to="/users">
          <UserCog :size="16" />
          用户
        </RouterLink>
        <RouterLink
          v-if="session.user?.is_admin"
          class="nav-link"
          to="/strategies"
        >
          <ShieldCheck :size="16" />
          策略
        </RouterLink>
        <RouterLink
          v-if="session.user?.is_admin"
          class="nav-link"
          to="/audit"
        >
          <ScrollText :size="16" />
          审计日志
        </RouterLink>
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
        <div class="topbar-right">
          <span v-if="session.demo" class="demo-badge">演示数据</span>
        </div>
      </header>

      <slot />
    </div>
  </div>
</template>
