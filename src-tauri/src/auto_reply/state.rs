use super::models::{AutoReplySettings, MsgSource, ReplyHistory};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const REPLIED_SET_FILE: &str = "replied_set.json";
const REPLIED_SET_MAX: usize = 10000;

/// 自动回复状态管理器
pub struct AutoReplyState {
    settings: Arc<RwLock<AutoReplySettings>>,
    replied_set: Arc<RwLock<HashSet<String>>>,
    data_dir: PathBuf,
}

impl AutoReplyState {
    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::home_dir()
            .ok_or("无法获取用户目录")?
            .join(".bilibili_account_manager");

        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;

        Ok(Self {
            settings: Arc::new(RwLock::new(AutoReplySettings::default())),
            replied_set: Arc::new(RwLock::new(HashSet::new())),
            data_dir,
        })
    }

    /// 从文件加载设置
    pub async fn load_settings(&self) {
        let file_path = self.data_dir.join("auto_reply_settings.json");
        if !file_path.exists() {
            return;
        }

        let json = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                log::warn!("读取自动回复设置失败: {}", e);
                return;
            }
        };

        let loaded: AutoReplySettings = match serde_json::from_str(&json) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("解析自动回复设置失败: {}", e);
                return;
            }
        };

        let mut settings = self.settings.write().await;
        settings.enabled = loaded.enabled;
        settings.message = loaded.message;
        settings.interval = loaded.interval;
        settings.reply_only_once = loaded.reply_only_once;
        settings.sources = loaded.sources;
        if !loaded.history.is_empty() {
            settings.history = loaded.history;
        }
        log::info!("已加载自动回复设置，历史记录 {} 条", settings.history.len());
    }

    /// 保存设置到文件
    pub async fn persist_settings(&self) {
        let settings = self.settings.read().await;
        let json = match serde_json::to_string(&*settings) {
            Ok(j) => j,
            Err(e) => {
                log::error!("序列化设置失败: {}", e);
                return;
            }
        };
        let file_path = self.data_dir.join("auto_reply_settings.json");
        if let Err(e) = tokio::fs::write(&file_path, json).await {
            log::error!("保存设置到文件失败: {}", e);
        }
    }

    /// 更新设置
    pub async fn update_settings<F, R>(&self, updater: F) -> Result<R, String>
    where
        F: FnOnce(&mut AutoReplySettings) -> R,
    {
        let mut settings = self.settings.write().await;
        let result = updater(&mut settings);
        drop(settings);
        self.persist_settings().await;
        Ok(result)
    }

    /// 获取设置副本
    pub async fn get_settings(&self) -> AutoReplySettings {
        self.settings.read().await.clone()
    }

    // ============================================================
    //  replied_set 持久化管理
    // ============================================================

    fn replied_set_path(&self) -> PathBuf {
        self.data_dir.join(REPLIED_SET_FILE)
    }

    /// 从磁盘加载已回复集合
    pub async fn load_replied_set(&self) {
        let file_path = self.replied_set_path();
        if !file_path.exists() {
            return;
        }

        let json = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                log::warn!("读取已回复集合失败: {}", e);
                return;
            }
        };

        let loaded: HashSet<String> = match serde_json::from_str(&json) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("解析已回复集合失败: {}", e);
                return;
            }
        };

        let mut set = self.replied_set.write().await;
        *set = loaded;
        let count = set.len();
        drop(set);
        log::info!("已加载已回复集合，共 {} 条记录", count);
    }

    /// 保存已回复集合到磁盘
    async fn persist_replied_set(&self) {
        let set = self.replied_set.read().await;
        let json = match serde_json::to_string(&*set) {
            Ok(j) => j,
            Err(e) => {
                log::error!("序列化已回复集合失败: {}", e);
                return;
            }
        };
        let file_path = self.replied_set_path();
        if let Err(e) = tokio::fs::write(&file_path, json).await {
            log::error!("保存已回复集合到文件失败: {}", e);
        }
    }

    /// 检查是否已回复过
    pub async fn is_replied(&self, key: &str) -> bool {
        let set = self.replied_set.read().await;
        set.contains(key)
    }

    /// 标记为已回复（同时持久化到磁盘）
    pub async fn mark_replied(&self, key: String) {
        {
            let mut set = self.replied_set.write().await;
            if set.len() >= REPLIED_SET_MAX {
                set.clear();
                log::warn!("已回复集合超过上限({})，已清空", REPLIED_SET_MAX);
            }
            set.insert(key);
        }
        self.persist_replied_set().await;
    }

    /// 从历史记录中检查是否已回复过某用户（针对私信/关注的降级保障）
    pub async fn is_replied_in_history(&self, user_identifier: &str, source: &MsgSource) -> bool {
        let settings = self.settings.read().await;
        settings.history.iter().any(|h| h.user == user_identifier && h.source == *source)
    }

    /// 添加回复历史记录
    pub async fn add_history(&self, user: String, message: String, source: MsgSource) {
        let history = ReplyHistory::new(user, message, source);
        self.update_settings(|settings| {
            settings.history.insert(0, history);
            if settings.history.len() > 100 {
                settings.history.truncate(100);
            }
        })
        .await
        .ok();
    }
}

/// 全局状态管理器实例
static GLOBAL_STATE: std::sync::OnceLock<Arc<AutoReplyState>> = std::sync::OnceLock::new();

/// 初始化全局状态
pub async fn init_global_state() {
    let state = match AutoReplyState::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("初始化状态管理器失败: {}", e);
            return;
        }
    };
    state.load_settings().await;
    state.load_replied_set().await;
    GLOBAL_STATE.get_or_init(|| state);
}

/// 获取全局状态管理器
pub fn get_global_state() -> &'static Arc<AutoReplyState> {
    GLOBAL_STATE.get().expect("全局状态未初始化，请先调用 init_global_state()")
}
