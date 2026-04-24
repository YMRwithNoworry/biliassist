-- BilibiliAccountManager 数据同步表
-- 在 Supabase SQL Editor 中执行此脚本

-- 1. B站账号同步表
CREATE TABLE IF NOT EXISTS bilibili_accounts (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  uid TEXT NOT NULL,
  name TEXT NOT NULL,
  avatar TEXT NOT NULL DEFAULT '',
  cookie TEXT NOT NULL DEFAULT '',
  active BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(user_id, uid)
);

-- 2. 自动回复设置同步表
CREATE TABLE IF NOT EXISTS auto_reply_settings (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE UNIQUE,
  settings JSONB NOT NULL DEFAULT '{}',
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 3. 启用 Row Level Security
ALTER TABLE bilibili_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE auto_reply_settings ENABLE ROW LEVEL SECURITY;

-- 4. RLS 策略：用户只能操作自己的数据
CREATE POLICY "用户只能查看自己的 B站账号" ON bilibili_accounts
  FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY "用户只能插入自己的 B站账号" ON bilibili_accounts
  FOR INSERT WITH CHECK (auth.uid() = user_id);

CREATE POLICY "用户只能更新自己的 B站账号" ON bilibili_accounts
  FOR UPDATE USING (auth.uid() = user_id);

CREATE POLICY "用户只能删除自己的 B站账号" ON bilibili_accounts
  FOR DELETE USING (auth.uid() = user_id);

-- 自动回复设置 RLS
CREATE POLICY "用户只能查看自己的自动回复设置" ON auto_reply_settings
  FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY "用户只能插入自己的自动回复设置" ON auto_reply_settings
  FOR INSERT WITH CHECK (auth.uid() = user_id);

CREATE POLICY "用户只能更新自己的自动回复设置" ON auto_reply_settings
  FOR UPDATE USING (auth.uid() = user_id);

-- 5. 自动更新 updated_at 的触发器
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER bilibili_accounts_updated_at
  BEFORE UPDATE ON bilibili_accounts
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER auto_reply_settings_updated_at
  BEFORE UPDATE ON auto_reply_settings
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();
