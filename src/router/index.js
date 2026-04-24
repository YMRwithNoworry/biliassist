import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/auth',
      name: 'auth',
      component: () => import('../views/AuthPage.vue')
    },
    {
      path: '/',
      name: 'home',
      component: HomeView,
      meta: { requiresAuth: true }
    },
    {
      path: '/login',
      name: 'bilibili-login',
      component: () => import('../views/LoginView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/accounts',
      name: 'accounts',
      component: () => import('../views/AccountsView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/auto-reply',
      name: 'auto-reply',
      component: () => import('../views/AutoReplyView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/sponsor',
      name: 'sponsor',
      component: () => import('../views/SponsorView.vue'),
      meta: { requiresAuth: true }
    }
  ]
})

router.beforeEach(async (to, from, next) => {
  const auth = useAuthStore()

  if (auth.loading) {
    await auth.getSession()
  }

  if (to.meta.requiresAuth && !auth.isAuthenticated) {
    next({ name: 'auth', query: { redirect: to.fullPath } })
  } else if (to.name === 'auth' && auth.isAuthenticated) {
    next({ name: 'home' })
  } else {
    next()
  }
})

export default router
