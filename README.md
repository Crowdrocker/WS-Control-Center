# WS Control Center

A local-first workstation command deck for Linux gaming and creator workflows. The Tauri v2 desktop app uses a dark industrial interface, with the current functional modules and their safety boundaries described below.

## Development

Requirements: Node.js, npm, Rust, and the platform dependencies required by Tauri.

```sh
npm install
npm run tauri dev
```

Create a production frontend build with `npm run build`, or package the desktop app with `npm run tauri build`.

## Current scope

The Steam library module can scan standard Linux Steam locations and their configured libraries when requested. It reads installed app manifests only. A game can be launched only after a separate confirmation; the app passes its validated numeric Steam ID to the desktop's `steam://` URI handler and does not invoke a shell or modify game files.

The app launcher stores user-added executable paths in the app's local configuration directory. It validates each path and only starts an app after an explicit confirmation, invoking the selected executable directly without a shell or custom arguments.

The MO2 / Proton module provides read-only checks for Arch Linux, Steam libraries, Proton tools, Protontricks, Wine, and an optional `ModOrganizer.exe` path. It shows manual setup notes but does not install packages, run commands, or modify prefixes.

The Ollama assistant connects on request to the local Ollama API at `127.0.0.1:11434`, lists available models, and keeps chat history in memory only. It does not send microphone audio, persist conversations, or perform system actions.

The dotfile module backs up only individually selected regular files inside the user's home directory (8 MB per file, 24 MB total). It requires the GitHub CLI to be installed and authenticated, previews the destination before writing, and creates a single commit under `dotfiles/` in an existing repository. Public repositories require an additional explicit confirmation. File contents are not scanned for secrets.

The OBS module connects to the local OBS WebSocket server (port 4455 by default) and reads the OBS version and scene list. An optional WebSocket password is held in memory only. This module does not switch scenes or control recording/streaming.

The System Tools module reads local Linux distribution, kernel, CPU, memory, and uptime information when requested. It does not access the network, run package manager commands, or change system settings.

Run the focused Rust tests with `cargo test --manifest-path src-tauri/Cargo.toml`.