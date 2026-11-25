# KeyCraft — Architecture & Security Invariants

This rule file governs all code written in KeyCraft (`crates/core-engine`, `crates/platform`, `tools/dict-builder`, `src-tauri`, `ui`). Every subagent and contributor MUST enforce these invariants without exception.

## 1. Zero AI, Zero Network, 100% Offline
- **No AI / ML / Neural Networks:** Zero LLMs, zero speech-to-text / Whisper, zero model weight files, zero inference runtimes. All language detection, layout conversion, typo fixing, and text expansion MUST be 100% deterministic.
- **No Network Calls or Telemetry:**
  - Zero HTTP/WebSocket/network client crates in `Cargo.toml` / `Cargo.lock` (`reqwest`, `hyper`, `ureq`, `surf`, `isahc`, `tokio-tungstenite`, etc. are strictly forbidden).
  - `src-tauri/tauri.conf.json` MUST enforce strict CSP with `connect-src 'none'` and zero external origins (no CDNs, no Google Fonts, no external scripts or images).
  - Zero analytics, telemetry, crash reporting, or auto-update checks.

## 2. Crate Boundaries & Clean Cross-Platform Architecture
- `crates/core-engine`:
  - **100% platform-agnostic pure Rust.** Zero OS-specific crates (`windows`, `winapi`, `objc2`, `x11rb`, etc.) allowed in `crates/core-engine/Cargo.toml`.
  - Embedded offline dictionary assets (`ru_freq.fst`, `en_freq.fst`, `ru_ngrams.bin`, `en_ngrams.bin`) loaded via `include_bytes!` — zero runtime downloads, zero required disk reads for base dictionaries.
  - All per-keystroke processing MUST complete in **< 2 ms** (target: < 0.5 ms).
- `crates/platform`:
  - Defines clean cross-platform traits (`PlatformBackend`, `FocusInspector`, `TextInjector`, `ScreenCapturer`, `CaretBadgeOverlay`, `PinnedImageOverlay`, `ClipboardAccess`, `AutostartManager`).
  - Full production implementation for **Windows 11** (`src/windows/`) using the official `windows` crate (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`, `SendInput`, `UIAutomation`, `SetWinEventHook`, DXGI Desktop Duplication + GDI BitBlt fallback, Win32 layered windows).
  - Typed, compile-clean stubs for `src/macos/` and `src/linux/` returning `PlatformError::UnsupportedPlatform` — **never** `todo!()` or `unimplemented!()`.
- `src-tauri`:
  - Background Rust daemon, dynamic system tray icon (`RU` / `EN` / Pause), local JSON storage (`%APPDATA%\KeyCraft`), custom `keycraft-capture://` URI scheme (in-memory PNG/BGRA only), and lazy WebView2 window lifecycle.

## 3. Zero-Keylogger Security & Privacy
- **RAM-Only Word Buffer:** Maximum 64 characters. Immediately cleared and overwritten (`zeroize`) on space/delimiter, mouse click, window focus change, navigation/modifier keys, or pause.
- **Never Persist Typed Text:** User keystrokes and word buffer contents MUST NEVER be written to log files, stdout/stderr in release builds, disk, or crash dumps.
- **Password Field Protection:** `UIAutomation` (`UIA_IsPasswordPropertyId` + secure input heuristics) MUST run on a dedicated COM worker thread and update an atomic/cached `is_password_field` flag. While `is_password_field == true`, the word buffer is immediately zeroized and all keyboard processing is bypassed.
- **No Hook Recursion or Starvation:**
  - `WH_KEYBOARD_LL` callback MUST never block (> 1 ms), never call COM/UIAutomation, and immediately ignore events tagged with `dwExtraInfo == KEYCRAFT_MAGIC_EXTRA_INFO` (`0x4B43_5246_0000_0001`).
  - All synthetic inputs sent via `SendInput` MUST set `dwExtraInfo = KEYCRAFT_MAGIC_EXTRA_INFO`.

## 4. Resource Budget & Quality Gates
- **Idle RAM Budget:** `<= 15–20 MB` when settings window is closed; `0.0%` idle CPU.
- **Destroy WebView2 on Close:** Closing the settings or screenshot overlay window MUST call `window.destroy()` (not `hide()`). Startup launches with `windows: []` (tray-only).
- **Native Lightweight Overlays:** The 700 ms caret layout badge and pinned floating screenshots MUST use pure Win32 layered windows (`WS_EX_LAYERED`) rather than spawning WebView2 instances.
- **Zero Stubs / Zero Warnings:** Never commit `TODO`, `unimplemented!()`, `todo!()`, or mock production logic. Every phase MUST pass `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
