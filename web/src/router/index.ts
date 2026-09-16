import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('../views/LoginView.vue'),
      meta: { public: true },
    },
    {
      path: '/',
      component: () => import('../components/layout/AppLayout.vue'),
      children: [
        {
          path: '',
          name: 'dashboard',
          component: () => import('../views/DashboardView.vue'),
        },
        {
          path: 'address-book',
          name: 'address-book',
          component: () => import('../views/AddressBookView.vue'),
        },
        {
          path: 'users',
          name: 'users',
          component: () => import('../views/UsersView.vue'),
          meta: { admin: true },
        },
        {
          path: 'shared-books',
          name: 'shared-books',
          component: () => import('../views/SharedBooksView.vue'),
          meta: { admin: true },
        },
        {
          path: 'devices',
          name: 'devices',
          component: () => import('../views/DevicesView.vue'),
          meta: { admin: true },
        },
        {
          path: 'groups',
          name: 'groups',
          component: () => import('../views/GroupsView.vue'),
          meta: { admin: true },
        },
        {
          path: 'audit-log',
          name: 'audit-log',
          component: () => import('../views/AuditLogView.vue'),
          meta: { admin: true },
        },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  if (!to.meta.public && !auth.isAuthenticated) {
    return { name: 'login' }
  }
  if (to.name === 'login' && auth.isAuthenticated) {
    return { name: 'dashboard' }
  }
})

export default router
