import { createClient } from '@supabase/supabase-js'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

if (!supabaseUrl || !supabaseAnonKey) {
  console.error(
    '缺少 Supabase 配置！请在项目根目录创建 .env 文件，并参考 .env.example 设置 VITE_SUPABASE_URL 和 VITE_SUPABASE_ANON_KEY'
  )
}

export const supabase = createClient(supabaseUrl || '', supabaseAnonKey || '')
