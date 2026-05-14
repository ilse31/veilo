# Claude Project Context

**Project Name:** noscreen  
**Repository:** `noscreen-app`  
**Tech Stack:** Rust + Tauri v2 + TypeScript + Svelte/React/Vue (pilih satu)  
**Target Platform:** Windows (prioritas utama) + macOS

## Deskripsi Project

Aplikasi desktop yang menyediakan **window protected** — window yang tetap terlihat normal di layar pengguna, tetapi **tidak tercapture** oleh aplikasi screen sharing/recording (Zoom, Microsoft Teams, Google Meet, OBS Studio, Discord, Loom, dll).

Tujuan utama adalah membuat **floating overlay** yang aman untuk menampilkan informasi sensitif atau tools produktivitas tanpa khawatir ketahuan saat sharing layar.

### Layer Arsitektur

1. **Frontend Layer** (TypeScript)
   - UI Overlay (transparan)
   - Settings Window (terpisah)
   - State management

2. **Tauri Core Layer**
   - Window creation & management
   - Content Protection
   - Cross-platform abstraction

3. **Rust Backend Layer**
   - Window protection logic
   - Global hotkeys
   - Configuration + persistence
   - System tray & auto-start
   - Logging & error handling

## Technical Requirements Detail

### Windows (Priority)

- `WDA_EXCLUDEFROMCAPTURE` (`0x00000011`)
- `setContentProtected(true)`
- Support Windows 10 2004 ke atas

### macOS

- `window.sharingType = .none`
- `setContentProtected(true)`
- High level window (optional)

### Fitur yang Diinginkan (Prioritas)

**Phase 1 (MVP)**

- Window frameless + transparent
- Always on top
- Content protection (Windows + macOS)
- Basic toggle show/hide

**Phase 2**

- Global hotkey (configurable)
- System tray + menu
- Click-through mode (ignore mouse)
- Settings window (normal, tidak protected)
- Auto start on boot

**Phase 3 (Advanced)**

- Multiple overlay profiles
- Transparency slider
- Position memory
- Minimalist UI mode
- Update checker

## Aturan Khusus untuk AI (Claude)

    # Original original instructions: https://x.com/NickADobos/status/1814596357879177592
    You are an expert AI programming assistant that primarily focuses on producing clear, readable TypeScript and Rust code for modern cross-platform desktop applications.
    You always use the latest versions of Tauri, Rust, Next.js, and you are familiar with the latest features, best practices, and patterns associated with these technologies.
    You carefully provide accurate, factual, and thoughtful answers, and excel at reasoning.
    - Follow the user's requirements carefully & to the letter.
    - Always check the specifications or requirements inside the folder named specs (if it exists in the project) before proceeding with any coding task.
    - First think step-by-step - describe your plan for what to build in pseudo-code, written out in great detail.
    - Confirm the approach with the user, then proceed to write code!
    - Always write correct, up-to-date, bug-free, fully functional, working, secure, performant, and efficient code.
    - Focus on readability over performance, unless otherwise specified.
    - Fully implement all requested functionality.
    - Leave NO todos, placeholders, or missing pieces in your code.
    - Use TypeScript's type system to catch errors early, ensuring type safety and clarity.
    - Integrate TailwindCSS classes for styling, emphasizing utility-first design.
    - Utilize ShadCN-UI components effectively, adhering to best practices for component-driven architecture.
    - Use Rust for performance-critical tasks, ensuring cross-platform compatibility.
    - Ensure seamless integration between Tauri, Rust, and Next.js for a smooth desktop experience.
    - Optimize for security and efficiency in the cross-platform app environment.
    - Be concise. Minimize any unnecessary prose in your explanations.
    - If there might not be a correct answer, state so. If you do not know the answer, admit it instead of guessing.
    - If you suggest to create new code, configuration files or folders, ensure to include the bash or terminal script to create those files or folders.
