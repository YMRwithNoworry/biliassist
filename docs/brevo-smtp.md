# 使用 Brevo 发送 Supabase 验证码

应用的邮箱验证码由 Supabase Auth 生成和校验。要改用 Brevo 发送邮件，请在 Supabase 控制台配置自定义 SMTP；客户端代码不需要保存 Brevo 密钥，也不需要改动 OTP 流程。

## Brevo 准备

1. 在 Brevo 中验证发件人邮箱或域名。
2. 打开 **Settings -> SMTP & API -> SMTP**，创建或查看 SMTP 凭据。
3. 记录 SMTP 登录名（通常是 Brevo 账号邮箱）和 SMTP Key。SMTP Key 不是 API Key。

## Supabase 配置

进入 **Project Settings -> Authentication -> SMTP Settings**，填写：

| 字段 | 值 |
| --- | --- |
| SMTP Host | `smtp-relay.brevo.com` |
| SMTP Port | `587`（STARTTLS） |
| SMTP User | Brevo SMTP 登录名 |
| SMTP Password | Brevo SMTP Key |
| Sender email | 已在 Brevo 验证的发件人 |
| Sender name | `BiliAssist`（或产品名称） |

保存后，在 Supabase 的 **Authentication -> Email Templates** 中确认 Magic Link/OTP 模板包含 `{{ .Token }}`，然后使用登录页发送验证码测试。

## 安全注意

- 不要把 SMTP Key、Brevo API Key 或 Supabase Service Role Key 提交到仓库。
- Supabase 的 SMTP 配置只在 Supabase 服务端保存；桌面客户端仍只使用公开的 Supabase URL 和 Anon Key。
- 生产环境应在 Brevo 和 Supabase 中配置已验证的域名及 SPF、DKIM、DMARC，避免验证码进入垃圾邮件。
