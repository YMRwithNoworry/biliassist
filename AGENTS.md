# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

BilibiliAccountManager — a Tauri 2 desktop app for managing Bilibili (B站) accounts with QR login and automated reply features. The UI is in Chinese.

## Tech Stack

- **Frontend**: Vue 3 (Composition API) + Vite + Vue Router + Pinia
- **Backend**: Rust + Tauri 2 + Tokio async runtime
- **Auth**: Supabase (app-level user auth, separate from Bilibili auth)
- **Storage**: AES-256-GCM encrypted local files in `~/.bilibili_account_manager/`

## Common Commands

```bash
# Install frontend deps
npm install

# Development (starts Vite + Tauri dev server)
npm run tauri dev

# Production build
npm run tauri build

# Frontend only (Vite)
npm run dev
npm run build

# Rust backend only
cd src-tauri && cargo build
```

## Architecture

### Frontend (`src/`)
- `main.js` — Vue app entry, mounts Pinia + Router
- `App.vue` — root component, handles init loading and auth check via `useAuthStore`
- `router/index.js` — hash-based routing; all routes except `/auth` require Supabase authentication (`meta.requiresAuth`)
- `stores/auth.js` — Pinia store wrapping Supabase auth (email OTP, password login)
- `lib/supabase.js` + `lib/config.js` — Supabase client setup
- `views/` — page components: `HomeView`, `LoginView` (Bilibili QR), `AccountsView`, `AutoReplyView`, `SponsorView`, `AuthPage`

### Backend (`src-tauri/src/`)
- `main.rs` — thin entry, calls `lib::run()`
- `lib.rs` — Tauri app setup: registers all `#[tauri::command]` handlers, creates system tray, initializes storage/auto-reply on startup, handles autostart and window close-to-tray
- `bilibili.rs` — Bilibili API integration: QR code generation, login polling, user info retrieval. Uses a global `reqwest::Client` with cookie store
- `storage.rs` — encrypted account persistence (AES-256-GCM), QR code key temp storage. Data dir: `~/.bilibili_account_manager/`
- `auto_reply/` — modular auto-reply system:
  - `mod.rs` — `AutoReplyService` with `HandlerRegistry`, main loop, backward-compat API
  - `handler.rs` — `MessageHandler` trait, `HandlerRegistry`, `format_message()` with `{用户名}` and `{时间}` variables
  - `models.rs` — `AutoReplySettings`, `MsgSource` enum (Comment/DirectMessage/Follow), `ReplyHistory`
  - `state.rs` — global async state with `RwLock`
  - `comment.rs` — comment reply handler (uses WBI signing)
  - `direct_message.rs` — DM reply handler
  - `follow.rs` — follow event handler
  - `wbi.rs` — Bilibili WBI signature implementation
  - `http.rs` — shared HTTP client helpers

### Frontend-Backend Communication
All Tauri commands are defined in `lib.rs` with `#[tauri::command]` and invoked from the frontend via `@tauri-apps/api`. Key commands: `get_qr_code`, `check_login_status`, `get_accounts`, `sync_accounts`, `activate_account`, `delete_account`, `get_auto_reply_settings`, `save_auto_reply_settings`, `test_auto_reply`, `manual_reply_video_comments`, `get_autostart_status`, `set_autostart`.

## CI/CD

- `.github/workflows/release.yml` — auto-versioning (reads commit messages for `feat:`/`fix:` to determine semver bump), builds for Windows/macOS/Linux, creates GitHub releases
- `.github/workflows/pages.yml` — deploys `docs/` to GitHub Pages on push to main
- Version is synced across three files: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`

## Commit Convention

Conventional commits: `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:`. The release workflow uses these to auto-bump version numbers.

## Data Storage

All persistent data lives in `~/.bilibili_account_manager/`:
- `bilibili_accounts.enc` — AES-256-GCM encrypted account list
- `auto_reply_settings.json` — auto-reply config (plaintext JSON)
- `key.bin` — 32-byte encryption key
