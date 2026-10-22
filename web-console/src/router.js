import { createRouter, createWebHashHistory } from 'vue-router'
import { can, session } from './lib/session'
import AddressBooksView from './views/AddressBooksView.vue'
import AuditView from './views/AuditView.vue'
import DeploymentView from './views/DeploymentView.vue'
import DevicesView from './views/DevicesView.vue'
import GroupsView from './views/GroupsView.vue'
import IntegrationsView from './views/IntegrationsView.vue'
import LoginView from './views/LoginView.vue'
import LogsView from './views/LogsView.vue'
import RecordingsView from './views/RecordingsView.vue'
import RelaysView from './views/RelaysView.vue'
import RolesView from './views/RolesView.vue'
import SettingsView from './views/SettingsView.vue'
import StrategiesView from './views/StrategiesView.vue'
import UsersView from './views/UsersView.vue'

const LANDING = [
  ['/devices', ''],
  ['/groups', 'group.read'],
  ['/address-books', 'address_book.read'],
  ['/strategies', 'strategy.read'],
  ['/audit', 'audit.read'],
  ['/logs', 'audit.read'],
  ['/users', 'user.read'],
  ['/roles', 'role.manage'],
  ['/deployment', 'deployment.manage'],
  ['/recordings', 'recording.read'],
  ['/relays', 'relay.manage'],
  ['/integrations', 'settings.read'],
  ['/settings', ''],
]

function landingPath() {
  const match = LANDING.find(([, permission]) => !permission || can(permission))
  return match ? match[0] : '/devices'
}

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
      meta: { crumb: '地址簿', permission: 'address_book.read' },
    },
    {
      path: '/users',
      name: 'users',
      component: UsersView,
      meta: { crumb: '用户', permission: 'user.read' },
    },
    {
      path: '/roles',
      name: 'roles',
      component: RolesView,
      meta: { crumb: '管理员角色', permission: 'role.manage' },
    },
    {
      path: '/strategies',
      name: 'strategies',
      component: StrategiesView,
      meta: { crumb: '策略', permission: 'strategy.read' },
    },
    {
      path: '/audit',
      name: 'audit',
      component: AuditView,
      meta: { crumb: '审计日志', permission: 'audit.read' },
    },
    {
      path: '/logs',
      name: 'logs',
      component: LogsView,
      meta: { crumb: '连接与文件日志', permission: 'audit.read' },
    },
    {
      path: '/deployment',
      name: 'deployment',
      component: DeploymentView,
      meta: { crumb: '客户端部署', permission: 'deployment.manage' },
    },
    {
      path: '/recordings',
      name: 'recordings',
      component: RecordingsView,
      meta: { crumb: '会话录像', permission: 'recording.read' },
    },
    {
      path: '/relays',
      name: 'relays',
      component: RelaysView,
      meta: { crumb: '中继服务器', permission: 'relay.manage' },
    },
    {
      path: '/integrations',
      name: 'integrations',
      component: IntegrationsView,
      meta: { crumb: '身份与邮件', permission: 'settings.read' },
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
  if (to.path === '/login' && session.token) return landingPath()
  if (to.meta.permission && !can(to.meta.permission)) return landingPath()
  return true
})

export default router
