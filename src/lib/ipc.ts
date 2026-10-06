import { invoke } from '@tauri-apps/api/core'

export interface QrCodeResponse {
  qrcode: string
  qrcodeKey: string
}

export interface UserInfo {
  uid: string
  name: string
  avatar: string
  cookie: string
}

export interface LoginStatus {
  status: string
  userInfo: UserInfo | null
}

export interface Account {
  uid: string
  name: string
  avatar: string
  cookie: string
  active: boolean
  createdAt: string
}

export type ReplyPolicy = 'perMessage' | 'oncePerUser'

export interface ChannelReplySettings {
  enabled: boolean
  message: string
  replyPolicy: ReplyPolicy
}

export interface CommentReplySettings extends ChannelReplySettings {
  likeComments: boolean
}

export interface TrackedVideoSettings extends CommentReplySettings {
  bvid: string
}

export interface AutoReplyChannels {
  comment: CommentReplySettings
  dynamic: CommentReplySettings
  directMessage: ChannelReplySettings
  follow: ChannelReplySettings
}

export type MsgSource = 'comment' | 'dynamic' | 'directMessage' | 'follow'

export interface ReplyHistory {
  user: string
  time: string
  message: string
  source: MsgSource
}

export interface AutoReplySettings {
  enabled: boolean
  interval: number
  fastInterval: number
  channels: AutoReplyChannels
  trackedVideos: TrackedVideoSettings[]
  history: ReplyHistory[]
}

export interface AuthSession {
  accessToken: string
  refreshToken: string
  userId: string
  email: string
  tier: string
}

export interface CloudDownload {
  accounts: Account[]
  settings: AutoReplySettings | null
  downloadedCount: number
}

/** 把任意 IPC / 运行时错误转换成可直接展示的中文文案。 */
export function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message)
  }
  return String(error)
}

export const api = {
  // 哔哩哔哩扫码登录
  getQrCode: () => invoke<QrCodeResponse>('get_qr_code'),
  generateQrCode: (data: string) => invoke<string>('generate_qr_code', { data }),
  checkLoginStatus: () => invoke<LoginStatus>('check_login_status'),

  // 本地账号
  getAccounts: () => invoke<Account[]>('get_accounts'),
  syncAccounts: (accounts: Account[]) => invoke<Account[]>('sync_accounts', { accounts }),
  activateAccount: (uid: string) => invoke<void>('activate_account', { uid }),
  deleteAccount: (uid: string) => invoke<void>('delete_account', { uid }),

  // 自动回复
  getAutoReplySettings: () => invoke<AutoReplySettings>('get_auto_reply_settings'),
  saveAutoReplySettings: (settings: AutoReplySettings) =>
    invoke<void>('save_auto_reply_settings', { settings }),
  getRepliedSet: () => invoke<string[]>('get_replied_set'),
  getLikedSet: () => invoke<string[]>('get_liked_set'),
  mergeRepliedSet: (entries: string[]) => invoke<void>('merge_replied_set', { entries }),
  mergeLikedSet: (entries: string[]) => invoke<void>('merge_liked_set', { entries }),
  testAutoReply: () => invoke<string>('test_auto_reply'),
  manualReplyVideoComments: () => invoke<string>('manual_reply_video_comments'),
  manualReplyDynamicComments: () => invoke<string>('manual_reply_dynamic_comments'),

  // Supabase 应用账号
  restoreSession: () => invoke<AuthSession | null>('auth_restore_session'),
  signIn: (email: string, password: string) =>
    invoke<AuthSession>('auth_sign_in', { email, password }),
  sendOtp: (email: string) => invoke<void>('auth_send_otp', { email }),
  verifyOtp: (email: string, token: string) =>
    invoke<AuthSession>('auth_verify_otp', { email, token }),
  logout: () => invoke<void>('auth_logout'),

  // 激活与系统集成
  isLicensed: () => invoke<boolean>('is_licensed'),
  activateLicense: (key: string) => invoke<void>('activate_license', { key }),
  getAutostartStatus: () => invoke<boolean>('get_autostart_status'),
  setAutostart: (enabled: boolean) => invoke<void>('set_autostart', { enabled }),

  // 云端同步
  cloudUploadAll: () => invoke<string>('cloud_upload_all'),
  cloudDownloadAll: () => invoke<CloudDownload>('cloud_download_all'),

  // 系统能力
  openExternalUrl: (url: string) => invoke<void>('open_external_url', { url }),
  copyTextToClipboard: (text: string) => invoke<void>('copy_text_to_clipboard', { text }),
}
