# Local build — LaserCAD R14

Quick reference for running and packaging LaserCAD locally after the migration to TypeScript + Tauri 2.

## Prerequisites

- **Node 22+** and **npm 10+**
- **Rust stable** (`rustup default stable`)
- System dependencies:

### Fedora 43+

```bash
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libsoup3-devel \
  javascriptcoregtk4.1-devel \
  librsvg2-devel \
  openssl-devel \
  patchelf
```

### Ubuntu/Debian

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev \
  patchelf
```

### macOS

```bash
xcode-select --install
```

### Windows

- Visual Studio Build Tools with the "Desktop development with C++" workload
- WebView2 (ships with Windows 11; Windows 10 downloads it via Tauri)

## Common workflows

```bash
# Frontend only (dev)
npm run dev                # http://localhost:1420

# Frontend only (build)
npm run build              # dist/

# Typecheck + lint + tests
npm run typecheck
npm run lint
npm test

# Native app (dev)
npm run tauri:dev          # native window with HMR

# Native app (release)
npm run tauri:build        # binaries in src-tauri/target/release/bundle/
```

## Produced artifacts

| Platform | Path                                                                          |
| -------- | ----------------------------------------------------------------------------- |
| Linux    | `src-tauri/target/release/bundle/appimage/*.AppImage` and `bundle/deb/*.deb`  |
| macOS    | `src-tauri/target/release/bundle/dmg/*.dmg` and `bundle/macos/*.app`          |
| Windows  | `src-tauri/target/release/bundle/msi/*.msi` and `bundle/nsis/*.exe`           |

## CI

- `.github/workflows/ci.yml` runs lint, typecheck, test, and build on every push/PR.
- `.github/workflows/release.yml` triggers on `v*` tags and packages binaries for Linux/Windows/macOS (arm64+x64).
