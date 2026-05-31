# noscreen

> A desktop privacy overlay that stays visible to you but invisible to screen sharing and recording.

**noscreen** lets you keep sensitive information, productivity tools, and AI assistants visible on your screen while remaining completely hidden from Zoom, Microsoft Teams, Google Meet, OBS Studio, Discord, Loom, and OS-level screenshot tools.

Built with Rust + Tauri 2 + Svelte 5 + Tailwind CSS.

---

## Why noscreen?

When you share your screen, everything visible gets captured — including notes, cheat sheets, password managers, AI chats, and any other tool you might want to reference privately. noscreen creates protected windows that the operating system explicitly excludes from screen capture, so you can:

- Reference notes during a live presentation without your audience seeing them
- Use an AI assistant during interviews, meetings, or pair-programming
- Keep technical documentation open while screen sharing
- Display personal information without it leaking to recordings

The protection is enforced at the OS level (Windows `WDA_EXCLUDEFROMCAPTURE`, macOS `NSWindowSharingNone`), not via post-processing or window cropping. Captured frames simply do not contain noscreen's pixels.

## Platform support

| Platform                | Status            | Notes                                                                                |
| ----------------------- | ----------------- | ------------------------------------------------------------------------------------ |
| Windows 10 (2004+) / 11 | ✅ Primary target | Full feature set                                                                     |
| macOS 12+               | ⚠️ Partial        | Core protection works; Windows-only features (ghost typing, WASAPI loopback) stubbed |
| Linux                   | ❌ Not planned    | Wayland/X11 do not expose equivalent protection APIs                                 |

## Features

### Core protection

