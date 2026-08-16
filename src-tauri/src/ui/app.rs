use super::auth::{self, AuthSession};
use super::cloud;
use super::platform;
use crate::auto_reply;
use crate::auto_reply::models::{
    AutoReplySettings, ChannelReplySettings, MsgSource, ReplyPolicy, TrackedVideoSettings,
};
use crate::bilibili;
use crate::runtime;
use crate::storage::{self, Account};
use base64::Engine as _;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, img, px, AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, Image,
    ImageFormat, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState, NumberInput, Textarea, TextareaState},
    scroll::ScrollableElement,
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt,
};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Bootstrap {
    pub accounts: Vec<Account>,
    pub settings: AutoReplySettings,
    pub licensed: bool,
    pub autostart: bool,
    pub auth: Option<AuthSession>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Auth,
    Dashboard,
    Login,
    Accounts,
    AutoReply,
    Sponsor,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Auth => "账号登录",
            Self::Dashboard => "概览",
            Self::Login => "扫码登录",
            Self::Accounts => "账号管理",
            Self::AutoReply => "自动回复",
            Self::Sponsor => "支持项目",
        }
    }
}

struct TrackedVideoEditor {
    bvid: Entity<InputState>,
    message: Entity<TextareaState>,
}

impl TrackedVideoEditor {
    fn new(video: &TrackedVideoSettings, window: &mut Window, cx: &mut Context<AppView>) -> Self {
        let bvid = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例如 BV1xx411c7mD")
                .default_value(video.bvid.clone())
        });
        let message = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .placeholder("输入该视频的独立回复内容")
                .default_value(video.reply.message.clone())
        });
        Self { bvid, message }
    }
}

pub struct AppView {
    focus_handle: FocusHandle,
    page: Page,
    accounts: Vec<Account>,
    settings: AutoReplySettings,
    licensed: bool,
    autostart: bool,
    auth: Option<AuthSession>,
    otp_sent: bool,
    active_channel: usize,
    busy: bool,
    backend_generation: u64,
    notice: Option<(String, bool)>,
    qr_image: Option<Arc<Image>>,
    qr_status: String,
    license_input: Entity<InputState>,
    email_input: Entity<InputState>,
    password_input: Entity<InputState>,
    otp_input: Entity<InputState>,
    interval_input: Entity<InputState>,
    channel_messages: [Entity<TextareaState>; 4],
    tracked_editors: Vec<TrackedVideoEditor>,
}

