import { createClient } from '@supabase/supabase-js'
import { SUPABASE_URL, SUPABASE_ANON_KEY } from './config'

function createSupabaseClient() {
  const supabaseUrl = SUPABASE_URL
  const supabaseAnonKey = SUPABASE_ANON_KEY

  if (!supabaseUrl || !supabaseAnonKey) {
    console.warn(
      '[Supabase] 缺少配置，认证功能不可用。'
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
