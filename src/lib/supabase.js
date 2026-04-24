import { createClient } from '@supabase/supabase-js'

function createSupabaseClient() {
  const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
  const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

  if (!supabaseUrl || !supabaseAnonKey) {
    console.warn(
      '[Supabase] 缺少配置（VITE_SUPABASE_URL / VITE_SUPABASE_ANON_KEY），认证功能不可用。'
    )
    return null
  }

  try {
    return createClient(supabaseUrl, supabaseAnonKey)
  } catch (e) {
    console.error('[Supabase] 客户端创建失败:', e)
    return null
  }
}

export const supabase = createSupabaseClient()
