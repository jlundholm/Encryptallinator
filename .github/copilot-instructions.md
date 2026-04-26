# Copilot instructions for Encryptallinator

## Build, test, and lint

- Install dependencies: `npm install`
- Frontend dev server: `npm run dev`
- Desktop dev app: `npm run tauri dev`
- Frontend production build: `npm run build`
- Desktop production build: `npm run tauri build`
- Cross-platform CI compile check: `.github/workflows/cross-platform-build.yml` runs `npm run tauri build -- --no-bundle` on macOS and Linux
- Rust test suite: `cargo test --manifest-path src-tauri\Cargo.toml`
- Single Rust test: `cargo test --manifest-path src-tauri\Cargo.toml <test_name>`

There is currently **no dedicated lint script** in `package.json`. Use the existing TypeScript compile step in `npm run build` and the Rust test suite instead of inventing a new lint toolchain.

## High-level architecture

- The desktop app is a **Tauri v2** shell with a **vanilla TypeScript** frontend in `src\` and a **Rust backend** in `src-tauri\src\`.
- The frontend stays intentionally thin: it manages UI state, invokes native dialogs, and calls a single Rust command to encrypt or decrypt the selected path.
- The Rust side owns the application logic:
  - password-based key derivation with **Argon2id**
  - authenticated encryption with **AES-256-GCM**
  - the versioned encrypted payload format
  - filesystem reads/writes
  - folder archiving and extraction
- Folders are not encrypted file-by-file in place. The backend archives a folder into one payload before encryption, then restores that archive during decryption.

## Key conventions

- Keep the UI as a **single-screen flow**: password, select target, encrypt/decrypt toggle, action button, and status output.
- Preserve the product styling choices already established in the app:
  - primary surface color: `#333C43`
  - accent/button color: `#1670BF`
- Default behavior should remain **non-destructive**: create a sibling encrypted/decrypted output instead of overwriting the original source.
- When changing the crypto flow, keep the frontend simple and push logic into Rust rather than duplicating file or encryption rules in TypeScript.
- macOS and Linux desktop builds should be validated on native runners or hosts rather than assumed from a Windows-only build.
