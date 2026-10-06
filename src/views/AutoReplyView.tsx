import { useCallback, useEffect, useRef, useState } from 'react'
import {
  History,
  KeyRound,
  Loader2,
  Lock,
  MessageSquare,
  Plus,
  RefreshCw,
  Save,
  Trash2,
  Video,
  Zap,
} from 'lucide-react'
import { toast } from 'sonner'
import Dialog from '@/components/Dialog'
import EmptyState from '@/components/EmptyState'
import SectionCard from '@/components/SectionCard'
import StatusBar, { type StatusTone } from '@/components/StatusBar'
import ViewShell from '@/components/ViewShell'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Separator } from '@/components/ui/separator'
import { Skeleton } from '@/components/ui/skeleton'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import {
  api,
  errorMessage,
  type AutoReplyChannels,
  type AutoReplySettings,
  type ChannelReplySettings,
  type CommentReplySettings,
  type MsgSource,
  type ReplyHistory,
  type ReplyPolicy,
  type TrackedVideoSettings,
} from '@/lib/ipc'
import { cn } from '@/lib/utils'
import { useAuth } from '@/state/auth'

type SaveState = 'idle' | 'saving' | 'saved' | 'error'

const DEFAULT_MESSAGE = '感谢您的留言！我会尽快回复。'
const DEFAULT_INTERVAL = 60
const DEFAULT_FAST_INTERVAL = 3
const MIN_INTERVAL = 1
const MAX_INTERVAL = 3600
const MIN_FAST_INTERVAL = 1
const MAX_FAST_INTERVAL = 60

const SAVE_STATE_LABELS: Record<SaveState, string> = {
  idle: '',
  saving: '保存中',
  saved: '已保存',
  error: '保存失败',
}

const CHANNEL_TABS: { key: MsgSource; label: string; description: string }[] = [
  { key: 'comment', label: '视频评论', description: '处理已发布视频下的新评论。' },
  { key: 'dynamic', label: '动态评论', description: '处理已发布动态下的新评论。' },
  { key: 'directMessage', label: '私信', description: '处理未读的一对一私信。' },
  { key: 'follow', label: '关注', description: '向新关注用户发送欢迎私信。' },
]

// 后端返回的配置可能来自旧版本 JSON，读取前统一做一次宽松归一化。
const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null

const readBoolean = (value: unknown, fallback: boolean) =>
  typeof value === 'boolean' ? value : fallback

const readString = (value: unknown, fallback: string) =>
  typeof value === 'string' ? value : fallback

const readPolicy = (value: unknown, fallback: ReplyPolicy): ReplyPolicy =>
  value === 'perMessage' || value === 'oncePerUser' ? value : fallback

const clampNumber = (value: unknown, min: number, max: number, fallback: number) => {
  const parsed = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.min(max, Math.max(min, Math.round(parsed)))
}

const isInRange = (text: string, min: number, max: number) => {
  const parsed = Number(text)
  return Number.isFinite(parsed) && Math.round(parsed) >= min && Math.round(parsed) <= max
}

const isMsgSource = (value: unknown): value is MsgSource =>
  value === 'comment' || value === 'dynamic' || value === 'directMessage' || value === 'follow'

const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T

const createTrackedVideo = (): TrackedVideoSettings => ({
  bvid: '',
  enabled: true,
  message: DEFAULT_MESSAGE,
  replyPolicy: 'perMessage',
  likeComments: true,
})

const normalizeChannel = (raw: unknown, fallbackPolicy: ReplyPolicy): ChannelReplySettings => {
  const source = isRecord(raw) ? raw : {}
  return {
    enabled: readBoolean(source.enabled, true),
    message: readString(source.message, DEFAULT_MESSAGE),
    replyPolicy: readPolicy(source.replyPolicy, fallbackPolicy),
  }
}

const normalizeCommentChannel = (
  raw: unknown,
  fallbackPolicy: ReplyPolicy,
): CommentReplySettings => {
  const source = isRecord(raw) ? raw : {}
  return {
    ...normalizeChannel(raw, fallbackPolicy),
    likeComments: readBoolean(source.likeComments, true),
  }
}

