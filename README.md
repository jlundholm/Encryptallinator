# Encryptallinator

Encryptallinator is a cross-platform Tauri desktop app for encrypting files and folders with a password.

## Stack

- **Desktop shell:** Tauri v2
- **Backend:** Rust
- **Frontend:** Vanilla TypeScript + HTML/CSS
- **Encryption:** AES-256-GCM
- **Password KDF:** Argon2id

Folders are archived into a single payload before encryption. Encrypting or decrypting creates a new sibling output by default rather than overwriting the source.

## Local development

Install dependencies:

```powershell
npm install
```

Run the frontend only:

```powershell
npm run dev
```

Run the desktop app in development:

```powershell
npm run tauri dev
```

Build the frontend bundle:

```powershell
npm run build
```

Run the Rust tests:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml
```

Run a single Rust test:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml wrong_password_is_rejected
```

Build the desktop app bundle:

```powershell
npm run tauri build
```

Build a portable Windows executable without an installer:

```powershell
npm run tauri:build:portable
```

This writes the executable to:

```text
src-tauri\target\release\encryptallinator.exe
```

This portable `.exe` still expects the target Windows machine to have the Microsoft Edge WebView2 runtime installed.

## Building on macOS and Linux

Encryptallinator already uses a cross-platform stack, but Tauri desktop builds should be produced on the **target operating system**:

- build macOS artifacts on **macOS**
- build Linux artifacts on **Linux**

### macOS prerequisites

Install:

- Xcode Command Line Tools: `xcode-select --install`
- Rust via `rustup`
- Node.js LTS

Then run:

```bash
npm install
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

For distributable macOS installers, add Apple signing and notarization later.

### Linux prerequisites

On Debian/Ubuntu-like systems, install the Tauri system packages:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Also install Rust via `rustup` and Node.js LTS, then run:

```bash
npm install
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

### CI

This repository now includes `.github/workflows/cross-platform-build.yml`, which builds the app on `macos-latest` and `ubuntu-latest` GitHub Actions runners using `npm run tauri build -- --no-bundle` to validate native compilation without requiring signing or installer packaging.
