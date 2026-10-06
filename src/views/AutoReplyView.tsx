import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import Dialog from '../components/Dialog'
import StatusBar, { type StatusTone } from '../components/StatusBar'
import { IconKey, IconLock } from '../components/icons'
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
} from '../lib/ipc'
import { useAuth } from '../state/auth'
import './autoreply.css'

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

const normalizeCommentChannel = (raw: unknown, fallbackPolicy: ReplyPolicy): CommentReplySettings => {
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

export default function AutoReplyView() {
  const navigate = useNavigate()
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
    setActionResult('')
    setActionError(false)
    try {
      await ensureSaved()
      setActionResult('设置已保存')
    } catch (error) {
      setActionResult(errorMessage(error) || '保存设置失败')
      setActionError(true)
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

  const renderOverview = (data: AutoReplySettings) => (
    <div className="surface-section overview-section">
      <div className="section-heading">
        <div>
          <h2>运行设置</h2>
          <p>统一控制自动回复服务的运行状态和检查频率。</p>
        </div>
      </div>

      <div className="setting-list">
        <div className="setting-row">
          <div className="setting-copy">
            <strong>自动回复总开关</strong>
            <span>关闭后暂停自动回复；视频、动态及指定视频点赞仍按各自设置执行。</span>
          </div>
          <label className="toggle">
            <input
              type="checkbox"
              checked={data.enabled}
              onChange={(event) => {
                const checked = event.target.checked
                update((draft) => {
                  draft.enabled = checked
                })
              }}
            />
            <span className="toggle-track" />
          </label>
        </div>

        <div className="setting-row">
          <div className="setting-copy">
            <strong>开机自启</strong>
            <span>系统启动后在后台运行。</span>
          </div>
          <label className="toggle">
            <input
              type="checkbox"
              checked={autostartEnabled}
              onChange={() => void toggleAutostart()}
            />
            <span className="toggle-track" />
          </label>
        </div>
      </div>

      <div className="compact-field">
        <div className="setting-copy">
          <label htmlFor="poll-interval">检查间隔</label>
          <span>完整补扫视频评论、动态评论、私信和关注的周期。</span>
        </div>
        <div className="number-control">
          <input
            id="poll-interval"
            type="number"
            min={MIN_INTERVAL}
            max={MAX_INTERVAL}
            inputMode="numeric"
            value={intervalText}
            onChange={(event) => setIntervalText(event.target.value)}
            onBlur={commitInterval}
          />
          <span>秒</span>
        </div>
      </div>

      <div className="compact-field">
        <div className="setting-copy">
          <label htmlFor="fast-interval">快速通道间隔</label>
          <span>只抓取每个评论目标的最新一页，用于新评论秒回；私信和关注仍按检查间隔处理。</span>
        </div>
        <div className="number-control">
          <input
            id="fast-interval"
            type="number"
            min={MIN_FAST_INTERVAL}
            max={MAX_FAST_INTERVAL}
            inputMode="numeric"
            value={fastIntervalText}
            onChange={(event) => setFastIntervalText(event.target.value)}
            onBlur={commitFastInterval}
          />
          <span>秒</span>
        </div>
      </div>
    </div>
  )

  const renderChannelPanel = (data: AutoReplySettings) => {
    const channel = data.channels[activeChannel]
    const commentChannel =
      activeChannel === 'comment'
        ? data.channels.comment
        : activeChannel === 'dynamic'
          ? data.channels.dynamic
          : null
    const isCommentChannel = commentChannel !== null

    return (
      <div className="surface-section channel-section">
        <div className="section-heading channel-heading">
          <div>
            <h2>分渠道配置</h2>
            <p>回复内容和回复策略互不影响。</p>
          </div>
        </div>

        <div className="channel-tabs" role="tablist" aria-label="自动回复渠道">
          {CHANNEL_TABS.map((tab) => (
            <button
              key={tab.key}
              id={`channel-tab-${tab.key}`}
              className={`channel-tab${activeChannel === tab.key ? ' active' : ''}`}
              type="button"
              role="tab"
              aria-selected={activeChannel === tab.key}
              aria-controls={`channel-panel-${tab.key}`}
              onClick={() => setActiveChannel(tab.key)}
            >
              <span>{tab.label}</span>
              <small className={isChannelEnabled(data, tab.key) ? 'enabled' : ''}>
                {isChannelEnabled(data, tab.key) ? '已开启' : '已关闭'}
              </small>
            </button>
          ))}
        </div>

        <div
          id={`channel-panel-${activeChannel}`}
          className="channel-panel"
          role="tabpanel"
          aria-labelledby={`channel-tab-${activeChannel}`}
        >
          <div className="channel-title-row">
            <div>
              <h3>{activeMeta.label}</h3>
              <p>{activeMeta.description}</p>
            </div>
            <label className="toggle">
              <input
                type="checkbox"
                checked={channel.enabled}
                onChange={(event) => {
                  const checked = event.target.checked
                  updateChannel((target) => {
                    target.enabled = checked
                  })
                }}
              />
              <span className="toggle-track" />
            </label>
          </div>

          {commentChannel ? (
            <div className="setting-row channel-option-row">
              <div className="setting-copy">
                <strong>自动点赞评论</strong>
                <span>该开关可独立于评论自动回复运行。</span>
              </div>
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={commentChannel.likeComments}
                  onChange={(event) => {
                    const checked = event.target.checked
                    updateCommentChannel((target) => {
                      target.likeComments = checked
                    })
                  }}
                />
                <span className="toggle-track" />
              </label>
            </div>
          ) : null}

          <div className="channel-form-grid">
            <div className="form-field full-width">
              <label>回复策略</label>
              <div
                className="segmented-control"
                role="group"
                aria-label={`${activeMeta.label}回复策略`}
              >
                <button
                  type="button"
                  className={channel.replyPolicy === 'perMessage' ? 'active' : ''}
                  onClick={() =>
                    updateChannel((target) => {
                      target.replyPolicy = 'perMessage'
                    })
                  }
                >
                  每条消息
                </button>
                <button
                  type="button"
                  className={channel.replyPolicy === 'oncePerUser' ? 'active' : ''}
                  onClick={() =>
                    updateChannel((target) => {
                      target.replyPolicy = 'oncePerUser'
                    })
                  }
                >
                  每个用户一次
                </button>
              </div>
            </div>

            <div className="form-field full-width">
              <label htmlFor={`${activeChannel}-message`}>固定回复内容</label>
              <textarea
                id={`${activeChannel}-message`}
                rows={4}
                placeholder="输入自动回复内容"
                value={channel.message}
                onChange={(event) => setChannelMessage(event.target.value)}
                onBlur={saveCurrent}
              />
              <span className="field-hint">{'支持 {用户名}、{时间}'}</span>
            </div>
          </div>

          {activeChannel === 'comment' ? (
            <section className="tracked-videos-section">
              <div className="tracked-videos-heading">
                <div>
                  <h4>指定视频</h4>
                  <p>填写 BV 号后，额外处理该视频评论；每个视频可使用独立回复内容。</p>
                </div>
                <button className="btn btn-ghost btn-sm" type="button" onClick={addTrackedVideo}>
                  <svg
                    width={17}
                    height={17}
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={2}
                  >
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  添加视频
                </button>
              </div>

              {data.trackedVideos.length === 0 ? (
                <div className="tracked-videos-empty">尚未添加指定视频</div>
              ) : (
                <div className="tracked-videos-list">
                  {data.trackedVideos.map((video, index) => (
                    <div key={index} className="tracked-video-item">
                      <div className="tracked-video-header">
                        <div className="tracked-video-index">视频 {index + 1}</div>
                        <label className="toggle" aria-label={`启用视频 ${index + 1}`}>
                          <input
                            type="checkbox"
                            checked={video.enabled}
                            onChange={(event) => {
                              const checked = event.target.checked
                              updateTrackedVideo(index, (target) => {
                                target.enabled = checked
                              })
                            }}
                          />
                          <span className="toggle-track" />
                        </label>
                        <button
                          className="icon-button tracked-video-remove"
                          type="button"
                          aria-label={`删除视频 ${index + 1}`}
                          title="删除指定视频"
                          onClick={() => removeTrackedVideo(index)}
                        >
                          <svg
                            width={18}
                            height={18}
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={2}
                          >
                            <path d="M6 6l12 12M18 6 6 18" />
                          </svg>
                        </button>
                      </div>

                      <div className="tracked-video-grid">
                        <div className="form-field">
                          <label htmlFor={`tracked-video-bvid-${index}`}>BV 号</label>
                          <input
                            id={`tracked-video-bvid-${index}`}
                            type="text"
                            placeholder="例如 BV1xx411c7mD"
                            autoComplete="off"
                            value={video.bvid}
                            onChange={(event) => {
                              const bvid = event.target.value
                              updateTrackedVideo(index, (target) => {
                                target.bvid = bvid
                              })
                            }}
                            onBlur={saveCurrent}
                          />
                          {video.bvid.trim() !== '' && !isBvidLike(video.bvid.trim()) ? (
                            <span className="field-hint error">BV 号格式不正确</span>
                          ) : (
                            <span className="field-hint">必填，支持大小写 BV 前缀</span>
                          )}
                        </div>

                        <div className="form-field">
                          <label>回复策略</label>
                          <div
                            className="segmented-control"
                            role="group"
                            aria-label={`指定视频 ${index + 1} 回复策略`}
                          >
                            <button
                              type="button"
                              className={video.replyPolicy === 'perMessage' ? 'active' : ''}
                              onClick={() =>
                                updateTrackedVideo(index, (target) => {
                                  target.replyPolicy = 'perMessage'
                                })
                              }
                            >
                              每条消息
                            </button>
                            <button
                              type="button"
                              className={video.replyPolicy === 'oncePerUser' ? 'active' : ''}
                              onClick={() =>
                                updateTrackedVideo(index, (target) => {
                                  target.replyPolicy = 'oncePerUser'
                                })
                              }
                            >
                              每个用户一次
                            </button>
                          </div>
                        </div>

                        <div className="form-field full-width">
                          <label htmlFor={`tracked-video-message-${index}`}>独立回复内容</label>
                          <textarea
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
                          <span className="field-hint">{'支持 {用户名}、{时间}'}</span>
                        </div>

                        <div className="setting-row tracked-video-like-row">
                          <div className="setting-copy">
                            <strong>自动点赞该视频评论</strong>
                            <span>与回复开关和全局视频评论设置相互独立。</span>
                          </div>
                          <label className="toggle">
                            <input
                              type="checkbox"
                              checked={video.likeComments}
                              onChange={(event) => {
                                const checked = event.target.checked
                                updateTrackedVideo(index, (target) => {
                                  target.likeComments = checked
                                })
                              }}
                            />
                            <span className="toggle-track" />
                          </label>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          ) : null}

          <div className="channel-actions">
            <button className="btn btn-ghost" type="button" onClick={() => void saveNow()}>
              <svg
                width={17}
                height={17}
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z" />
                <path d="M17 21v-8H7v8M7 3v5h8" />
              </svg>
              保存设置
            </button>

            <button
              className="btn btn-ghost"
              type="button"
              disabled={previewRunning}
              onClick={() => void testReply()}
            >
              {previewRunning ? (
                <span className="spinner" aria-hidden="true" />
              ) : (
                <svg
                  width={17}
                  height={17}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={2}
                >
                  <path d="M21 15a4 4 0 0 1-4 4H8l-5 3V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8Z" />
                </svg>
              )}
              测试回复
            </button>

            {isCommentChannel ? (
              <button
                className="btn btn-accent"
                type="button"
                disabled={manualRunning}
                onClick={() => void manualReply()}
              >
                {manualRunning ? (
                  <span className="spinner" aria-hidden="true" />
                ) : (
                  <svg
                    width={17}
                    height={17}
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={2}
                  >
                    <path d="m13 2-2 8h7l-7 12 2-8H6l7-12Z" />
                  </svg>
                )}
                {activeChannel === 'dynamic' ? '立即处理动态评论' : '立即处理视频评论'}
              </button>
            ) : null}
          </div>

          {actionResult ? (
            <div className={`action-result${actionError ? ' error' : ''}`} role="status">
              {actionResult}
            </div>
          ) : null}
        </div>
      </div>
    )
  }

  const renderHistory = (data: AutoReplySettings) => {
    const items = data.history.filter((item) => item.source === activeChannel)
    return (
      <section className="history-surface">
        <div className="section-heading history-heading">
          <div>
            <h2>{activeMeta.label}回复记录</h2>
            <p>最近保存的当前渠道回复。</p>
          </div>
          <span className="count-badge">{items.length}</span>
        </div>

        {items.length === 0 ? (
          <div className="empty-history">
            <svg
              width={42}
              height={42}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.5}
            >
              <path d="M21 15a4 4 0 0 1-4 4H8l-5 3V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8Z" />
            </svg>
            <span>暂无回复记录</span>
          </div>
        ) : (
          <div className="history-list">
            {items.map((item, index) => (
              <article key={`${item.time}-${index}`} className="history-item">
                <div className="history-meta">
                  <strong>{item.user}</strong>
                  <time>{item.time}</time>
                </div>
                <p>{item.message}</p>
              </article>
            ))}
          </div>
        )}
      </section>
    )
  }

  return (
    <div className="auto-reply-page">
      <header className="page-header">
        <div className="header-content">
          <button
            className="icon-button"
            type="button"
            aria-label="返回首页"
            title="返回首页"
            onClick={() => void navigate('/')}
          >
            <svg
              width={20}
              height={20}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
          </button>
          <h1>自动回复</h1>
          <span className={`save-state ${saveState}`}>{SAVE_STATE_LABELS[saveState]}</span>
        </div>
      </header>

      <main className="page-main">
        {notice ? <StatusBar tone={notice.tone}>{notice.text}</StatusBar> : null}

        {isPlus ? null : (
          <section className="plus-lock" role="status">
            <span className="plus-lock-icon">
              <IconLock size={18} />
            </span>
            <div className="plus-lock-text">
              <strong>自动回复需要先激活 Plus</strong>
              <span>激活后可使用自动回复和自动点赞；当前设置仍可查看和编辑。</span>
            </div>
            <button className="btn btn-accent" type="button" onClick={() => setKeyDialog(true)}>
              <IconKey size={16} />
              激活 Plus
            </button>
          </section>
        )}

        {loading ? (
          <section className="loading-panel" aria-live="polite">
            <span className="spinner spinner-lg" aria-hidden="true" />
            <span>正在加载自动回复设置</span>
          </section>
        ) : loadError ? (
          <section className="error-panel" role="alert">
            <strong>设置加载失败</strong>
            <span>{loadError}</span>
            <button className="btn btn-ghost" type="button" onClick={() => void load()}>
              重试
            </button>
          </section>
        ) : settings ? (
          <>
            <section className="settings-surface">
              {renderOverview(settings)}
              {renderChannelPanel(settings)}
            </section>
            {renderHistory(settings)}
          </>
        ) : null}
      </main>

      {keyDialog ? (
        <Dialog
          title="输入激活密钥"
          onClose={() => setKeyDialog(false)}
          footer={
            <>
              <button className="btn btn-ghost" onClick={() => setKeyDialog(false)}>
                取消
              </button>
              <button
                className="btn btn-accent"
                onClick={() => void submitLicense()}
                disabled={!licenseKey.trim() || keySubmitting}
              >
                {keySubmitting ? '验证中...' : '激活'}
              </button>
            </>
          }
        >
          <p className="dialog-desc">
            在爱发电购买 Plus 方案后会收到激活码，粘贴到下方即可解锁自动回复与自动点赞。
          </p>
          <input
            className="input"
            value={licenseKey}
            placeholder="请输入激活密钥"
            autoFocus
            onChange={(event) => setLicenseKey(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') void submitLicense()
            }}
          />
          {keyError ? <StatusBar tone="error">{keyError}</StatusBar> : null}
        </Dialog>
      ) : null}
    </div>
  )
}