const normalizeTrackedVideo = (raw: unknown): TrackedVideoSettings => {
  const source = isRecord(raw) ? raw : {}
  return {
    bvid: readString(source.bvid, ''),
    enabled: readBoolean(source.enabled, true),
    message: readString(source.message, DEFAULT_MESSAGE),
    replyPolicy: readPolicy(source.replyPolicy, 'perMessage'),
    likeComments: readBoolean(source.likeComments, true),
  }
}

const normalizeHistory = (raw: unknown): ReplyHistory[] => {
  if (!Array.isArray(raw)) return []
  const items: ReplyHistory[] = []
  for (const entry of raw as unknown[]) {
    if (!isRecord(entry) || !isMsgSource(entry.source)) continue
    items.push({
      user: readString(entry.user, '未知用户'),
      time: readString(entry.time, ''),
      message: readString(entry.message, ''),
      source: entry.source,
    })
  }
  return items
}

const normalizeSettings = (raw: unknown): AutoReplySettings => {
  const source = isRecord(raw) ? raw : {}
  const channels = isRecord(source.channels) ? source.channels : {}
  return {
    enabled: readBoolean(source.enabled, true),
    interval: clampNumber(source.interval, MIN_INTERVAL, MAX_INTERVAL, DEFAULT_INTERVAL),
    fastInterval: clampNumber(
      source.fastInterval,
      MIN_FAST_INTERVAL,
      MAX_FAST_INTERVAL,
      DEFAULT_FAST_INTERVAL,
    ),
    channels: {
      comment: normalizeCommentChannel(channels.comment, 'perMessage'),
      dynamic: normalizeCommentChannel(channels.dynamic, 'perMessage'),
      directMessage: normalizeChannel(channels.directMessage, 'oncePerUser'),
      follow: normalizeChannel(channels.follow, 'oncePerUser'),
    },
    trackedVideos: Array.isArray(source.trackedVideos)
      ? (source.trackedVideos as unknown[]).map(normalizeTrackedVideo)
      : [],
    history: normalizeHistory(source.history),
  }
}

const isBvidLike = (bvid: string) => /^bv[0-9a-z]+$/i.test(bvid)

/** 开关行：左说明右开关，页面里所有布尔设置都长这样。 */
function SettingToggle({
  label,
  description,
  checked,
  onCheckedChange,
  className,
}: {
  label: string
  description?: string
  checked: boolean
  onCheckedChange: (checked: boolean) => void
  className?: string
}) {
  return (
    <div className={cn('flex items-center justify-between gap-4', className)}>
      <div className="min-w-0 space-y-1">
        <p className="text-sm font-medium">{label}</p>
        {description ? <p className="text-xs text-muted-foreground">{description}</p> : null}
      </div>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  )
}

/** 回复策略：渠道与指定视频共用同一组选项。 */
function ReplyPolicyField({
  value,
  onChange,
  idPrefix,
}: {
  value: ReplyPolicy
  onChange: (policy: ReplyPolicy) => void
  idPrefix: string
}) {
  return (
    <div className="space-y-2">
      <Label>回复策略</Label>
      <RadioGroup
        value={value}
        onValueChange={(next) => onChange(next as ReplyPolicy)}
        className="flex flex-wrap gap-x-5 gap-y-2"
      >
        <div className="flex items-center gap-2">
          <RadioGroupItem value="perMessage" id={`${idPrefix}-per-message`} />
          <Label htmlFor={`${idPrefix}-per-message`} className="font-normal">
            每条消息
          </Label>
        </div>
        <div className="flex items-center gap-2">
          <RadioGroupItem value="oncePerUser" id={`${idPrefix}-once-per-user`} />
          <Label htmlFor={`${idPrefix}-once-per-user`} className="font-normal">
            每个用户一次
          </Label>
        </div>
      </RadioGroup>
    </div>
  )
}

