use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{FixedOffset, TimeZone, Utc};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::Rng;

use crate::bilibili::UserInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub uid: String,
    pub name: String,
    pub avatar: String,
    pub cookie: String,
    pub active: bool,
    #[serde(alias = "created_at")]
    pub created_at: String,
}

const ACCOUNTS_FILE: &str = "bilibili_accounts.enc";

static DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

static QR_CODE_KEY: std::sync::OnceLock<Arc<RwLock<Option<String>>>> = std::sync::OnceLock::new();

fn get_data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        dirs::home_dir()
            .expect("无法获取用户目录")
            .join(".bilibili_account_manager")
    })
}

fn get_qrcode_key_lock() -> &'static Arc<RwLock<Option<String>>> {
    QR_CODE_KEY.get_or_init(|| Arc::new(RwLock::new(None)))
}

pub async fn init() {
    let dir = get_data_dir().clone();
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir)
            .await
            .expect("无法创建数据目录");
    }

    // 确保加密密钥存在
    let key_file = dir.join("key.bin");
    if !key_file.exists() {
        let mut key = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut key);
        tokio::fs::write(&key_file, &key)
            .await
            .expect("无法保存加密密钥");
    }
}

async fn get_encryption_key() -> Vec<u8> {
    let key_file = get_data_dir().join("key.bin");
    tokio::fs::read(&key_file)
        .await
        .expect("无法读取加密密钥")
}

async fn encrypt_data(data: &[u8]) -> Result<Vec<u8>, String> {
    let key = get_encryption_key().await;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("创建加密器失败: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    let mut rng = rand::thread_rng();
    rng.fill(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), data)
        .map_err(|e| format!("加密失败: {}", e))?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

async fn decrypt_data(encrypted: &[u8]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 12 {
        return Err("加密数据太短".to_string());
    }

    let key = get_encryption_key().await;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("创建解密器失败: {}", e))?;

    let (nonce, ciphertext) = encrypted.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("解密失败: {}", e))?;

    Ok(plaintext)
}

pub async fn save_account(user_info: &UserInfo) -> Result<(), String> {
    let mut accounts = load_accounts_internal().await?;

    let account = Account {
        uid: user_info.uid.clone(),
        name: user_info.name.clone(),
        avatar: user_info.avatar.clone(),
        cookie: user_info.cookie.clone(),
        active: accounts.is_empty(),
        created_at: FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .from_utc_datetime(&Utc::now().naive_utc())
            .to_rfc3339(),
    };

    // 如果新账号设为 active，其他取消
    if account.active {
        accounts.iter_mut().for_each(|a| a.active = false);
    }

    // 移除同 uid 的旧记录
    accounts.retain(|a| a.uid != account.uid);
    accounts.push(account);

    save_accounts_internal(&accounts).await
}

pub async fn get_accounts() -> Result<Vec<Account>, String> {
    load_accounts_internal().await
}

pub async fn sync_accounts(accounts: Vec<Account>) -> Result<Vec<Account>, String> {
    save_accounts_internal(&accounts).await?;
    Ok(accounts)
}

pub async fn activate_account(uid: String) -> Result<(), String> {
    let mut accounts = load_accounts_internal().await?;
    accounts.iter_mut().for_each(|a| a.active = a.uid == uid);
    save_accounts_internal(&accounts).await
}

pub async fn delete_account(uid: String) -> Result<(), String> {
    let mut accounts = load_accounts_internal().await?;
    let was_active = accounts.iter().any(|a| a.uid == uid && a.active);
    accounts.retain(|a| a.uid != uid);
    if was_active && !accounts.is_empty() {
        accounts[0].active = true;
    }
    save_accounts_internal(&accounts).await
}

pub async fn get_active_account() -> Option<UserInfo> {
    let accounts = load_accounts_internal().await.ok()?;
    let account = accounts.into_iter().find(|a| a.active)?;
    Some(UserInfo {
        uid: account.uid,
        name: account.name,
        avatar: account.avatar,
        cookie: account.cookie,
    })
}

async fn load_accounts_internal() -> Result<Vec<Account>, String> {
    let file_path = get_data_dir().join(ACCOUNTS_FILE);
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    let encrypted = match tokio::fs::read(&file_path).await {
        Ok(data) => data,
        Err(e) => {
            log::error!("读取账号文件失败: {}", e);
            return Ok(Vec::new());
        }
    };
    let decrypted = match decrypt_data(&encrypted).await {
        Ok(data) => data,
        Err(e) => {
            log::error!("解密账号数据失败: {}，可能密钥已变更，将清空旧数据", e);
            // 删除无法解密的旧文件
            let _ = tokio::fs::remove_file(&file_path).await;
            return Ok(Vec::new());
        }
    };
    match serde_json::from_slice(&decrypted) {
        Ok(accounts) => Ok(accounts),
        Err(e) => {
            log::error!("解析账号数据失败: {}，将清空旧数据", e);
            let _ = tokio::fs::remove_file(&file_path).await;
            Ok(Vec::new())
        }
    }
}

async fn save_accounts_internal(accounts: &[Account]) -> Result<(), String> {
    let json = serde_json::to_string(accounts)
        .map_err(|e| format!("序列化失败: {}", e))?;
    let encrypted = encrypt_data(json.as_bytes()).await?;
    let file_path = get_data_dir().join(ACCOUNTS_FILE);
    tokio::fs::write(&file_path, encrypted)
        .await
        .map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(())
}

// 二维码 key 临时存储
pub async fn save_current_qrcode_key(key: String) {
    let lock = get_qrcode_key_lock();
    let mut guard = lock.write().await;
    *guard = Some(key);
}

pub async fn get_current_qrcode_key() -> Option<String> {
    let lock = get_qrcode_key_lock();
    let guard = lock.read().await;
    guard.clone()
}
