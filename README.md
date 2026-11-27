# WS Control Center

WS Control Center is a local-first Tauri v2 desktop app for Linux gaming and creator workflows. It is designed primarily for Arch Linux and combines a dark industrial interface with local diagnostics, launchers, and integrations.

## Features and safety

| Module | What it does | Safety boundary |
| --- | --- | --- |
| Steam library | Scans standard Linux Steam locations and configured libraries for installed game manifests. | Scanning is read-only. Launching a game requires confirmation and uses its validated Steam app ID; it does not invoke a shell or modify game files. |
| App launcher | Saves user-selected executable paths in the app's local configuration and launches them directly. | Each launch requires confirmation. No shell or custom arguments are used. |
| MO2 / Proton | Checks Arch Linux, Steam libraries, Proton tools, Protontricks, Wine, and an optional `ModOrganizer.exe` path. | Diagnostics and setup notes only. It does not install packages, run setup commands, or modify Proton prefixes. |
| Ollama assistant | Lists local models and chats with Ollama at `127.0.0.1:11434` when requested. | Chat history stays in memory. No microphone audio or system actions are used. |
| Dotfile backup | Previews and backs up individually selected files from the home directory to an existing GitHub repository. | Requires GitHub CLI authentication and explicit confirmation before creating a commit. Files are limited to 8 MB each and 24 MB total. Public repositories require an additional confirmation. File contents are not scanned for secrets. |
| System tools | Displays Linux distribution, kernel, CPU, memory, and uptime information when requested. | Reads local system files only. It does not access the network, run package-manager commands, or change settings. |
| OBS Studio | Reads OBS and WebSocket versions, current program scene, and scene list from the local OBS WebSocket server. | Connects only to `127.0.0.1`; an optional password is held in memory. No scene, recording, or streaming controls are provided. |

## Requirements

The application is Linux-focused. System status and MO2 checks expect Linux, and the MO2 guidance is specific to Arch Linux. Other distributions may work, but are not the primary target.

For development and source builds, install:

- Git
- Node.js LTS and npm
- Rust stable and Cargo (Rustup is recommended)
- Tauri's Linux system dependencies, including WebKitGTK 4.1 and a C/C++ toolchain

See the [Tauri v2 Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux) for current instructions for other distributions.

### Arch Linux prerequisites

The following installs the Tauri development libraries and build tools, plus Node.js, npm, and Rustup:

```sh
sudo pacman -Syu --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  appmenu-gtk-module \
  libappindicator-gtk3 \
  librsvg \
  xdotool \
  nodejs \
  npm \
  rustup
rustup default stable
```

`pacman -Syu` performs a full Arch package upgrade as well as installing the listed dependencies. Review the package manager's proposed changes before confirming. If Rust is already installed with another toolchain manager, keep your existing setup and verify that `rustc` and `cargo` are available instead of installing Rustup again.

### Debian and Ubuntu prerequisites

Install the native Tauri libraries and build tools:

