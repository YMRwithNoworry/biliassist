import { createRouter, createWebHashHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/auth',
      name: 'auth',
      component: () => import('../views/AuthPage.vue')
    },
    {
      path: '/payment',
      name: 'payment',
      component: () => import('../views/PaymentPage.vue')
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
    try {
      await Promise.race([
        auth.getSession(),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('timeout')), 8000)
        )
      ])
    } catch {
      auth.loading = false
    }
  }

  // Auth guard
  if (to.name === 'auth') {
    if (auth.isAuthenticated) {
      next({ name: 'home' })
    } else {
      next()
    }
    return
  }

  if (!auth.isAuthenticated) {
    next({ name: 'auth' })
    return
  }

  // Tier guard: only Plus users can use the app
  if (to.name !== 'payment') {
    if (!auth.tierChecked) {
      await auth.checkTier()
    }
    if (!auth.isPlus) {
      next({ name: 'payment' })
      return
    }
  }

  next()
})

export default router
