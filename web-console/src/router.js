import { createRouter, createWebHashHistory } from 'vue-router'
import { session } from './lib/session'
import AddressBooksView from './views/AddressBooksView.vue'
import AuditView from './views/AuditView.vue'
import DevicesView from './views/DevicesView.vue'
import GroupsView from './views/GroupsView.vue'
import LoginView from './views/LoginView.vue'
import SettingsView from './views/SettingsView.vue'
import StrategiesView from './views/StrategiesView.vue'
import UsersView from './views/UsersView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: LoginView,
      meta: { public: true },
    },
    {
      path: '/devices',
      name: 'devices',
      component: DevicesView,
      meta: { crumb: '设备' },
    },
    {
      path: '/groups',
      name: 'groups',
      component: GroupsView,
      meta: { crumb: '分组' },
    },
    {
      path: '/address-books',
      name: 'address-books',
      component: AddressBooksView,
      meta: { crumb: '地址簿', admin: true },
    },
    {
      path: '/users',
      name: 'users',
      component: UsersView,
      meta: { crumb: '用户', admin: true },
    },
    {
      path: '/strategies',
      name: 'strategies',
      component: StrategiesView,
      meta: { crumb: '策略', admin: true },
    },
    {
      path: '/audit',
      name: 'audit',
      component: AuditView,
      meta: { crumb: '审计日志', admin: true },
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsView,
      meta: { crumb: '设置' },
    },
    {
      path: '/control/:guid',
      name: 'web-control',
      component: () => import('./views/WebControlView.vue'),
      meta: { crumb: '远程控制' },
    },
    { path: '/', redirect: '/devices' },
    { path: '/:pathMatch(.*)*', redirect: '/devices' },
  ],
})

router.beforeEach((to) => {
  if (!to.meta.public && !session.token) {
    return {
      path: '/login',
      query: to.fullPath === '/' ? undefined : { redirect: to.fullPath },
    }
  }
  if (to.path === '/login' && session.token) return '/devices'
  if (to.meta.admin && session.user?.is_admin !== true) return '/devices'
  return true
})

export default router