impl AppView {
    pub fn new(bootstrap: Bootstrap, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let license_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入 Plus 激活码"));
        let email_input = cx.new(|cx| InputState::new(window, cx).placeholder("邮箱地址"));
        let password_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("密码").masked(true));
        let otp_input = cx.new(|cx| InputState::new(window, cx).placeholder("邮件验证码"));
        let interval_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("检查间隔")
                .default_value(bootstrap.settings.interval.to_string())
                .min(1.)
                .max(3600.)
        });
        let channel_messages = std::array::from_fn(|index| {
            let source = Self::source_for_index(index);
            let value = bootstrap.settings.channel(source).message.clone();
            cx.new(|cx| {
                TextareaState::new(window, cx)
                    .rows(4)
                    .placeholder("输入自动回复内容")
                    .default_value(value)
            })
        });
        let tracked_editors = bootstrap
            .settings
            .tracked_videos
            .iter()
            .map(|video| TrackedVideoEditor::new(video, window, cx))
            .collect();

        let view = Self {
            focus_handle: cx.focus_handle(),
            page: if bootstrap.auth.is_some() {
                Page::Dashboard
            } else {
                Page::Auth
            },
            accounts: bootstrap.accounts,
            settings: bootstrap.settings,
            licensed: bootstrap.licensed,
            autostart: bootstrap.autostart,
            auth: bootstrap.auth,
            otp_sent: false,
            active_channel: 0,
            busy: false,
            backend_generation: 0,
            notice: None,
            qr_image: None,
            qr_status: "点击生成二维码后使用哔哩哔哩客户端扫码".into(),
            license_input,
            email_input,
            password_input,
            otp_input,
            interval_input,
            channel_messages,
            tracked_editors,
        };
        view.start_history_refresh(window, cx);
        view
    }

    fn source_for_index(index: usize) -> MsgSource {
        match index {
            0 => MsgSource::Comment,
            1 => MsgSource::Dynamic,
            2 => MsgSource::DirectMessage,
            _ => MsgSource::Follow,
        }
    }

    fn current_source(&self) -> MsgSource {
        Self::source_for_index(self.active_channel)
    }

    fn current_channel_mut(&mut self) -> &mut ChannelReplySettings {
        match self.active_channel {
            0 => &mut self.settings.channels.comment.reply,
            1 => &mut self.settings.channels.dynamic.reply,
            2 => &mut self.settings.channels.direct_message,
            _ => &mut self.settings.channels.follow,
        }
    }

    fn current_like_comments(&self) -> Option<bool> {
        match self.active_channel {
            0 => Some(self.settings.channels.comment.like_comments),
            1 => Some(self.settings.channels.dynamic.like_comments),
            _ => None,
        }
    }

    fn set_current_like_comments(&mut self, enabled: bool) {
        match self.active_channel {
            0 => self.settings.channels.comment.like_comments = enabled,
            1 => self.settings.channels.dynamic.like_comments = enabled,
            _ => {}
        }
    }

    fn navigate(&mut self, page: Page, _: &mut Window, cx: &mut Context<Self>) {
        if page == Page::AutoReply && !self.licensed {
            self.notice = Some(("自动回复需要先激活 Plus".into(), true));
            self.page = Page::Dashboard;
        } else {
            self.page = page;
            self.notice = None;
        }
        cx.notify();
    }

    fn activate_license(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.license_input.read(cx).value().to_string();
        match platform::activate_license(&key) {
            Ok(()) => {
                self.licensed = true;
                self.notice = Some(("Plus 已激活，自动回复功能已解锁".into(), false));
            }
            Err(error) => self.notice = Some((error, true)),
        }
        cx.notify();
    }

    fn toggle_autostart(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        match platform::set_autostart(enabled) {
            Ok(()) => {
                self.autostart = enabled;
                self.notice = Some((
                    if enabled {
                        "已开启开机自启".into()
                    } else {
                        "已关闭开机自启".into()
                    },
                    false,
                ));
            }
            Err(error) => self.notice = Some((format!("设置开机自启失败：{error}"), true)),
        }
        cx.notify();
    }

    fn complete_auth(&mut self, session: AuthSession) {
        match auth::save_session(&session) {
            Ok(()) => {
                self.licensed = platform::is_licensed() || session.tier == "plus";
                self.auth = Some(session);
                self.otp_sent = false;
                self.page = Page::Dashboard;
                self.notice = Some(("登录成功".into(), false));
            }
            Err(error) => {
                self.notice = Some((format!("保存登录会话失败：{error}"), true));
            }
        }
    }

    fn sign_in_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = self.email_input.read(cx).value().to_string();
        let password = self.password_input.read(cx).value().to_string();
        self.spawn_backend(
            auth::sign_in_password(email, password),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(session) => this.complete_auth(session),
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn send_otp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = self.email_input.read(cx).value().to_string();
        self.spawn_backend(
            auth::send_otp(email),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(()) => {
                    this.otp_sent = true;
                    this.notice = Some(("验证码已发送，请检查邮箱".into(), false));
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn verify_otp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = self.email_input.read(cx).value().to_string();
        let token = self.otp_input.read(cx).value().to_string();
        self.spawn_backend(
            auth::verify_otp(email, token),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(session) => this.complete_auth(session),
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn sign_out(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        match auth::clear_session() {
            Ok(()) => {
                self.backend_generation = self.backend_generation.wrapping_add(1);
                self.busy = false;
                self.auth = None;
                self.licensed = platform::is_licensed();
                self.page = Page::Auth;
                self.otp_sent = false;
                self.notice = None;
                self.qr_image = None;
                self.qr_status = "点击生成二维码后使用哔哩哔哩客户端扫码".into();
            }
            Err(error) => self.notice = Some((format!("退出登录失败：{error}"), true)),
        }
        cx.notify();
    }

    fn spawn_backend<T, Fut, F>(
        &mut self,
        future: Fut,
        window: &mut Window,
        cx: &mut Context<Self>,
        callback: F,
    ) where
        T: Send + 'static,
        Fut: Future<Output = Result<T, String>> + Send + 'static,
        F: FnOnce(&mut Self, Result<T, String>, &mut Window, &mut Context<Self>) + 'static,
    {
        let generation = self.backend_generation;
        self.busy = true;
        self.notice = None;
        cx.notify();

        let (sender, receiver) = async_channel::bounded(1);
        runtime().spawn(async move {
            let result = future.await;
            let _ = sender.send(result).await;
        });

        cx.spawn_in(window, async move |this, window| {
            let result = receiver
                .recv()
                .await
                .unwrap_or_else(|_| Err("后台任务意外终止".into()));
            let _ = this.update_in(window, move |this, window, cx| {
                if this.backend_generation != generation {
                    return;
                }
                this.busy = false;
                callback(this, result, window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn start_history_refresh(&self, window: &mut Window, cx: &mut Context<Self>) {
        let (sender, receiver) = async_channel::bounded(1);
        runtime().spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if sender.send(auto_reply::get_settings().await).await.is_err() {
                    break;
                }
            }
        });

        cx.spawn_in(window, async move |this, window| {
            while let Ok(result) = receiver.recv().await {
                let Ok(settings) = result else {
                    continue;
                };
                if this
                    .update_in(window, move |this, _, cx| {
                        if this.settings.history != settings.history {
                            this.settings.history = settings.history;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn start_qr_login(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.qr_status = "正在生成二维码".into();
        self.spawn_backend(
            bilibili::get_qr_code(),
            window,
            cx,
            |this, result, window, cx| match result {
                Ok(response) => match Self::decode_png(&response.qrcode) {
                    Ok(image) => {
                        this.qr_image = Some(image);
                        this.qr_status = "等待扫码".into();
                        this.poll_qr_login(window, cx);
                    }
                    Err(error) => {
                        this.qr_status = error.clone();
                        this.notice = Some((error, true));
                    }
                },
                Err(error) => {
                    this.qr_status = "二维码生成失败".into();
                    this.notice = Some((error, true));
                }
            },
        );
    }

    fn poll_qr_login(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.spawn_backend(
            async {
                for _ in 0..90 {
                    let status = bilibili::check_login_status().await?;
                    match status.status.as_str() {
                        "success" => return Ok(status),
                        "expired" => return Err("二维码已过期，请重新生成".into()),
                        _ => tokio::time::sleep(Duration::from_secs(2)).await,
                    }
                }
                Err("扫码等待超时，请重新生成二维码".into())
            },
            window,
            cx,
            |this, result, _, _| match result {
                Ok(_) => {
                    this.accounts = runtime()
                        .block_on(storage::get_accounts())
                        .unwrap_or_default();
                    this.qr_status = "登录成功，账号已保存".into();
                    this.notice = Some(("哔哩哔哩账号登录成功".into(), false));
                }
                Err(error) => {
                    this.qr_status = error.clone();
                    this.notice = Some((error, true));
                }
            },
        );
    }

    fn decode_png(encoded: &str) -> Result<Arc<Image>, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| format!("二维码解码失败：{error}"))?;
        Ok(Arc::new(Image::from_bytes(ImageFormat::Png, bytes)))
    }

    fn refresh_accounts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.spawn_backend(
            storage::get_accounts(),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(accounts) => {
                    this.accounts = accounts;
                    this.notice = Some(("账号列表已刷新".into(), false));
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn upload_cloud(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.auth.clone() else {
            self.notice = Some(("请先登录应用账号".into(), true));
            cx.notify();
            return;
        };
        self.spawn_backend(
            cloud::upload_all(session),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(message) => this.notice = Some((message, false)),
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn replace_settings(
        &mut self,
        settings: AutoReplySettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interval_input.update(cx, |input, cx| {
            input.set_value(settings.interval.to_string(), window, cx)
        });
        let messages = [
            settings.channels.comment.reply.message.clone(),
            settings.channels.dynamic.reply.message.clone(),
            settings.channels.direct_message.message.clone(),
            settings.channels.follow.message.clone(),
        ];
        for (input, message) in self.channel_messages.iter().zip(messages) {
            input.update(cx, |input, cx| input.set_value(message, window, cx));
        }
        self.tracked_editors = settings
            .tracked_videos
            .iter()
            .map(|video| TrackedVideoEditor::new(video, window, cx))
            .collect();
        self.settings = settings;
    }

    fn download_cloud(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.auth.clone() else {
            self.notice = Some(("请先登录应用账号".into(), true));
            cx.notify();
            return;
        };
        self.spawn_backend(
            cloud::download_all(session),
            window,
            cx,
            |this, result, window, cx| match result {
                Ok(download) => {
                    this.accounts = download.accounts;
                    if let Some(settings) = download.settings {
                        this.replace_settings(settings, window, cx);
                    }
                    this.notice = Some((
                        if download.downloaded_count == 0 {
                            "云端暂无可同步数据".into()
                        } else {
                            format!("已从云端同步 {} 项数据", download.downloaded_count)
                        },
                        false,
                    ));
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn activate_account(&mut self, uid: String, window: &mut Window, cx: &mut Context<Self>) {
        self.spawn_backend(
            async move {
                storage::activate_account(uid).await?;
                storage::get_accounts().await
            },
            window,
            cx,
            |this, result, _, _| match result {
                Ok(accounts) => {
                    this.accounts = accounts;
                    this.notice = Some(("已切换当前账号".into(), false));
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn delete_account(&mut self, uid: String, window: &mut Window, cx: &mut Context<Self>) {
        self.spawn_backend(
            async move {
                storage::delete_account(uid).await?;
                storage::get_accounts().await
            },
            window,
            cx,
            |this, result, _, _| match result {
                Ok(accounts) => {
                    this.accounts = accounts;
                    this.notice = Some(("账号已删除".into(), false));
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn collect_settings(&mut self, cx: &App) {
        let interval = self
            .interval_input
            .read(cx)
            .value()
            .parse::<u64>()
            .unwrap_or(60)
            .clamp(1, 3600);
        self.settings.interval = interval;
        self.settings.channels.comment.reply.message =
            self.channel_messages[0].read(cx).value().to_string();
        self.settings.channels.dynamic.reply.message =
            self.channel_messages[1].read(cx).value().to_string();
        self.settings.channels.direct_message.message =
            self.channel_messages[2].read(cx).value().to_string();
        self.settings.channels.follow.message =
            self.channel_messages[3].read(cx).value().to_string();

        for (video, editor) in self
            .settings
            .tracked_videos
            .iter_mut()
            .zip(self.tracked_editors.iter())
        {
            video.bvid = editor.bvid.read(cx).value().to_string();
            video.reply.message = editor.message.read(cx).value().to_string();
        }
    }

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.collect_settings(cx);
        let settings = self.settings.clone();
        self.spawn_backend(
            auto_reply::save_settings(settings),
            window,
            cx,
            |this, result, _, _| match result {
                Ok(()) => this.notice = Some(("自动回复设置已保存".into(), false)),
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn manual_reply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.collect_settings(cx);
        let settings = self.settings.clone();
        let source = self.current_source();
        self.spawn_backend(
            async move {
                auto_reply::save_settings(settings).await?;
                match source {
                    MsgSource::Comment => auto_reply::manual_reply_comments().await,
                    MsgSource::Dynamic => auto_reply::manual_reply_dynamic_comments().await,
                    _ => Err("仅评论渠道支持立即处理".into()),
                }
            },
            window,
            cx,
            |this, result, _, _| match result {
                Ok(message) => {
                    this.notice = Some((message, false));
                    this.settings = runtime()
                        .block_on(auto_reply::get_settings())
                        .unwrap_or_else(|_| this.settings.clone());
                }
                Err(error) => this.notice = Some((error, true)),
            },
        );
    }

    fn add_tracked_video(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let video = TrackedVideoSettings::default();
        self.tracked_editors
            .push(TrackedVideoEditor::new(&video, window, cx));
        self.settings.tracked_videos.push(video);
        cx.notify();
    }

    fn remove_tracked_video(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) {
        if index < self.settings.tracked_videos.len() {
            self.settings.tracked_videos.remove(index);
            self.tracked_editors.remove(index);
            cx.notify();
        }
    }

    fn render_auth(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .w(px(420.))
                    .gap_5()
                    .child(
                        v_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size_12()
                                    .rounded(theme.radius)
                                    .bg(theme.primary)
                                    .text_color(theme.primary_foreground)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_xl()
                                    .font_medium()
                                    .child("B"),
                            )
                            .child(div().text_2xl().font_medium().child("BiliAssist"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("登录后管理哔哩哔哩账号与自动回复"),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_4()
                            .p_6()
                            .border_1()
                            .border_color(theme.border)
                            .rounded(theme.radius)
                            .bg(theme.secondary)
                            .when_some(self.render_notice(cx), |form, notice| form.child(notice))
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(div().text_sm().font_medium().child("邮箱地址"))
                                    .child(Input::new(&self.email_input).w_full()),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(div().text_sm().font_medium().child("密码"))
                                    .child(Input::new(&self.password_input).w_full()),
                            )
                            .child(
                                Button::new("password-sign-in")
                                    .primary()
                                    .w_full()
                                    .label(if self.busy {
                                        "登录中"
                                    } else {
                                        "密码登录"
                                    })
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.sign_in_password(window, cx)
                                    })),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_3()
                                    .child(div().h_px().flex_1().bg(theme.border))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child("邮件验证码"),
                                    )
                                    .child(div().h_px().flex_1().bg(theme.border)),
                            )
                            .when(!self.otp_sent, |form| {
                                form.child(
                                    Button::new("send-otp")
                                        .outline()
                                        .w_full()
                                        .icon(IconName::Inbox)
                                        .label("发送验证码")
                                        .disabled(self.busy)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.send_otp(window, cx)
                                        })),
                                )
                            })
                            .when(self.otp_sent, |form| {
                                form.child(
                                    v_flex()
                                        .gap_3()
                                        .child(Input::new(&self.otp_input).w_full())
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .gap_3()
                                                .child(
                                                    Button::new("verify-otp")
                                                        .primary()
                                                        .flex_1()
                                                        .label("验证并登录")
                                                        .disabled(self.busy)
                                                        .on_click(cx.listener(
                                                            |this, _, window, cx| {
                                                                this.verify_otp(window, cx)
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new("resend-otp")
                                                        .outline()
                                                        .label("重新发送")
                                                        .disabled(self.busy)
                                                        .on_click(cx.listener(
                                                            |this, _, window, cx| {
                                                                this.send_otp(window, cx)
                                                            },
                                                        )),
                                                ),
                                        ),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let items = [
            (Page::Dashboard, "概览", IconName::GalleryVerticalEnd),
            (Page::Login, "扫码登录", IconName::SquareTerminal),
            (Page::Accounts, "账号管理", IconName::CircleUser),
            (Page::AutoReply, "自动回复", IconName::Bot),
            (Page::Sponsor, "支持项目", IconName::Heart),
        ];
        let auth_email = self
            .auth
            .as_ref()
            .map(|session| session.email.clone())
            .unwrap_or_default();

        v_flex()
            .w_64()
            .h_full()
            .p_4()
            .gap_2()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.sidebar)
            .child(
                h_flex()
                    .gap_3()
                    .px_2()
                    .py_3()
                    .child(
                        div()
                            .size_9()
                            .rounded(theme.radius)
                            .bg(theme.primary)
                            .text_color(theme.primary_foreground)
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_medium()
                            .child("B"),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(div().font_medium().child("BiliAssist"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("GPUI 原生桌面版"),
                            ),
                    ),
            )
            .child(div().h_3())
            .children(items.into_iter().map(|(page, label, icon)| {
                let active = self.page == page;
                Button::new(format!("nav-{label}"))
                    .ghost()
                    .w_full()
                    .justify_start()
                    .icon(icon)
                    .label(label)
                    .when(active, |button| button.bg(theme.sidebar_accent))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)),
                    )
            }))
            .child(div().flex_1())
            .child(
                h_flex()
                    .justify_between()
                    .px_2()
                    .py_2()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(div().text_sm().font_medium().child(auth_email))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(
                                format!(
                                    "{} · v{}",
                                    if self.licensed { "Plus" } else { "Basic" },
                                    env!("CARGO_PKG_VERSION")
                                ),
                            )),
                    )
                    .child(
                        Button::new("sign-out")
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip("退出登录")
                            .on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx))),
                    ),
            )
            .into_any_element()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let active = self.accounts.iter().find(|account| account.active);
        h_flex()
            .min_h_16()
            .px_7()
            .justify_between()
            .border_b_1()
            .border_color(theme.border)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_medium().child(self.page.title()))
                    .child(div().text_sm().text_color(theme.muted_foreground).child(
                        match self.page {
                            Page::Auth => "登录后使用本地账号管理功能",
                            Page::Dashboard => "查看运行状态和常用入口",
                            Page::Login => "使用哔哩哔哩客户端扫码添加账号",
                            Page::Accounts => "切换或移除本地加密保存的账号",
                            Page::AutoReply => "配置评论、动态、私信和关注回复",
                            Page::Sponsor => "支持项目持续维护",
                        },
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .rounded_full()
                            .bg(theme.secondary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::User).small()),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div().text_sm().font_medium().child(
                                    active
                                        .map(|account| account.name.clone())
                                        .unwrap_or_else(|| "未登录".into()),
                                ),
                            )
                            .child(
                                div().text_xs().text_color(theme.muted_foreground).child(
                                    active
                                        .map(|account| account.uid.clone())
                                        .unwrap_or_else(|| "添加账号后启用自动回复".into()),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_notice(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (message, is_error) = self.notice.as_ref()?;
        let theme = cx.theme().clone();
        Some(
            h_flex()
                .w_full()
                .gap_3()
                .p_3()
                .rounded(theme.radius)
                .bg(if *is_error {
                    theme.danger.opacity(0.12)
                } else {
                    theme.success.opacity(0.12)
                })
                .text_color(if *is_error {
                    theme.danger
                } else {
                    theme.success
                })
                .child(Icon::new(if *is_error {
                    IconName::TriangleAlert
                } else {
                    IconName::CircleCheck
                }))
                .child(div().text_sm().child(message.clone()))
                .into_any_element(),
        )
    }

    fn metric(
        &self,
        label: &str,
        value: String,
        detail: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .min_w_0()
            .flex_1()
            .gap_2()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(label.to_string()),
            )
            .child(div().text_2xl().font_medium().child(value))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(detail.to_string()),
            )
            .into_any_element()
    }

    fn render_dashboard(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let active = self.accounts.iter().find(|account| account.active);
        let enabled_channels = self.settings.enabled_sources().len();

        v_flex()
            .w_full()
            .gap_6()
            .child(
                h_flex().w_full().gap_4().children([
                    self.metric(
                        "当前账号",
                        active
                            .map(|account| account.name.clone())
                            .unwrap_or_else(|| "未登录".into()),
                        &format!("共 {} 个本地账号", self.accounts.len()),
                        cx,
                    ),
                    self.metric(
                        "自动回复",
                        if self.settings.enabled {
                            "运行中".into()
                        } else {
                            "已暂停".into()
                        },
                        &format!("{} 个渠道已启用", enabled_channels),
                        cx,
                    ),
                    self.metric(
                        "回复记录",
                        self.settings.history.len().to_string(),
                        "本地保存的最近回复",
                        cx,
                    ),
                ]),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .py_5()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(div().text_lg().font_medium().child("运行设置"))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_5()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().font_medium().child("开机自启"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("系统登录后自动启动并运行评论检查"),
                                    ),
                            )
                            .child(
                                Switch::new("dashboard-autostart")
                                    .checked(self.autostart)
                                    .on_click(cx.listener(|this, checked, window, cx| {
                                        this.toggle_autostart(*checked, window, cx)
                                    })),
                            ),
                    ),
            )
            .when(!self.licensed, |view| {
                view.child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .py_5()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(div().text_lg().font_medium().child("激活 Plus"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("激活后可使用自动回复和自动点赞"),
                        )
                        .child(
                            h_flex()
                                .max_w_128()
                                .gap_3()
                                .child(Input::new(&self.license_input).flex_1())
                                .child(
                                    Button::new("activate-license")
                                        .primary()
                                        .label("激活")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.activate_license(window, cx)
                                        })),
                                ),
                        ),
                )
            })
            .into_any_element()
    }

    fn render_login(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .w_full()
            .items_center()
            .gap_5()
            .py_8()
            .child(
                div()
                    .size(px(264.))
                    .border_1()
                    .border_color(theme.border)
                    .rounded(theme.radius)
                    .bg(theme.secondary)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if let Some(image) = self.qr_image.clone() {
                        img(image).size(px(232.)).into_any_element()
                    } else {
                        v_flex()
                            .items_center()
                            .gap_3()
                            .text_color(theme.muted_foreground)
                            .child(Icon::new(IconName::SquareTerminal).with_size(px(42.)))
                            .child(div().text_sm().child("二维码将在此显示"))
                            .into_any_element()
                    }),
            )
            .child(div().font_medium().child(self.qr_status.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("请使用哔哩哔哩客户端扫码并确认登录"),
            )
            .child(
                Button::new("generate-qrcode")
                    .primary()
                    .icon(IconName::LoaderCircle)
                    .label(if self.qr_image.is_some() {
                        "重新生成"
                    } else {
                        "生成二维码"
                    })
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| this.start_qr_login(window, cx))),
            )
            .into_any_element()
    }

    fn render_accounts(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .w_full()
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("本机加密保存 {} 个账号", self.accounts.len())),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("upload-cloud")
                                    .outline()
                                    .icon(IconName::ArrowUp)
                                    .label("上传云端")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.upload_cloud(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("download-cloud")
                                    .outline()
                                    .icon(IconName::ArrowDown)
                                    .label("下载云端")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.download_cloud(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("refresh-accounts")
                                    .outline()
                                    .icon(IconName::LoaderCircle)
                                    .label("刷新")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.refresh_accounts(window, cx)
                                    })),
                            ),
                    ),
            )
            .when(self.accounts.is_empty(), |view| {
                view.child(
                    v_flex()
                        .items_center()
                        .gap_3()
                        .py_16()
                        .text_color(theme.muted_foreground)
                        .child(Icon::new(IconName::CircleUser).with_size(px(42.)))
                        .child("暂无账号，请先前往扫码登录"),
                )
            })
            .children(self.accounts.iter().enumerate().map(|(index, account)| {
                let uid_activate = account.uid.clone();
                let uid_delete = account.uid.clone();
                h_flex()
                    .w_full()
                    .gap_4()
                    .p_4()
                    .border_1()
                    .border_color(if account.active {
                        theme.primary
                    } else {
                        theme.border
                    })
                    .rounded(theme.radius)
                    .child(
                        div()
                            .size_10()
                            .rounded_full()
                            .bg(theme.secondary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::User)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(div().font_medium().child(account.name.clone()))
                                    .when(account.active, |row| {
                                        row.child(
                                            div()
                                                .px_2()
                                                .py_0p5()
                                                .rounded_full()
                                                .bg(theme.primary.opacity(0.14))
                                                .text_xs()
                                                .text_color(theme.primary)
                                                .child("当前"),
                                        )
                                    }),
                            )
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                format!("UID {} · 添加于 {}", account.uid, account.created_at),
                            )),
                    )
                    .when(!account.active, |row| {
                        row.child(
                            Button::new(("activate-account", index))
                                .outline()
                                .label("设为当前")
                                .disabled(self.busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.activate_account(uid_activate.clone(), window, cx)
                                })),
                        )
                    })
                    .child(
                        Button::new(("delete-account", index))
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip("删除账号")
                            .disabled(self.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.delete_account(uid_delete.clone(), window, cx)
                            })),
                    )
            }))
            .into_any_element()
    }

    fn render_reply_policy(&self, cx: &mut Context<Self>) -> AnyElement {
        let policy = self.settings.channel(self.current_source()).reply_policy;
        h_flex()
            .gap_2()
            .child(
                Button::new("policy-per-message")
                    .label("每条消息")
                    .when(policy == ReplyPolicy::PerMessage, |button| button.primary())
                    .when(policy != ReplyPolicy::PerMessage, |button| button.outline())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.current_channel_mut().reply_policy = ReplyPolicy::PerMessage;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("policy-once-user")
                    .label("每个用户一次")
                    .when(policy == ReplyPolicy::OncePerUser, |button| {
                        button.primary()
                    })
                    .when(policy != ReplyPolicy::OncePerUser, |button| {
                        button.outline()
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.current_channel_mut().reply_policy = ReplyPolicy::OncePerUser;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn render_tracked_videos(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .w_full()
            .gap_4()
            .pt_5()
            .border_t_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_medium().child("指定视频"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("填写 BV 号后使用独立回复内容处理该视频评论"),
                            ),
                    )
                    .child(
                        Button::new("add-tracked-video")
                            .outline()
                            .icon(IconName::Plus)
                            .label("添加视频")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_tracked_video(window, cx)
                            })),
                    ),
            )
            .when(self.tracked_editors.is_empty(), |view| {
                view.child(
                    div()
                        .w_full()
                        .py_6()
                        .text_center()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .border_1()
                        .border_color(theme.border)
                        .rounded(theme.radius)
                        .child("尚未添加指定视频"),
                )
            })
            .children(self.tracked_editors.iter().enumerate().map(|(index, editor)| {
                let video = &self.settings.tracked_videos[index];
                v_flex()
                    .w_full()
                    .gap_3()
                    .py_4()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().font_medium().child(format!("视频 {}", index + 1)))
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        Switch::new(("tracked-enabled", index))
                                            .checked(video.reply.enabled)
                                            .on_click(cx.listener(move |this, checked, _, cx| {
                                                if let Some(video) = this.settings.tracked_videos.get_mut(index) {
                                                    video.reply.enabled = *checked;
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new(("remove-tracked", index))
                                            .ghost()
                                            .icon(IconName::Close)
                                            .tooltip("删除指定视频")
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.remove_tracked_video(index, window, cx)
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(div().text_sm().font_medium().child("BV 号"))
                                    .child(Input::new(&editor.bvid).w_full()),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(div().text_sm().font_medium().child("回复策略"))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                Button::new(("tracked-policy-message", index))
                                                    .label("每条消息")
                                                    .when(video.reply.reply_policy == ReplyPolicy::PerMessage, |button| button.primary())
                                                    .when(video.reply.reply_policy != ReplyPolicy::PerMessage, |button| button.outline())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        if let Some(video) = this.settings.tracked_videos.get_mut(index) {
                                                            video.reply.reply_policy = ReplyPolicy::PerMessage;
                                                        }
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                Button::new(("tracked-policy-user", index))
                                                    .label("每用户一次")
                                                    .when(video.reply.reply_policy == ReplyPolicy::OncePerUser, |button| button.primary())
                                                    .when(video.reply.reply_policy != ReplyPolicy::OncePerUser, |button| button.outline())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        if let Some(video) = this.settings.tracked_videos.get_mut(index) {
                                                            video.reply.reply_policy = ReplyPolicy::OncePerUser;
                                                        }
                                                        cx.notify();
                                                    })),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().font_medium().child("独立回复内容"))
                            .child(Textarea::new(&editor.message).h(px(96.)).w_full()),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("自动点赞该视频评论"),
                            )
                            .child(
                                Switch::new(("tracked-like", index))
                                    .checked(video.like_comments)
                                    .on_click(cx.listener(move |this, checked, _, cx| {
                                        if let Some(video) = this.settings.tracked_videos.get_mut(index) {
                                            video.like_comments = *checked;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
            }))
            .into_any_element()
    }

    fn render_auto_reply(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let source = self.current_source();
        let channel = self.settings.channel(source);
        let history = self
            .settings
            .history
            .iter()
            .rev()
            .filter(|item| item.source == source)
            .take(20)
            .collect::<Vec<_>>();

        v_flex()
            .w_full()
            .gap_6()
            .child(
                h_flex()
                    .justify_between()
                    .gap_5()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_medium().child("自动回复总开关"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("关闭后暂停回复，评论点赞仍按各渠道设置执行"),
                            ),
                    )
                    .child(
                        Switch::new("auto-reply-master")
                            .checked(self.settings.enabled)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.settings.enabled = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .gap_5()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_medium().child("检查间隔"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("范围 1 至 3600 秒"),
                            ),
                    )
                    .child(NumberInput::new(&self.interval_input).w(px(160.))),
            )
            .child(
                TabBar::new("reply-channel-tabs")
                    .w_full()
                    .segmented()
                    .selected_index(self.active_channel)
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.active_channel = *index;
                        cx.notify();
                    }))
                    .child(Tab::new().label("视频评论"))
                    .child(Tab::new().label("动态评论"))
                    .child(Tab::new().label("私信"))
                    .child(Tab::new().label("关注")),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .py_5()
                    .border_y_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_medium()
                                            .child(format!("{}自动回复", source.display_name())),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("该渠道可独立启用并配置内容"),
                                    ),
                            )
                            .child(
                                Switch::new("active-channel-enabled")
                                    .checked(channel.enabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.current_channel_mut().enabled = *checked;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .when_some(self.current_like_comments(), |section, like_comments| {
                        section.child(
                            h_flex()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_sm().font_medium().child("自动点赞评论"))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("点赞可独立于自动回复运行"),
                                        ),
                                )
                                .child(
                                    Switch::new("active-channel-like")
                                        .checked(like_comments)
                                        .on_click(cx.listener(|this, checked, _, cx| {
                                            this.set_current_like_comments(*checked);
                                            cx.notify();
                                        })),
                                ),
                        )
                    })
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().font_medium().child("回复策略"))
                            .child(self.render_reply_policy(cx)),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().font_medium().child("固定回复内容"))
                            .child(
                                Textarea::new(&self.channel_messages[self.active_channel])
                                    .h(px(116.))
                                    .w_full(),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("支持 {用户名}、{时间}"),
                            ),
                    )
                    .when(self.active_channel == 0, |section| {
                        section.child(self.render_tracked_videos(cx))
                    }),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("save-auto-reply")
                            .primary()
                            .icon(IconName::Check)
                            .label("保存设置")
                            .disabled(self.busy)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save_settings(window, cx)),
                            ),
                    )
                    .when(
                        matches!(source, MsgSource::Comment | MsgSource::Dynamic),
                        |row| {
                            row.child(
                                Button::new("manual-auto-reply")
                                    .outline()
                                    .icon(IconName::Play)
                                    .label(if source == MsgSource::Dynamic {
                                        "立即处理动态评论"
                                    } else {
                                        "立即处理视频评论"
                                    })
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.manual_reply(window, cx)
                                    })),
                            )
                        },
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .pt_5()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .font_medium()
                                    .child(format!("{}回复记录", source.display_name())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(history.len().to_string()),
                            ),
                    )
                    .when(history.is_empty(), |section| {
                        section.child(
                            div()
                                .py_8()
                                .text_center()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("暂无回复记录"),
                        )
                    })
                    .children(history.into_iter().map(|item| {
                        v_flex()
                            .gap_1()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(div().text_sm().font_medium().child(item.user.clone()))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(item.time.clone()),
                                    ),
                            )
                            .child(div().text_sm().child(item.message.clone()))
                    })),
            )
            .into_any_element()
    }

    fn render_sponsor(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let image = Arc::new(Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("../../../docs/sponsor-qr.png").to_vec(),
        ));
        v_flex()
            .w_full()
            .items_center()
            .gap_5()
            .py_8()
            .child(div().text_xl().font_medium().child("支持 BiliAssist"))
            .child(
                div()
                    .max_w_128()
                    .text_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("赞助将用于接口兼容、稳定性维护和多平台原生版本构建"),
            )
            .child(
                div()
                    .size(px(284.))
                    .p_3()
                    .border_1()
                    .border_color(theme.border)
                    .rounded(theme.radius)
                    .child(img(image).size_full()),
            )
            .child(
                Button::new("open-project")
                    .outline()
                    .icon(IconName::Github)
                    .label("查看项目")
                    .on_click(|_, _, cx| {
                        cx.open_url("https://github.com/YMRwithNoworry/biliassist")
                    }),
            )
            .into_any_element()
    }

    fn render_page(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.page {
            Page::Auth => self.render_auth(cx),
            Page::Dashboard => self.render_dashboard(cx),
            Page::Login => self.render_login(cx),
            Page::Accounts => self.render_accounts(cx),
            Page::AutoReply => self.render_auto_reply(cx),
            Page::Sponsor => self.render_sponsor(cx),
        }
    }
}

impl Focusable for AppView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        if self.auth.is_none() {
            return self.render_auth(cx);
        }

        h_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(self.render_sidebar(cx))
            .child(
                v_flex()
                    .min_w_0()
                    .h_full()
                    .flex_1()
                    .child(self.render_header(cx))
                    .child(
                        v_flex()
                            .id("main-content")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .p_7()
                            .gap_5()
                            .when_some(self.render_notice(cx), |content, notice| {
                                content.child(notice)
                            })
                            .child(self.render_page(cx)),
                    ),
            )
            .into_any_element()
    }
}