export default function AutoReplyView() {
  const { isPlus, activateLicense } = useAuth()

  const [settings, setSettings] = useState<AutoReplySettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState('')
  const [activeChannel, setActiveChannel] = useState<MsgSource>('comment')
  const [saveState, setSaveState] = useState<SaveState>('idle')
  const [previewRunning, setPreviewRunning] = useState(false)
  const [manualRunning, setManualRunning] = useState(false)
  const [actionResult, setActionResult] = useState('')
  const [actionError, setActionError] = useState(false)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [intervalText, setIntervalText] = useState(String(DEFAULT_INTERVAL))
  const [fastIntervalText, setFastIntervalText] = useState(String(DEFAULT_FAST_INTERVAL))

  // Plus 门禁只做提示与入口，不隐藏内容，避免与自动保存互相打架。
  const [notice, setNotice] = useState<{ tone: StatusTone; text: string } | null>(null)
  const [keyDialog, setKeyDialog] = useState(false)
  const [licenseKey, setLicenseKey] = useState('')
  const [keySubmitting, setKeySubmitting] = useState(false)
  const [keyError, setKeyError] = useState('')

  const saveQueueRef = useRef<Promise<void>>(Promise.resolve())
  const saveVersionRef = useRef(0)
  // 输入框失焦时提交的是最新一次编辑，不能依赖创建事件处理器那一刻的闭包快照。
  const settingsRef = useRef<AutoReplySettings | null>(null)
  settingsRef.current = settings

  const activeMeta = CHANNEL_TABS.find((tab) => tab.key === activeChannel) ?? CHANNEL_TABS[0]

  const applySettings = useCallback((next: AutoReplySettings) => {
    setSettings(next)
    setIntervalText(String(next.interval))
    setFastIntervalText(String(next.fastInterval))
  }, [])

  const load = useCallback(async () => {
    setLoading(true)
    setLoadError('')
    try {
      applySettings(normalizeSettings(await api.getAutoReplySettings()))
    } catch (error) {
      setSettings(null)
      setLoadError(errorMessage(error) || '无法读取本地设置')
    } finally {
      setLoading(false)
    }

    try {
      setAutostartEnabled(await api.getAutostartStatus())
    } catch (error) {
      console.error('加载开机自启状态失败:', error)
    }
  }, [applySettings])

  useEffect(() => {
    void load()
  }, [load])

  // 写入串行化：连续修改时只让最后一次的结果决定保存状态提示。
  const enqueueSave = useCallback((snapshot: AutoReplySettings): Promise<boolean> => {
    const version = saveVersionRef.current + 1
    saveVersionRef.current = version
    setSaveState('saving')

    const run = saveQueueRef.current.then(() => api.saveAutoReplySettings(snapshot))
    saveQueueRef.current = run.then(
      () => undefined,
      () => undefined,
    )

    return run.then(
      () => {
        if (saveVersionRef.current === version) setSaveState('saved')
        return true
      },
      (error: unknown) => {
        if (saveVersionRef.current === version) setSaveState('error')
        console.error('保存设置失败:', error)
        return false
      },
    )
  }, [])

  const update = (mutate: (draft: AutoReplySettings) => void) => {
    const current = settingsRef.current
    if (!current) return
    const draft = clone(current)
    mutate(draft)
    setSettings(draft)
    void enqueueSave(draft)
  }

  const saveCurrent = () => {
    const current = settingsRef.current
    if (current) void enqueueSave(current)
  }

  const ensureSaved = async () => {
    const current = settingsRef.current
    if (!current) throw new Error('设置尚未加载完成')
    if (!(await enqueueSave(current))) throw new Error('保存设置失败，请重试')
  }

  const saveNow = async () => {
    try {
      await ensureSaved()
      toast.success('设置已保存')
    } catch (error) {
      toast.error(errorMessage(error) || '保存设置失败')
    }
  }

  const updateChannel = (mutate: (channel: AutoReplyChannels[MsgSource]) => void) => {
    update((draft) => {
      mutate(draft.channels[activeChannel])
    })
  }

  const updateCommentChannel = (mutate: (channel: CommentReplySettings) => void) => {
    update((draft) => {
      if (activeChannel === 'comment') mutate(draft.channels.comment)
      else if (activeChannel === 'dynamic') mutate(draft.channels.dynamic)
    })
  }

  const setChannelMessage = (message: string) => {
    updateChannel((channel) => {
      channel.message = message
    })
  }

  const commitInterval = () => {
    const next = clampNumber(intervalText, MIN_INTERVAL, MAX_INTERVAL, DEFAULT_INTERVAL)
    setIntervalText(String(next))
    update((draft) => {
      draft.interval = next
    })
  }

  const commitFastInterval = () => {
    const next = clampNumber(
      fastIntervalText,
      MIN_FAST_INTERVAL,
      MAX_FAST_INTERVAL,
      DEFAULT_FAST_INTERVAL,
    )
    setFastIntervalText(String(next))
    update((draft) => {
      draft.fastInterval = next
    })
  }

  const addTrackedVideo = () => {
    update((draft) => {
      draft.trackedVideos.push(createTrackedVideo())
    })
  }

  const removeTrackedVideo = (index: number) => {
    update((draft) => {
      draft.trackedVideos.splice(index, 1)
    })
  }

  const updateTrackedVideo = (index: number, mutate: (video: TrackedVideoSettings) => void) => {
    update((draft) => {
      const video = draft.trackedVideos[index]
      if (video) mutate(video)
    })
  }

  const testReply = async () => {
    setPreviewRunning(true)
    setActionResult('')
    setActionError(false)
    try {
      await ensureSaved()
      setActionResult(await api.testAutoReply())
    } catch (error) {
      setActionResult(errorMessage(error) || '回复模板预览失败')
      setActionError(true)
    } finally {
      setPreviewRunning(false)
    }
  }

  const manualReply = async () => {
    setManualRunning(true)
    setActionResult('')
    setActionError(false)
    try {
      await ensureSaved()
      const result =
        activeChannel === 'dynamic'
          ? await api.manualReplyDynamicComments()
          : await api.manualReplyVideoComments()
      setActionResult(result)
      try {
        applySettings(normalizeSettings(await api.getAutoReplySettings()))
      } catch (error) {
        console.warn('刷新回复记录失败:', error)
      }
    } catch (error) {
      setActionResult(errorMessage(error) || '处理评论失败')
      setActionError(true)
    } finally {
      setManualRunning(false)
    }
  }

  const toggleAutostart = async () => {
    const next = !autostartEnabled
    setAutostartEnabled(next)
    setNotice(null)
    try {
      await api.setAutostart(next)
      setNotice({ tone: 'success', text: next ? '已开启开机自启' : '已关闭开机自启' })
    } catch (error) {
      setAutostartEnabled(!next)
      setNotice({ tone: 'error', text: '设置开机自启失败：' + errorMessage(error) })
    }
  }

  const submitLicense = async () => {
    const key = licenseKey.trim()
    if (!key || keySubmitting) return
    setKeySubmitting(true)
    setKeyError('')
    try {
      await activateLicense(key)
      setKeyDialog(false)
      setLicenseKey('')
      setNotice({ tone: 'success', text: 'Plus 已激活，自动回复功能已解锁' })
    } catch (error) {
      setKeyError(errorMessage(error) || '激活失败，请检查激活码')
    } finally {
      setKeySubmitting(false)
    }
  }

  const isChannelEnabled = (data: AutoReplySettings, key: MsgSource) =>
    data.channels[key].enabled ||
    (key === 'comment' &&
      data.trackedVideos.some((video) => video.enabled && isBvidLike(video.bvid.trim())))

  const renderChannelPanel = (data: AutoReplySettings) => {
    const channel = data.channels[activeChannel]
    const commentChannel =
      activeChannel === 'comment'
        ? data.channels.comment
        : activeChannel === 'dynamic'
          ? data.channels.dynamic
          : null

    return (
      <div className="space-y-5">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <h3 className="text-sm font-semibold">{activeMeta.label}</h3>
            <p className="text-xs text-muted-foreground">{activeMeta.description}</p>
          </div>
          <Switch
            checked={channel.enabled}
            onCheckedChange={(checked) =>
              updateChannel((target) => {
                target.enabled = checked
              })
            }
          />
        </div>

        {commentChannel ? (
          <SettingToggle
            label="自动点赞评论"
            description="该开关可独立于评论自动回复运行。"
            checked={commentChannel.likeComments}
            onCheckedChange={(checked) =>
              updateCommentChannel((target) => {
                target.likeComments = checked
              })
            }
          />
        ) : null}

        <ReplyPolicyField
          value={channel.replyPolicy}
          idPrefix={activeChannel}
          onChange={(policy) =>
            updateChannel((target) => {
              target.replyPolicy = policy
            })
          }
        />

        <div className="space-y-2">
          <Label htmlFor={`${activeChannel}-message`}>固定回复内容</Label>
          <Textarea
            id={`${activeChannel}-message`}
            rows={4}
            placeholder="输入自动回复内容"
            value={channel.message}
            onChange={(event) => setChannelMessage(event.target.value)}
            onBlur={saveCurrent}
          />
          <p className="text-xs text-muted-foreground">{'支持 {用户名}、{时间}'}</p>
        </div>

        {activeChannel === 'comment' ? (
          <div className="space-y-4 border-t pt-5">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="space-y-1">
                <h4 className="text-sm font-semibold">指定视频</h4>
                <p className="text-xs text-muted-foreground">
                  填写 BV 号后，额外处理该视频评论；每个视频可使用独立回复内容。
                </p>
              </div>
              <Button variant="outline" size="sm" onClick={addTrackedVideo}>
                <Plus />
                添加视频
              </Button>
            </div>

            {data.trackedVideos.length === 0 ? (
              <EmptyState
                icon={Video}
                title="尚未添加指定视频"
                description="添加后该视频的评论会走独立回复内容与点赞开关。"
              />
            ) : (
              <div className="space-y-4">
                {data.trackedVideos.map((video, index) => {
                  const invalidBvid = video.bvid.trim() !== '' && !isBvidLike(video.bvid.trim())

                  return (
                    <div key={index} className="space-y-4 rounded-lg border bg-muted/40 p-4">
                      <div className="flex items-center gap-3">
                        <span className="flex-1 text-sm font-medium">视频 {index + 1}</span>
                        <Switch
                          aria-label={`启用视频 ${index + 1}`}
                          checked={video.enabled}
                          onCheckedChange={(checked) =>
                            updateTrackedVideo(index, (target) => {
                              target.enabled = checked
                            })
                          }
                        />
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={`删除视频 ${index + 1}`}
                          onClick={() => removeTrackedVideo(index)}
                        >
                          <Trash2 />
                        </Button>
                      </div>

                      <div className="grid gap-4 sm:grid-cols-2">
                        <div className="space-y-2">
                          <Label htmlFor={`tracked-video-bvid-${index}`}>BV 号</Label>
                          <Input
                            id={`tracked-video-bvid-${index}`}
                            placeholder="例如 BV1xx411c7mD"
                            autoComplete="off"
                            aria-invalid={invalidBvid}
                            value={video.bvid}
                            onChange={(event) => {
                              const bvid = event.target.value
                              updateTrackedVideo(index, (target) => {
                                target.bvid = bvid
                              })
                            }}
                            onBlur={saveCurrent}
                          />
                          {invalidBvid ? (
                            <p className="text-xs text-destructive">BV 号格式不正确</p>
                          ) : (
                            <p className="text-xs text-muted-foreground">必填，支持大小写 BV 前缀</p>
                          )}
                        </div>

                        <ReplyPolicyField
                          value={video.replyPolicy}
                          idPrefix={`tracked-video-${index}`}
                          onChange={(policy) =>
                            updateTrackedVideo(index, (target) => {
                              target.replyPolicy = policy
                            })
                          }
                        />

                        <div className="space-y-2 sm:col-span-2">
                          <Label htmlFor={`tracked-video-message-${index}`}>独立回复内容</Label>
                          <Textarea
                            id={`tracked-video-message-${index}`}
                            rows={3}
                            placeholder="输入该视频的自动回复内容"
                            value={video.message}
                            onChange={(event) => {
                              const message = event.target.value
                              updateTrackedVideo(index, (target) => {
                                target.message = message
                              })
                            }}
                            onBlur={saveCurrent}
                          />
                          <p className="text-xs text-muted-foreground">
                            {'支持 {用户名}、{时间}'}
                          </p>
                        </div>

                        <SettingToggle
                          className="sm:col-span-2"
                          label="自动点赞该视频评论"
                          description="与回复开关和全局视频评论设置相互独立。"
                          checked={video.likeComments}
                          onCheckedChange={(checked) =>
                            updateTrackedVideo(index, (target) => {
                              target.likeComments = checked
                            })
                          }
                        />
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        ) : null}

        <div className="flex flex-wrap gap-2 border-t pt-5">
          <Button variant="outline" disabled={previewRunning} onClick={() => void testReply()}>
            {previewRunning ? <Loader2 className="animate-spin" /> : <MessageSquare />}
            测试回复
          </Button>
          {activeChannel === 'comment' || activeChannel === 'dynamic' ? (
            <Button variant="brand" disabled={manualRunning} onClick={() => void manualReply()}>
              {manualRunning ? <Loader2 className="animate-spin" /> : <Zap />}
              {activeChannel === 'dynamic' ? '立即处理动态评论' : '立即处理视频评论'}
            </Button>
          ) : null}
        </div>

        {actionResult ? (
          <StatusBar tone={actionError ? 'error' : 'success'} className="whitespace-pre-wrap">
            {actionResult}
          </StatusBar>
        ) : null}
      </div>
    )
  }

  const renderHistory = (data: AutoReplySettings) => {
    const items = data.history.filter((item) => item.source === activeChannel)

    return (
      <SectionCard
        title={`${activeMeta.label}回复记录`}
        description="最近保存的当前渠道回复。"
        actions={<Badge variant="secondary">{items.length}</Badge>}
      >
        {items.length === 0 ? (
          <EmptyState
            icon={History}
            title="暂无回复记录"
            description="开启自动回复后，这里会显示当前渠道最近处理的记录。"
          />
        ) : (
          <div className="divide-y">
            {items.map((item, index) => (
              <div key={`${item.time}-${index}`} className="space-y-1.5 py-3 first:pt-0 last:pb-0">
                <div className="flex items-baseline justify-between gap-4">
                  <span className="truncate text-sm font-medium">{item.user}</span>
                  <span className="shrink-0 text-xs text-muted-foreground">{item.time}</span>
                </div>
                <p className="text-sm break-words text-muted-foreground">{item.message}</p>
              </div>
            ))}
          </div>
        )}
      </SectionCard>
    )
  }

  return (
    <ViewShell
      title="自动回复"
      subtitle="配置各渠道的自动回复策略、点赞与指定视频"
      actions={
        <>
          {saveState === 'idle' ? null : (
            <span
              className={cn(
                'text-xs',
                saveState === 'saved' && 'text-success',
                saveState === 'error' && 'text-destructive',
                saveState === 'saving' && 'text-muted-foreground',
              )}
            >
              {SAVE_STATE_LABELS[saveState]}
            </span>
          )}
          <Button
            variant="default"
            size="sm"
            disabled={!settings || loading}
            onClick={() => void saveNow()}
          >
            <Save />
            保存设置
          </Button>
        </>
      }
    >
      {notice ? <StatusBar tone={notice.tone}>{notice.text}</StatusBar> : null}

      {isPlus ? null : (
        <div className="flex flex-wrap items-center gap-4 rounded-xl border border-warning/30 bg-warning-tint px-4 py-3">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-background text-warning">
            <Lock className="size-4" />
          </span>
          <div className="min-w-0 flex-1 space-y-0.5">
            <p className="text-sm font-medium">自动回复需要先激活 Plus</p>
            <p className="text-xs text-muted-foreground">
              激活后可使用自动回复和自动点赞；当前设置仍可查看和编辑。
            </p>
          </div>
          <Button variant="brand" size="sm" onClick={() => setKeyDialog(true)}>
            <KeyRound />
            激活 Plus
          </Button>
        </div>
      )}

      {loading ? (
        <SectionCard>
          <div className="space-y-4">
            <Skeleton className="h-5 w-40" />
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-24 w-full" />
          </div>
        </SectionCard>
      ) : loadError ? (
        <SectionCard title="设置加载失败">
          <div className="space-y-4">
            <StatusBar tone="error">{loadError}</StatusBar>
            <Button variant="outline" onClick={() => void load()}>
              <RefreshCw />
              重试
            </Button>
          </div>
        </SectionCard>
      ) : settings ? (
        <>
          <SectionCard title="运行设置" description="统一控制自动回复服务的运行状态和检查频率。">
            <div className="space-y-5">
              <SettingToggle
                label="自动回复总开关"
                description="关闭后暂停自动回复；视频、动态及指定视频点赞仍按各自设置执行。"
                checked={settings.enabled}
                onCheckedChange={(checked) =>
                  update((draft) => {
                    draft.enabled = checked
                  })
                }
              />

              <Separator />

              <SettingToggle
                label="开机自启"
                description="系统启动后在后台运行。"
                checked={autostartEnabled}
                onCheckedChange={() => void toggleAutostart()}
              />

              <Separator />

              <div className="grid gap-5 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="poll-interval">检查间隔（秒）</Label>
                  <Input
                    id="poll-interval"
                    type="number"
                    min={MIN_INTERVAL}
                    max={MAX_INTERVAL}
                    inputMode="numeric"
                    value={intervalText}
                    onChange={(event) => setIntervalText(event.target.value)}
                    onBlur={commitInterval}
                  />
                  {isInRange(intervalText, MIN_INTERVAL, MAX_INTERVAL) ? (
                    <p className="text-xs text-muted-foreground">
                      完整补扫视频评论、动态评论、私信和关注的周期。
                    </p>
                  ) : (
                    <p className="text-xs text-destructive">
                      请输入 {MIN_INTERVAL}–{MAX_INTERVAL} 之间的秒数。
                    </p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="fast-interval">快速通道间隔（秒）</Label>
                  <Input
                    id="fast-interval"
                    type="number"
                    min={MIN_FAST_INTERVAL}
                    max={MAX_FAST_INTERVAL}
                    inputMode="numeric"
                    value={fastIntervalText}
                    onChange={(event) => setFastIntervalText(event.target.value)}
                    onBlur={commitFastInterval}
                  />
                  {isInRange(fastIntervalText, MIN_FAST_INTERVAL, MAX_FAST_INTERVAL) ? (
                    <p className="text-xs text-muted-foreground">
                      只抓取每个评论目标的最新一页，用于新评论秒回；私信和关注仍按检查间隔处理。
                    </p>
                  ) : (
                    <p className="text-xs text-destructive">
                      请输入 {MIN_FAST_INTERVAL}–{MAX_FAST_INTERVAL} 之间的秒数。
                    </p>
                  )}
                </div>
              </div>
            </div>
          </SectionCard>

          <SectionCard title="分渠道配置" description="回复内容和回复策略互不影响。">
            <Tabs
              value={activeChannel}
              onValueChange={(value) => setActiveChannel(value as MsgSource)}
            >
              <TabsList className="w-full">
                {CHANNEL_TABS.map((tab) => (
                  <TabsTrigger
                    key={tab.key}
                    value={tab.key}
                    className="flex-col gap-0.5 py-1.5 leading-tight"
                  >
                    <span>{tab.label}</span>
                    <span
                      className={cn(
                        'text-[11px] font-normal',
                        isChannelEnabled(settings, tab.key)
                          ? 'text-success'
                          : 'text-muted-foreground',
                      )}
                    >
                      {isChannelEnabled(settings, tab.key) ? '已开启' : '已关闭'}
                    </span>
                  </TabsTrigger>
                ))}
              </TabsList>

              <TabsContent value={activeChannel} className="pt-5">
                {renderChannelPanel(settings)}
              </TabsContent>
            </Tabs>
          </SectionCard>

          {renderHistory(settings)}
        </>
      ) : null}

      {keyDialog ? (
        <Dialog
          title="输入激活密钥"
          description="在爱发电购买 Plus 方案后会收到激活码，粘贴到下方即可解锁自动回复与自动点赞。"
          onClose={() => setKeyDialog(false)}
          footer={
            <>
              <Button variant="outline" onClick={() => setKeyDialog(false)}>
                取消
              </Button>
              <Button
                variant="brand"
                onClick={() => void submitLicense()}
                disabled={!licenseKey.trim() || keySubmitting}
              >
                {keySubmitting ? '验证中...' : '激活'}
              </Button>
            </>
          }
        >
          <div className="space-y-2">
            <Label htmlFor="license-key">激活密钥</Label>
            <Input
              id="license-key"
              value={licenseKey}
              placeholder="请输入激活密钥"
              autoFocus
              onChange={(event) => setLicenseKey(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') void submitLicense()
              }}
            />
          </div>
          {keyError ? <StatusBar tone="error">{keyError}</StatusBar> : null}
        </Dialog>
      ) : null}
    </ViewShell>
  )
}
