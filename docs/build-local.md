# Build local — LaserCAD R14

Documentação rápida para rodar e empacotar o LaserCAD localmente após a migração para TypeScript + Tauri 2.

## Pré-requisitos

- **Node 22+** e **npm 10+**
- **Rust stable** (`rustup default stable`)
- Dependências de sistema:

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

- Visual Studio Build Tools com workload "Desktop development with C++"
- WebView2 (já vem no Windows 11; Windows 10 baixa via Tauri)

## Fluxos comuns

```bash
# Frontend isolado (dev)
npm run dev                # http://localhost:1420

# Frontend isolado (build)
npm run build              # dist/

# Tipagem + lint + testes
npm run typecheck
npm run lint
npm test

# App nativo (dev)
npm run tauri:dev          # janela nativa com HMR

# App nativo (release)
npm run tauri:build        # binários em src-tauri/target/release/bundle/
```

## Artefatos produzidos

| Plataforma | Caminho                                                                    |
| ---------- | -------------------------------------------------------------------------- |
| Linux      | `src-tauri/target/release/bundle/appimage/*.AppImage` e `bundle/deb/*.deb` |
| macOS      | `src-tauri/target/release/bundle/dmg/*.dmg` e `bundle/macos/*.app`         |
| Windows    | `src-tauri/target/release/bundle/msi/*.msi` e `bundle/nsis/*.exe`          |

## CI

- `.github/workflows/ci.yml` roda lint, typecheck, test e build a cada push/PR.
- `.github/workflows/release.yml` dispara em tags `v*` e empacota binários para Linux/Windows/macOS (arm64+x64).
