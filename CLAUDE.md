# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BilibiliAccountManager is a Tauri 2 desktop app for managing Bilibili accounts. The UI is Chinese and supports app-level Supabase auth, Bilibili QR login, encrypted local account storage, optional cloud account sync, plus-gated automation features, and automatic replies for comments/direct messages/follows.

The root directory is the active application. `tauri-app/` is a separate Vue/Vite starter template and should not be treated as the main app unless the user explicitly asks about it.

## Common Commands

Run commands from the repository root unless noted otherwise.

```bash
# Install dependencies
npm install
npm ci

# Full desktop app development: starts Vite before Tauri
npm run tauri dev

# Frontend-only Vite dev server (configured for localhost:1420, HMR 1421)
npm run dev

# Frontend production build only
npm run build

# Tauri production build; bundles are under src-tauri/target/release/bundle/
npm run tauri build

# Preview the frontend build
npm run preview

# Rust backend checks/builds
cd src-tauri && cargo check
cd src-tauri && cargo build
cd src-tauri && cargo build --release
cd src-tauri && cargo fmt

# Rust tests, if tests are added later
cd src-tauri && cargo test
cd src-tauri && cargo test <test_name>
```

Windows helper scripts exist for manual use:

```bash
start.bat
build.bat
```

There is currently no configured JavaScript lint script or project test script in `package.json`; do not claim one exists without adding it.

## Architecture

### Frontend (`src/`)

- `main.js` creates the Vue 3 app, installs Pinia and Vue Router, and mounts `App.vue`.
- `App.vue` performs startup session initialization via `useAuthStore()`, retries Supabase session loading, and renders either the init loading/error state or the active route.
- `router/index.js` uses hash routing. `/auth` is public; every other route requires an authenticated Supabase session.
- `stores/auth.js` is the Pinia auth store. It wraps Supabase auth, tracks `basic`/`plus` tier state, stores local license activation in `localStorage`, and exposes the session methods used by route guards and pages.
- `lib/supabase.js` creates the Supabase client from `lib/config.js`. The current config is hard-coded in source rather than read from `.env`; `.env.example` still documents the intended Vite variables.
- `views/` contains page-level flows:
  - `AuthPage.vue` handles app-level Supabase sign-in/sign-up.
  - `HomeView.vue` is the main navigation and Plus/license entry point.
  - `LoginView.vue` drives Bilibili QR login via Tauri commands.
  - `AccountsView.vue` manages local Bilibili accounts and Supabase cloud sync.
  - `AutoReplyView.vue` edits auto-reply, AI reply, comment-like, manual trigger, and autostart settings.
  - `SponsorView.vue` contains sponsor/upgrade UI.

### Backend (`src-tauri/src/`)

- `main.rs` is a thin binary entrypoint that calls `bilibili_account_manager_lib::run()`.
- `lib.rs` wires Tauri plugins, registers all `#[tauri::command]` handlers, initializes storage and auto-reply state at startup, starts the background auto-reply loop, creates the system tray, implements close-to-tray behavior, and guards autostart in dev mode.
- `bilibili.rs` integrates Bilibili QR login: generates QR codes, polls login status, extracts cookies from Bilibili responses, fetches user info, and saves successful logins.
- `storage.rs` persists Bilibili accounts in `~/.bilibili_account_manager/bilibili_accounts.enc` using AES-256-GCM. It also stores the active QR-code key in process memory.
- `auto_reply/` is the automation subsystem:
  - `mod.rs` owns `AutoReplyService`, registers handlers, runs the polling loop, and exposes command-facing helpers.
  - `models.rs` defines `AutoReplySettings`, `MsgSource`, `ReplyHistory`, and OpenAI-compatible `AiReplyConfig`.
  - `state.rs` holds global async state and persists settings/history plus replied/liked sets.
  - `handler.rs` defines the `MessageHandler` trait, registry, message formatting, and shared handler result types.
  - `comment.rs`, `direct_message.rs`, and `follow.rs` implement source-specific processing.
  - `wbi.rs` signs Bilibili WBI requests.
  - `http.rs` contains shared HTTP helpers.
  - `ai.rs` calls an OpenAI-compatible `/chat/completions` endpoint for generated replies.

### Tauri command boundary

Frontend pages call Rust through `@tauri-apps/api/core` `invoke(...)`. Commands are registered in `src-tauri/src/lib.rs`; keep the frontend command names, argument casing, Rust command signatures, and serde `rename_all = "camelCase"` models aligned.

Current command set includes:

- Bilibili login/account: `get_qr_code`, `check_login_status`, `get_accounts`, `sync_accounts`, `activate_account`, `delete_account`.
- Auto-reply and AI: `get_auto_reply_settings`, `save_auto_reply_settings`, `test_auto_reply`, `test_ai_reply`, `manual_reply_video_comments`.
- License/autostart/utilities: `verify_license`, `generate_qr_code`, `get_autostart_status`, `set_autostart`.

### Persistence and user data

Runtime data is stored under `~/.bilibili_account_manager/`:

- `bilibili_accounts.enc` — encrypted Bilibili account list.
- `key.bin` — 32-byte AES key used for account encryption.
- `auto_reply_settings.json` — plaintext auto-reply settings, reply history, and AI config including API key.
- `replied_set.json` — IDs already replied to when one-time reply behavior is enabled.
- `liked_set.json` — IDs already liked by the comment-like automation.

Be careful when testing against real local data: deleting or changing `key.bin` can make existing encrypted accounts unreadable.

## Build and Release

- `src-tauri/tauri.conf.json` runs `npm run dev` before Tauri dev, `npm run build` before Tauri build, and points Tauri at `../dist`.
- `vite.config.js` pins the dev server to `127.0.0.1:1420` and ignores large/generated directories in file watching.
- `.github/workflows/release.yml` triggers on pushes to `main`/`master`, version tags, or manual dispatch. It auto-bumps versions based on the latest commit message unless manually specified, updates `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, builds Windows/macOS/Linux packages, and edits the GitHub Release notes.
- `.github/workflows/pages.yml` deploys the `docs/` directory to GitHub Pages when docs change.
- Keep the app version synchronized in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` when changing versions outside the release workflow.

## Repository Notes

- Root `README.md`, `QUICKSTART.md`, `API.md`, and `PROJECT_STATUS.md` describe older and current behavior; verify against source before relying on details, especially API shapes and file layout.
- `CONTRIBUTING.md` documents Node.js 18+ and Rust 1.70+ as development prerequisites and conventional commit prefixes.
- `.codeartsdoer/rule/1.mdc` asks contributors to prioritize performance and minimize overhead; apply that as a project preference, not as permission to bypass correctness or safety.
- The UI uses a dark, GitHub-like visual style in page-scoped CSS. Avoid broad style rewrites unless requested.
