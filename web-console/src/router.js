import { createRouter, createWebHashHistory } from 'vue-router'
import { session } from './lib/session'
import DevicesView from './views/DevicesView.vue'
import GroupsView from './views/GroupsView.vue'
import LoginView from './views/LoginView.vue'
import SettingsView from './views/SettingsView.vue'

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
      path: '/settings',
      name: 'settings',
      component: SettingsView,
      meta: { crumb: '设置' },
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
  return true
})

export default router
