# API 文档

## Tauri Commands

### 1. 扫码登录相关

#### get_qr_code
获取 B站登录二维码

**返回值：**
```typescript
{
  qrcode: string;      // base64 编码的二维码图片
  qrcode_key: string;  // 二维码唯一标识
}
```

#### check_login_status
检查扫码登录状态

**返回值：**
```typescript
{
  status: string;      // "waiting" | "scanned" | "success" | "expired"
  user_info?: {
    uid: string;
    name: string;
    cookie: string;
  }
}
```

### 2. 账号管理相关

#### get_accounts
获取所有已保存的账号

**返回值：**
```typescript
{
  uid: string;
  name: string;
  cookie: string;
  active: boolean;
  created_at: string;
}[]
```

#### activate_account
激活指定账号

**参数：**
- `uid: string` - 账号 UID

**返回值：** `void`

#### delete_account
删除指定账号

**参数：**
- `uid: string` - 账号 UID

**返回值：** `void`

### 3. 自动回复相关

#### get_auto_reply_settings
获取自动回复配置

**返回值：**
```typescript
{
  enabled: boolean;
  message: string;
  interval: number;
  reply_only_once: boolean;
  history: {
    user: string;
    time: string;
    message: string;
  }[];
}
```

#### save_auto_reply_settings
保存自动回复配置

**参数：**
```typescript
{
  enabled: boolean;
  message: string;
  interval: number;
  replyOnlyOnce: boolean;
}
```

**返回值：** `void`

#### test_auto_reply
测试自动回复内容

**返回值：** `string` - 格式化后的回复内容

## B站 API

### 扫码登录流程

1. 生成二维码
   - GET `https://passport.bilibili.com/x/passport-login/web/qrcode/generate`
   - 参数：`appkey`, `local_id`

2. 轮询登录状态
   - GET `https://passport.bilibili.com/x/passport-login/web/qrcode/poll`
   - 参数：`qrcode_key`

3. 获取用户信息
   - GET `https://api.bilibili.com/x/web-interface/nav`

### 私信相关

1. 获取私信列表
   - GET `https://api.vc.bilibili.com/svr_sync/v1/svr_sync/fetch_session_msgs`

2. 发送私信
   - POST `https://api.vc.bilibili.com/web_im/v1/web_im/send_msg`

## 数据存储

### 存储位置
- Windows: `C:\Users\<用户名>\.bilibili_account_manager\`
- macOS: `/Users/<用户名>/.bilibili_account_manager/`
- Linux: `/home/<用户名>/.bilibili_account_manager/`

### 文件说明

#### bilibili_accounts.enc
加密存储的账号信息，使用 AES-256-GCM 加密

#### auto_reply_settings.json
自动回复配置（明文存储）

#### key.bin
加密密钥文件

## 安全说明

1. 所有账号信息使用 AES-256-GCM 加密存储
2. 加密密钥单独存储在 `key.bin` 文件中
3. Cookie 信息仅在内存中使用，不写入日志
4. 网络请求使用 HTTPS 加密传输