```sh
sudo apt update
sudo apt install \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Also install Node.js LTS with npm and Rust stable with Cargo. See [Node.js](https://nodejs.org/) and [Rustup](https://rustup.rs/) for installation options.

## Install and run from source

Clone the repository, install the locked JavaScript dependencies, and start the Tauri desktop app:

```sh
git clone https://github.com/Crowdrocker/WS-Control-Center.git
cd WS-Control-Center
npm ci
npm run tauri dev
```

The first run downloads and compiles Rust dependencies and may take a few minutes. The native integrations are available in the Tauri desktop window; running only the Vite browser development server (`npm run dev`) does not provide access to Tauri commands.

## Build and install

Build the frontend by itself:

```sh
npm run build
```

Build the desktop application and platform-supported bundles:

```sh
npm run tauri build
```

Tauri writes release binaries under `src-tauri/target/release/` and bundles, when supported by the host and available packaging tools, under `src-tauri/target/release/bundle/`. Inspect that directory for the generated package format. Install a generated `.deb` or `.rpm` using your distribution's package manager; an AppImage can be made executable and launched directly. This project does not provide a package manager integration that installs or updates the host system.

For example, use the command matching a bundle that was actually generated:

```sh
# Debian or Ubuntu
sudo apt install ./src-tauri/target/release/bundle/deb/*.deb

# Fedora or another RPM-based distribution
sudo dnf install ./src-tauri/target/release/bundle/rpm/*.rpm

# AppImage
chmod +x ./src-tauri/target/release/bundle/appimage/*.AppImage
./src-tauri/target/release/bundle/appimage/*.AppImage
```

To build only the native executable without installer bundles:

```sh
npm run tauri build -- --no-bundle
./src-tauri/target/release/ws-control-center
```

## Optional integrations

- **GitHub dotfile backup:** install the [GitHub CLI](https://cli.github.com/) and authenticate it with `gh auth login`. The app backs up selected files only to an existing repository.
- **Ollama assistant:** install and start [Ollama](https://ollama.com/), then download at least one model through Ollama. The app connects to its local API at `127.0.0.1:11434`.
- **OBS Studio:** enable OBS WebSocket Server in **Tools > WebSocket Server Settings**. The app expects the local server, normally on port `4455`.
- **Steam and games:** install Steam and add any extra library folders through Steam's storage settings before scanning.

## Troubleshooting

### Tauri reports missing WebKitGTK, `pkg-config`, or linker libraries

Install the system dependencies for your distribution from the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux), then retry the build. On Arch, make sure `webkit2gtk-4.1` and `base-devel` are installed. If you have just installed Rustup, open a new terminal or reload your shell configuration.

### `npm ci` or the frontend build fails

Check that Node.js and npm are available:

```sh
node --version
npm --version
```

Use the Node.js LTS release, run `npm ci` from the repository root, then retry `npm run build`. `npm ci` uses `package-lock.json` to install the project's pinned dependency tree.

### Rust or Cargo is missing

Verify the Rust toolchain:

```sh
rustc --version
cargo --version
rustup show
```

If Rustup is installed but no default toolchain is configured, select stable with `rustup default stable`.

### Native features are unavailable in a browser

Run `npm run tauri dev` to open the desktop app. Local integrations call Tauri commands and are intentionally unavailable in a regular browser preview.

### Steam games are not detected

Open the Steam library module and run a scan. The scanner reads standard Linux Steam locations and configured library paths from Steam's `libraryfolders.vdf`; it reports installed apps based on their local manifests. Custom library locations must be registered in Steam. No games are launched by scanning.

### MO2 or Proton checks show missing items

The MO2 module reports what it finds and provides manual Arch-oriented setup guidance. It does not install Protontricks, Proton, Wine, or Mod Organizer 2. Check that Steam has scanned the library containing the game and that any supplied `ModOrganizer.exe` path is the full path to an existing file.

### Ollama cannot connect or no models appear

The assistant connects to the local Ollama API at `127.0.0.1:11434`. Make sure Ollama is installed and running, then refresh the model list in the module. If the service is reachable but the list is empty, add a model through your normal Ollama workflow. Chat is not saved by this app.

### OBS connection fails

In OBS, enable the WebSocket server in **Tools > WebSocket Server Settings** and confirm the configured port (default `4455`). Enter the WebSocket password in the module if authentication is enabled. The app connects only to `127.0.0.1`; it does not connect to OBS on another machine.

### Dotfile backup cannot access GitHub

Install and authenticate the GitHub CLI, then check its status with `gh auth status`. The backup module uses an existing repository, previews selected files before writing, and requires confirmation before creating a commit. Review file selections carefully: this feature does not scan file contents for secrets.

### An executable does not launch

For the App Launcher, use the absolute path to an existing executable and confirm the launch prompt. The launcher starts that executable directly without arguments; it does not interpret shell syntax, command aliases, or environment setup commands.

### An AppImage reports a missing FUSE library

Install the FUSE compatibility runtime provided by your distribution, or use a generated `.deb` or `.rpm` bundle if available. AppImage runtime dependencies vary by distribution.

## Development checks

Run the frontend build and Rust tests from the repository root:

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```