- **Capture-invisible windows** — every app window is registered with `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on Windows and `setContentProtected(true)` on macOS
- **Always-on-top floating overlay** — frameless, transparent, sits above other windows without stealing focus
- **Toggle protection at runtime** — disable the shield temporarily without restarting

### Window management

- **Global hotkeys** — show/hide instantly from any app (`Ctrl+Shift+Space` by default)
- **Click-through mode** — make the overlay pass mouse events through to whatever is behind it
- **Show-without-activate** — bringing the window forward never takes focus from the underlying app (no `blur` events fire in the browser behind it)
- **System tray** — quick access to show/hide, settings, and quit
- **Multi-window architecture** — separate windows for the main hub, settings, and embedded AI services

### Embedded AI services

- **ChatGPT, Claude, and Google Translate** as embedded webviews — all protected from capture
- **Anti-bot fingerprint patch** — overrides `navigator.webdriver`, injects `window.chrome`, etc. so Cloudflare doesn't block the embedded browser
- **Native AI chat** — built-in chat panel that talks to any OpenAI-compatible endpoint (OpenAI, Ollama, Groq, LM Studio, etc.)
- **Text injection** — push text from one service into another's input box programmatically

### Ghost typing (Windows-only)

A low-level keyboard hook that captures every keystroke globally and routes it to noscreen, so you can type into the overlay while the browser behind it stays focused. The browser never sees the keys — no blur event, no visibility change — and noscreen never has to steal focus.

Triggered with `Ctrl+Alt+G`. Useful for typing prompts into an AI without the meeting app noticing you switched windows.

### Speech-to-text (Windows-only)

Continuous dictation via Windows built-in Speech Recognition (no API key required). Transcribes whatever is set as your default recording device, including system loopback via Stereo Mix.

### Conversation history

- SQLite-backed (`rusqlite` with bundled SQLite)
- Stores conversations and messages per profile
- Survives restarts; on-disk only, never sent to remote services

### Customization

- Opacity slider (live)
- Light/dark theme
- Configurable AI endpoint URL, API key, and model
- Override URLs for embedded webviews (e.g. point ChatGPT to a self-hosted alternative)

### In development

- **Cluely-style copilot mode** — live audio capture from system loopback, real-time transcription, and proactive AI suggestions in a floating answer card. Design spec at [docs/superpowers/specs/2026-05-31-copilot-cluely-mode-design.md](docs/superpowers/specs/2026-05-31-copilot-cluely-mode-design.md).

## Quick start

### Prerequisites

- Node.js 20+ and npm (or Bun; `bun.lock` is present)
- Rust toolchain (`rustup` → stable)
- Platform-specific Tauri prerequisites: see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
  - Windows: WebView2 (ships with Win10 2004+) + Microsoft C++ Build Tools
  - macOS: Xcode Command Line Tools

### Install & run

```bash
npm install
npm run tauri dev
```

### Build a production installer

```bash
npm run tauri build
```

Output lands in `src-tauri/target/release/bundle/` (`.msi` and `.exe` on Windows, `.dmg` and `.app` on macOS).

### Tests

```bash
npm test           # Vitest — frontend unit tests
cd src-tauri && cargo test   # Rust unit tests
```

### Type checking

```bash
npm run check      # svelte-check + tsc
```

## Architecture overview

```
┌────────────────────────────────────────────────────────────────────┐
│ Frontend — Svelte 5 (Runes) + SvelteKit (static adapter) + Tailwind│
│   src/routes/         pages: /, /control-bar, /settings            │
│   src/lib/components/ Hub, HotkeyRecorder, MicButton, …            │
│   src/lib/components/hub/   Dashboard, NativeChat, Sidebar, …      │
│   src/lib/stores/     reactive state (Svelte 5 $state runes)       │
└────────────────────────────────────────────────────────────────────┘
                              ▲ Tauri commands / events
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│ Rust backend — src-tauri/src/                                      │
│   lib.rs           setup, window creation, hotkey registration     │
│   commands.rs      all #[tauri::command]s                          │
│   protection.rs    WDA_EXCLUDEFROMCAPTURE, WS_EX_NOACTIVATE        │
│   ghost_typing.rs  WH_KEYBOARD_LL low-level keyboard hook          │
│   stt.rs           Windows Media.SpeechRecognition (WinRT)         │
│   db.rs            rusqlite — profile, conversations, messages     │
│   config.rs        JSON config persistence                         │
│   tray.rs          system tray + menu                              │
└────────────────────────────────────────────────────────────────────┘
```

## How the protection works

**Windows** — Each window's HWND is passed to `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)`. The Desktop Window Manager omits these windows when composing the frame that screen-capture APIs receive. Works against:

- `BitBlt` / `GetWindowDC`-based capture (Snipping Tool, older OBS)
- Desktop Duplication API (modern OBS, NDI, most meeting apps)
- Windows.Graphics.Capture API (newer Teams, Snip & Sketch)

**macOS** — Tauri's `content_protected(true)` sets `NSWindow.sharingType = .none`. Equivalent behavior in `CGWindowListCreateImage` and `SCStream`.

**Linux** — Not supported. Wayland's `wlr-screencopy` and X11's `XGetImage` do not expose a per-window opt-out.

## Default keyboard shortcuts

| Shortcut            | Action                     |
| ------------------- | -------------------------- |
| `Ctrl+Shift+Space`  | Show / hide the overlay    |
| `Ctrl+Alt+G`        | Toggle ghost typing mode   |
| `Ctrl+1` … `Ctrl+5` | Navigate between hub pages |
| `Ctrl+,`            | Open settings              |

(Configurable hotkey UI is on the roadmap.)

## Limitations & honest disclaimers

- **Physical cameras still see your screen.** If someone points a phone at your monitor, noscreen cannot help.
- **Screen-mirroring hardware (HDMI splitters, KVMs) bypass OS protection** — those see the raw display output.
- **Some legacy capture tools** that use undocumented kernel APIs may bypass `WDA_EXCLUDEFROMCAPTURE`. Modern tools (Zoom, Teams, Meet, OBS) respect it.
- **Test before you trust it.** Try sharing your screen in a test meeting with noscreen open before relying on it for anything important.
- **Use responsibly.** This is a privacy tool, not a tool for academic dishonesty, contractual deception, or anything that violates the rules of the context you're in.

## License

MIT — see [package.json](package.json).

## Recommended IDE setup

[VS Code](https://code.visualstudio.com/) with these extensions:

- [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Tailwind CSS IntelliSense](https://marketplace.visualstudio.com/items?itemName=bradlc.vscode-tailwindcss)
