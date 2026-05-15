# LaserCAD R14

CAD 2D para corte a laser, compatível com LaserGRBL. Disponível como app web e como executável nativo (Linux, Windows, macOS) via Tauri 2.

## Uso

### Como executável nativo (recomendado para usuários finais)

Baixe o instalador da sua plataforma na [página de releases](../../releases). Disponível para Linux (`.AppImage`, `.deb`), macOS (`.dmg`) e Windows (`.msi`, `.exe`).

### Como app web (para desenvolvimento)

```bash
npm install
npm run dev      # http://localhost:1420
```

Veja [`docs/build-local.md`](docs/build-local.md) para construir binários localmente, incluindo dependências de sistema por SO.

## O que está pronto

- Viewport SVG infinito em milímetros, com pan/zoom (botão do meio ou `Espaço`+drag, scroll para zoom).
- Grid 1mm/10mm + eixos X/Y.
- Ferramentas de desenho: **Line**, **Polyline**, **Rectangle**, **Circle**, **Arc**.
- Edição: **Select**, **Move**, **Trim**, **Extend**, **Delete**.
- Snaps: **endpoint**, **midpoint**, **center**, **intersection** (toggle com F3).
- Ortho lock com **Shift** ou toggle F8.
- Undo/Redo (`Ctrl+Z` / `Ctrl+Y`), até 200 níveis.
- Entrada por command line: comandos (`line`, `circle`…), coordenadas absolutas `124.5, 87.3` e relativas `@50, 0`, distância pura para tools com preview.
- Export **plain SVG** compatível com LaserGRBL: presets **cut** (vermelho), **mark** (azul), **engrave** (verde). `Ctrl+S` salva o preset `cut`.
- Autosave em `localStorage` (versão web) ou `tauri-plugin-store` (versão nativa).

## Atalhos

Ver [`docs/atalhos.md`](docs/atalhos.md). Resumo:

| Tecla             | Ação                                  |
| ----------------- | ------------------------------------- |
| `L P R C A`       | Line / Polyline / Rect / Circle / Arc |
| `S M T E`         | Select / Move / Trim / Extend         |
| `Del`             | Apaga seleção                         |
| `Esc`             | Cancela ferramenta                    |
| `F3 F7 F8`        | Toggle Snap / Grid / Ortho            |
| `Ctrl+Z / Ctrl+Y` | Undo / Redo                           |
| `Ctrl+S`          | Salvar SVG (preset cut)               |

## Arquitetura

- **Frontend**: TypeScript (ES2022 modules) + SVG nativo + CSS Grid.
- **Build**: Vite 5.
- **Shell nativo**: Tauri 2 (Rust + WebView do sistema).
- **Testes**: Vitest (jsdom).
- **Tipos centrais**: `src/core/types.ts` define `Vec2`, `Entity`, `AppState`, `Command`, `Tool`.
- **State**: mutação só via `state.set*` ou `toolManager.commit(cmd)`; ver `specs/_conventions/state-contract.md`.
- **Event bus**: lista canônica de eventos em `src/app/event-bus.ts`.

## Estrutura

```
index.html                — shell HTML, entry point único
assets/css/               — reset, app, theme (paleta laser 450nm)
src/core/                 — kernel puro (geometria + documento)
src/core/types.ts         — tipos centrais (Vec2, Entity, AppState…)
src/render/               — pipeline SVG (camera, grid, overlays, entity-renderers)
src/tools/                — máquina de ferramentas + 9 tools
src/ui/                   — menubar, toolbar, command-line, statusbar, dialogs
src/app/                  — state, config, shortcuts, event-bus, bootstrap
src/io/                   — export-svg, file-download, autosave
src/tauri-bridge.ts       — ponte runtime: detecta Tauri vs. browser puro
src-tauri/                — shell Rust + tauri.conf.json
specs/                    — specs por módulo
docs/adr/                 — decisões arquiteturais
docs/build-local.md       — como construir localmente
```

## Scripts

```bash
npm run dev               # vite dev server (web)
npm run build             # vite build → dist/
npm run typecheck         # tsc --noEmit
npm run lint              # eslint
npm run format            # prettier --write
npm test                  # vitest
npm run tauri:dev         # janela nativa com HMR (requer Rust + deps)
npm run tauri:build       # binários nativos (requer Rust + deps)
```

## Compatibilidade com LaserGRBL

O exportador segue o checklist da `plan.md` §"Exportação SVG para LaserGRBL":

- `xmlns` no `<svg>` raiz
- `width`/`height` em **mm**, `viewBox` em coordenadas de mundo
- `fill="none"` forçado; sem texto vivo; sem `filter`/`mask`/`clipPath`
- 1 grupo por preset (`<g id="CUT" stroke="#ff0000" stroke-width="0.1">`)
- Arcos como `<path d="A">`

## Contribuição

Stack acessível para qualquer dev web (TypeScript + Vite). Para empacotar como executável é preciso Rust stable + dependências de sistema (ver `docs/build-local.md`).
