<div align="center">

# ⚡ x-hub — Local Personal Productivity Workspace

Built with **Tauri 2 + Vue 3 + TypeScript**, this desktop productivity tool follows a Bento-style interface.
**Your data is stored locally by default.** Content leaves your machine only when you use features that call external services, such as AI providers, weather and quote lookups, or extension market and update checks.

![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)
![Vue](https://img.shields.io/badge/Vue-3.x-42b883?logo=vuedotjs&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-6.x-3178c6?logo=typescript&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.77+-dea584?logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-local-003b57?logo=sqlite&logoColor=white)
![Version](https://img.shields.io/badge/version-0.9.7-blue)
![License](https://img.shields.io/badge/license-MIT-blue)

</div>

## ✨ Features

### 🕐 Workbench
A **freely arranged Bento grid**: clock (**large clock / lunar calendar / minimal time** in three modes; quotes from online Hitokoto, with local fallback when offline), **weather card** (compact/detail modes, Open-Meteo live temperature/feels-like/humidity/wind + city/IP detection), system monitor (CPU/memory, 2s polling), sticky notes ×2 (600ms debounced auto-save), **diary** (press Enter to save into Speednote), Speednote / Todo / Suda **overview cards**, prompt toolbox, to-do list, **todo calendar** (month view with todo distribution and overdue highlights), countdown list, **custom Suda ×4** (pin favorite resources into one-click launch groups), recent usage bar — all modules can be added, removed, dragged, resized, and switched between variants in the **WYSIWYG layout editor**, and saved as a default layout for quick restoration.

### ⏳ Countdown
Four modes: duration / scheduled / daily / interval (**up to 6**); pause/resume/floating window/delete; **background-driven** (Rust 1s polling; system notification when due; `once` enters gray state, `daily` and `interval` auto-advance, and missed events when sleeping are silently postponed); can float as a **transparent circular countdown window** (animated water-level waves, independent topmost mini-window, position persistence); optional due-time sound (WebAudio-generated dual-tone chime).

### 🚀 Suda
Unified management for **applications / web pages / files**; **category × subcategory** two-level filtering with **custom zones** for grouped display; drag-and-drop import of exe/lnk files with **automatic extraction of high-resolution app icons** (256×256 native extraction, including .lnk parsing); **scan installed apps in bulk**; one-click launch — visible or minimized app windows are brought to the foreground, while tray-hidden single-instance apps are relaunched to restore their windows where supported; web items can be opened **in-app** (embedded panel or standalone browser mini-window with tabs) or in the **default browser**; descriptions + **encrypted notes** (encrypted locally, viewable with eye toggle); right-click menu actions; delete with undo support.

### 📝 Speednote
**WYSIWYG Markdown editor** (real-time preview / split / source / preview-only modes, 600ms debounced auto-save, automatic title derivation); **paste / drag / click-to-upload images** directly saved locally (deduplicated by content hash); **backlinks** `[[notes]]` completion, navigation and backlink panel; **folder tree** management (nested / drag-move), tag filtering, **trash** (soft-delete with restore), **clip collection** (source link included for deduplication); AI **deep polish** (full document or selected fragment semantic reorganization, with explicit pre-send confirmation) and **one-click formatting cleanup**; notes support full-library **export / import**.

### 🔍 Global Search
`Ctrl+K` to open, 300ms debounce, searches **resources, notes, and todos** simultaneously, with click-to-open results.

### ✅ Todo List
Priority dots, **due date and reminder** (system notification when due), **subtasks** (parent row shows progress), **pinning**, **recurring todos** (daily/weekly/monthly…; marking complete advances to the next cycle without stacking overdue duplicates), **tags**, in-group **drag-and-drop sorting**, inline editing, delete with undo; independent Todo view includes **month / week calendar** (drag to reschedule cells, overdue faint red highlights, recurring virtual instances) and a two-dimensional “scope × mode” filter; supports global search direct jump and highlighting.

### 📋 Clipboard History
`Ctrl+\`` globally opens the floating overlay; stores **text / image / file** content — copying images/screenshots automatically saves thumbnails, copying files saves paths; supports paste back to clipboard, image preview and “save image”, and auto-deduplication for identical content; image/file history toggles are configurable.

### 🤖 AI Assistant (Chat + Agent)
**OpenAI-compatible streaming chat** (SSE typewriter effect + Markdown rendering, multi-session management; embedded panel can dock to **any side** or open as an **independent mini-window**, triggered by `Ctrl+Shift+K`), with an integrated **host-level Agent runtime** — AI can do more than chat: **34 built-in tools** across notes / todos / Suda resources / countdown / clipboard / prompts / sticky notes for queries and writes (including launching apps), and extensions can also expose tools to the AI assistant via self-description. **Three permission levels**: read-only / ask-each-time (inline confirmation card inside the dialog + “allow this type of action for this session”) / full permission; each tool call is logged in the timeline and can be interrupted at any time. **Dual model onboarding**: platform free quota (**points system** billed by token usage, remaining balance shown in the chat header and redemption hints when low) and **BYOK self-hosted keys** for direct access to mainstream vendors (**one-click presets**, connectivity testing and bulk model list retrieval, API keys stored in the **system keychain**, masked in UI and not written in plaintext), configurable at any time in settings.

### 💬 Prompt Vault
Management for recurring prompt snippets; pinning + copy count tracking; one-click copy from cards.

### 🧩 Extension System
The **Extension Center** shows installed extensions locally, supports installation from a **GitHub repo zip URL**, uninstall, and update checks; the **manifest registry** resolves permission declarations and version metadata; extensions support **module cards** (embedded in the main UI) / **view pages** / **window / drawer floating windows**; the host injects the **bridge API (`window.xhub`)** to interact with the app (permissions are granted per manifest item, and **high-risk capabilities are explicitly shown before install**; opening external links requires `open-url` permission, otherwise a visible message is shown instead of silent failure); **service hosting** downloads the runtime on demand with automatic fallback; extensions can be **pinned to the left sidebar**, click to open, and choose **view / window / drawer** modes; built-in **extension marketplace** (remote manifest signature validation, install from registry, category filtering, official badge, install counts, one-click updates); extensions can expose their capabilities to the AI assistant via manifest; built-in **extension development skill package (Skills)** can be installed into the local AI coding assistant.

### ⚙️ System Settings
Two-level navigation with **9 categories** (General / Appearance / Workbench / AI Assistant / Features / Extensions / Account / Data & About / Guide) + **settings search**; **built-in guide** (inline `docs/user-guide.md`: left-side chapter tree + in-page search, every feature’s operation path is traceable); **3-axis theme** (mode Light/Dark/System × 10 solid colors + 10 gradient presets × accent colors in 8 presets/custom), **wallpaper** (blur / mask / immersive mode / glass opacity), **global hotkey capture** (all seven global shortcuts can be remapped or individually disabled with conflict pre-checking), **startup on login** (Run key registration with silent tray residency after login), **Workbench Layout Editor**, **network** (global toggle / city settings / quote source), **desktop floating ball** switch, **AI assistant** (permission tier / provider models / quota / conversation mode), **data backup and restore**, **data storage path**.

### 🖥️ Window Capabilities
Borderless + translucent custom title bar (drag / maximize / restore / pin button / close to tray); system tray stays resident; global shortcuts can invoke features; window position/size are remembered (multi-monitor / DPI-aware); sticky notes / countdown / prompts / todo can open as independent floating windows; **desktop floating ball** (edge-peeking, hover expansion, circular menu for quick access to common features, drag free placement and snap-to-edge behavior).

### 🔄 Auto-Update
**Custom update flow**: fetches the new version info from `releases/update.json` (Ed25519 detached signature verification; embedded public key must validate before trust is granted) → semver version comparison + **minimum upgrade protection** (`minimumUpgradable`) → **automatic silent check** (5s after startup + default every 4 hours, can be disabled in About); when an update is found, a **global update dialog** appears (version number / changelog / size / portable flag), with support for **skip this version**.

### 🧰 Prerequisites
- **Windows 10/11** with the WebView2 runtime. The codebase uses Windows-only APIs (WebView2, DPAPI, registry, COM), so Windows is the only supported build target.
- **Rust 1.77.2 or newer** with the MSVC toolchain and the Windows SDK (the test wrapper uses `mt.exe` from the SDK).
- **Node.js 24**, as used in CI. `npm run test:utils` loads `.ts` files directly, which CI runs on Node 24.
- **PowerShell 7 (`pwsh`)** for `npm run tauri:test`.

### 📚 Command Reference
```bash
npm run dev           # Vite browser preview (http://localhost:1420)
npm run tauri:dev     # Tauri desktop development window
npm run build         # vue-tsc type-check + vite build
npm run tauri:build -- --bundles nsis  # Windows NSIS installer: src-tauri/target/release/bundle/nsis/
npm run test:utils    # Focused frontend utility tests
npm run tauri:test    # Rust unit tests (Windows manifest wrapper)
```

### 🧰 VS Code Workspace

Open [`x-hub.code-workspace`](./x-hub.code-workspace) or open the repository root folder. Do not open the `agent` directory: it does not exist at the repository root; the Rust agent implementation is under `src-tauri/src/agent/`.

The workspace sets new integrated terminals and tasks to `${workspaceFolder}`. If VS Code still reports the obsolete `...\agent` working directory, check user-level terminal profiles or launch settings for a stale `cwd`, close the affected terminal, and create a new one from the workspace.

Available VS Code tasks include frontend build/tests, Rust checks/tests, launching the Tauri development app, producing a Windows NSIS installer, and validating workspace configuration. The Rust test task is Windows-only and requires PowerShell 7 plus the Windows SDK. Rust debugging uses CodeLLDB; its pre-launch task starts Vite and builds the debug executable.

This project does not currently declare application environment variables or include a `.env` file. No `envFile` is configured; do not add secrets to the workspace configuration. If future features introduce required variables, document their names in a committed example file and keep real `.env` files untracked.

## 📂 Project Structure

```text
src/
├── main.ts / App.vue        # Entry and window shell (App.vue routes by window label: main / sticky / countdown / clipboard / extension / prompt / todo / AI chat / floating ball / notice)
├── index/index.vue          # Home screen: sidebar navigation (Workbench / Todo / Speednote / Suda + extension center) + theme + layout coordination + search/settings coordination
├── style.css                # Design tokens (light/dark) + common styles
├── api/tauri.ts             # Type-safe Tauri invoke wrapper
├── stores/workbench.ts      # Reactive state management (workbench / sticky / todo / countdown / prompts / AI chat / extensions / auto-update)
├── composables/             # Composable functions (theme / layout editor / extension bridge / shortcut recorder / Suda drag…)
├── utils/                   # File categories / time / lunar / weather codes / todo scheduling / sound / error reporting
└── components/              # Feature components (workbench cards / Suda / Speednote / Todo / search / AI chat / extension center / settings / update dialogs…)

src-tauri/src/
├── lib.rs                   # App bootstrapping: database / tray / shortcuts / window state / singleton / data migration / command registration
├── commands.rs / models.rs  # Tauri command handlers and data models
├── db.rs / config.rs        # SQLite migration + persistent configuration
├── agent/                   # Agent runtime (34 built-in tool registry / turn loop / 3 permission tiers + session-scoped authorization / context trimming / system prompt / extension tool aggregation)
├── chat.rs                  # OpenAI-compatible SSE streaming client (including tool_calls parsing) + API key storage in the system keychain
├── account.rs               # Platform account (login / signup / points quota)
├── extension.rs / xhub_api.rs / service.rs / proxy.rs  # Extension system (scan + install / bridge API / Node hosting / reverse proxy)
├── market.rs / updater.rs   # Extension marketplace + app auto-update (Ed25519 verification + download validation + restart self-replacement)
├── suda_browser.rs          # Suda “open web in-app” feature (embedded panel + browser window pool)
├── app_icon.rs              # Native app icon extraction (256×256, .lnk parsing)
├── process.rs               # App startup and window activation / URL open / privilege escalation (UAC)
├── note_io.rs               # Speednote export / import
├── todo_recurrence.rs / todo_reminder.rs  # Recurring todo rules (single implementation) + reminder background thread
├── countdown_ticker.rs / countdown_window.rs  # Countdown background driver + circular floating window
├── floating_ball.rs         # Desktop floating ball (edge snap / hover expansion / circular menu)
├── notify.rs                # Bottom-right custom notification window (works consistently across Win10/11; frontend renders NoticeOverlay.vue cards)
├── clipboard.rs             # Clipboard monitoring + history + paste injection
├── shortcut.rs / tray.rs    # Global shortcuts (7) / system tray
├── sysmon.rs                # System monitoring (CPU / memory)
└── repo/                    # Data access layer (resources / notes / todos / stickies / prompts / countdown / chat / clipboard / tags…)
```

## 🔒 Data & Privacy

Data is stored under the **Data Root** directory (standard version defaults to `%APPDATA%\x-hub`, configurable in settings; portable mode uses `exe\data`):

- **Database**: `DataRoot\app.db` (SQLite, 20+ tables: Suda resources and subcategories/zones, notes and folders/links, todos and tags, stickies, prompts, countdown, AI chat, clipboard history, etc.)
- **Icons**: `DataRoot\icons\` (program icons automatically extracted during drag-drop import or app scan)
- **Note images**: `DataRoot\notes\images\` (editor-uploaded images, deduplicated by content hash)
- **Wallpapers**: `DataRoot\wallpapers\` (imported app wallpapers, hashed by content)
- **Logs**: `DataRoot\logs\x-hub.log` (file-based log for troubleshooting)
- **Clipboard snapshots**: `DataRoot\clipboard\images\` (thumbnail snapshots for copied images; automatically deleted on delete / clear / expiration)
- **Backups**: built-in backup/restore in settings, packaged as `x-hub-backup-timestamp.zip` (database + icons)
- **App updates**: downloaded update packages are stored under `DataRoot\updates\`, then cleaned up after self-replacement succeeds or a rollback is triggered
- **AI chat**: sessions and messages are stored in local SQLite; API keys are saved in the **system keychain** (keyring), hidden in the UI, **not stored in plaintext**, and not uploaded

## Screenshots
<img width="1400" height="933" alt="Workbench overview" src="https://github.com/user-attachments/assets/ffb4358b-8e39-4388-aab6-3827e64ed883" />
<img width="1400" height="933" alt="Workbench (second screenshot)" src="https://github.com/user-attachments/assets/aa7323e6-e66b-411e-9c9b-b63f3c057c88" />
<img width="1400" height="933" alt="todo" src="https://github.com/user-attachments/assets/324cc42f-8e6f-477d-bd2b-aa2d3d38f1ee" />
<img width="1400" height="933" alt="settings" src="https://github.com/user-attachments/assets/7ea3db47-f5a6-40db-b632-f2fee0ec4215" />
<img width="1400" height="933" alt="notes" src="https://github.com/user-attachments/assets/8c0bd21e-187d-4117-a098-640c3b95451c" />
<img width="1400" height="933" alt="launcher" src="https://github.com/user-attachments/assets/c3e827c0-fcb4-4cec-ad5e-789cf8669bff" />
<img width="1400" height="933" alt="extensions" src="https://github.com/user-attachments/assets/6ddd20ce-b35c-4db0-8a37-cd1c4439dab1" />

## 💬 Community Group

If you run into problems, have feature suggestions, or want to exchange productivity tool ideas, feel free to join the WeChat group: [Join the WeChat group](https://docs.qq.com/doc/DRnBVSGREUm1qUkJZ)

## 📄 License

MIT
