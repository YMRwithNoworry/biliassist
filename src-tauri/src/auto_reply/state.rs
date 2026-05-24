use super::models::{AutoReplySettings, MsgSource, ReplyHistory};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const REPLIED_SET_FILE: &str = "replied_set.json";
const REPLIED_SET_MAX: usize = 10000;

/// \u{81ea}\u{52a8}\u{56de}\u{590d}\u{72b6}\u{6001}\u{7ba1}\u{7406}\u{5668}
pub struct AutoReplyState {
    settings: Arc<RwLock<AutoReplySettings>>,
    replied_set: Arc<RwLock<HashSet<String>>>,
    data_dir: PathBuf,
}

impl AutoReplyState {
    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::home_dir()
            .ok_or("\u{65e0}\u{6cd5}\u{83b7}\u{53d6}\u{7528}\u{6237}\u{76ee}\u{5f55}")?
            .join(".bilibili_account_manager");

        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("\u{521b}\u{5efa}\u{6570}\u{636e}\u{76ee}\u{5f55}\u{5931}\u{8d25}: {}", e))?;

        Ok(Self {
            settings: Arc::new(RwLock::new(AutoReplySettings::default())),
            replied_set: Arc::new(RwLock::new(HashSet::new())),
            data_dir,
        })
    }

    /// \u{4ece}\u{6587}\u{4ef6}\u{52a0}\u{8f7d}\u{8bbe}\u{7f6e}
    pub async fn load_settings(&self) {
        let file_path = self.data_dir.join("auto_reply_settings.json");
        if !file_path.exists() {
            return;
        }

        let json = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                log::warn!("\u{8bfb}\u{53d6}\u{81ea}\u{52a8}\u{56de}\u{590d}\u{8bbe}\u{7f6e}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };

        let loaded: AutoReplySettings = match serde_json::from_str(&json) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("\u{89e3}\u{6790}\u{81ea}\u{52a8}\u{56de}\u{590d}\u{8bbe}\u{7f6e}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };

        let mut settings = self.settings.write().await;
        settings.enabled = loaded.enabled;
        settings.message = loaded.message;
        settings.interval = loaded.interval;
        settings.reply_only_once = loaded.reply_only_once;
        settings.sources = loaded.sources;
        settings.like_comments = loaded.like_comments;
        settings.ai = loaded.ai;
        if !loaded.history.is_empty() {
            settings.history = loaded.history;
        }
        log::info!("\u{5df2}\u{52a0}\u{8f7d}\u{81ea}\u{52a8}\u{56de}\u{590d}\u{8bbe}\u{7f6e}\u{ff0c}\u{5386}\u{53f2}\u{8bb0}\u{5f55} {} \u{6761}", settings.history.len());
    }

    /// \u{4fdd}\u{5b58}\u{8bbe}\u{7f6e}\u{5230}\u{6587}\u{4ef6}
    pub async fn persist_settings(&self) {
        let settings = self.settings.read().await;
        let json = match serde_json::to_string(&*settings) {
            Ok(j) => j,
            Err(e) => {
                log::error!("\u{5e8f}\u{5217}\u{5316}\u{8bbe}\u{7f6e}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };
        let file_path = self.data_dir.join("auto_reply_settings.json");
        if let Err(e) = tokio::fs::write(&file_path, json).await {
            log::error!("\u{4fdd}\u{5b58}\u{8bbe}\u{7f6e}\u{5230}\u{6587}\u{4ef6}\u{5931}\u{8d25}: {}", e);
        }
    }

    /// \u{66f4}\u{65b0}\u{8bbe}\u{7f6e}
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

    /// \u{83b7}\u{53d6}\u{8bbe}\u{7f6e}\u{526f}\u{672c}
    pub async fn get_settings(&self) -> AutoReplySettings {
        self.settings.read().await.clone()
    }

    // ============================================================
    //  replied_set \u{6301}\u{4e45}\u{5316}\u{7ba1}\u{7406}
    // ============================================================

    fn replied_set_path(&self) -> PathBuf {
        self.data_dir.join(REPLIED_SET_FILE)
    }

    /// \u{4ece}\u{78c1}\u{76d8}\u{52a0}\u{8f7d}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}
    pub async fn load_replied_set(&self) {
        let file_path = self.replied_set_path();
        if !file_path.exists() {
            return;
        }

        let json = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                log::warn!("\u{8bfb}\u{53d6}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };

        let loaded: HashSet<String> = match serde_json::from_str(&json) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("\u{89e3}\u{6790}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };

        let mut set = self.replied_set.write().await;
        *set = loaded;
        let count = set.len();
        drop(set);
        log::info!("\u{5df2}\u{52a0}\u{8f7d}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{ff0c}\u{5171} {} \u{6761}\u{8bb0}\u{5f55}", count);
    }

    /// \u{4fdd}\u{5b58}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{5230}\u{78c1}\u{76d8}
    async fn persist_replied_set(&self) {
        let set = self.replied_set.read().await;
        let json = match serde_json::to_string(&*set) {
            Ok(j) => j,
            Err(e) => {
                log::error!("\u{5e8f}\u{5217}\u{5316}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{5931}\u{8d25}: {}", e);
                return;
            }
        };
        let file_path = self.replied_set_path();
        if let Err(e) = tokio::fs::write(&file_path, json).await {
            log::error!("\u{4fdd}\u{5b58}\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{5230}\u{6587}\u{4ef6}\u{5931}\u{8d25}: {}", e);
        }
    }

    /// \u{68c0}\u{67e5}\u{662f}\u{5426}\u{5df2}\u{56de}\u{590d}\u{8fc7}
    pub async fn is_replied(&self, key: &str) -> bool {
        let set = self.replied_set.read().await;
        set.contains(key)
    }

    /// \u{6807}\u{8bb0}\u{4e3a}\u{5df2}\u{56de}\u{590d}\u{ff08}\u{540c}\u{65f6}\u{6301}\u{4e45}\u{5316}\u{5230}\u{78c1}\u{76d8}\u{ff09}
    pub async fn mark_replied(&self, key: String) {
        {
            let mut set = self.replied_set.write().await;
            if set.len() >= REPLIED_SET_MAX {
                set.clear();
                log::warn!("\u{5df2}\u{56de}\u{590d}\u{96c6}\u{5408}\u{8d85}\u{8fc7}\u{4e0a}\u{9650}({})\u{ff0c}\u{5df2}\u{6e05}\u{7a7a}", REPLIED_SET_MAX);
            }
            set.insert(key);
        }
        self.persist_replied_set().await;
    }

    /// \u{4ece}\u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{4e2d}\u{68c0}\u{67e5}\u{662f}\u{5426}\u{5df2}\u{56de}\u{590d}\u{8fc7}\u{67d0}\u{7528}\u{6237}\u{ff08}\u{9488}\u{5bf9}\u{79c1}\u{4fe1}/\u{5173}\u{6ce8}\u{7684}\u{964d}\u{7ea7}\u{4fdd}\u{969c}\u{ff09}
    pub async fn is_replied_in_history(&self, user_identifier: &str, source: &MsgSource) -> bool {
        let settings = self.settings.read().await;
        settings.history.iter().any(|h| h.user == user_identifier && h.source == *source)
    }

    /// \u{6dfb}\u{52a0}\u{56de}\u{590d}\u{5386}\u{53f2}\u{8bb0}\u{5f55}
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

/// \u{5168}\u{5c40}\u{72b6}\u{6001}\u{7ba1}\u{7406}\u{5668}\u{5b9e}\u{4f8b}
static GLOBAL_STATE: std::sync::OnceLock<Arc<AutoReplyState>> = std::sync::OnceLock::new();

/// \u{521d}\u{59cb}\u{5316}\u{5168}\u{5c40}\u{72b6}\u{6001}
pub async fn init_global_state() {
    let state = match AutoReplyState::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("\u{521d}\u{59cb}\u{5316}\u{72b6}\u{6001}\u{7ba1}\u{7406}\u{5668}\u{5931}\u{8d25}: {}", e);
            return;
        }
    };
    state.load_settings().await;
    state.load_replied_set().await;
    GLOBAL_STATE.get_or_init(|| state);
}

/// \u{83b7}\u{53d6}\u{5168}\u{5c40}\u{72b6}\u{6001}\u{7ba1}\u{7406}\u{5668}
pub fn get_global_state() -> &'static Arc<AutoReplyState> {
    GLOBAL_STATE.get().expect("\u{5168}\u{5c40}\u{72b6}\u{6001}\u{672a}\u{521d}\u{59cb}\u{5316}\u{ff0c}\u{8bf7}\u{5148}\u{8c03}\u{7528} init_global_state()")
}
