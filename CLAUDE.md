# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

LaserCAD R14 é um micro-CAD 2D para corte a laser, compatível com LaserGRBL. O frontend é TypeScript + Vite + SVG nativo; o shell nativo (Linux/macOS/Windows) é Tauri 2 (Rust + WebView do sistema). Documentação de produto está em pt-BR; código e comentários em inglês ou pt-BR conforme arquivo.

## Comandos

```bash
npm install
npm run dev               # vite dev (web) em http://localhost:1420
npm run build             # vite build → dist/
npm run typecheck         # tsc --noEmit
npm run lint              # eslint
npm run format            # prettier --write .
npm test                  # vitest run (jsdom)
npm run test:watch        # vitest watch mode
npm run tauri:dev         # janela nativa com HMR (requer Rust + deps do SO)
npm run tauri:build       # binários nativos em src-tauri/target/release/bundle/
```

Rodar um único teste: `npx vitest run src/core/geometry/vec2.test.ts` (ou `-t "padrão do nome"`). Testes ficam ao lado da implementação (`*.test.ts` em `src/`) ou em `tests/`; ambiente é jsdom (ver `vitest.config.ts` e `tests/setup.ts`).

Pré-requisitos nativos (Rust + libs por SO) estão em `docs/build-local.md`.

## Arquitetura

### Camadas e regra de pureza

```
src/core/        kernel puro: geometria (vec2, line, circle, arc, intersect, snap, project) + documento (schema, validators, commands, history)
src/render/      pipeline SVG: camera, svg-root, grid, bed, overlays, entity-renderers
src/tools/       máquina de ferramentas (tool-manager) + 9 tools (line, polyline, rect, circle, arc, select, move, trim, extend)
src/ui/          chrome HTML: menubar, toolbar, command-line, statusbar, dialogs
src/io/          export-svg, file-download, autosave (localStorage no web, tauri-plugin-store no nativo)
src/app/         state singleton, config, shortcuts, event-bus, bootstrap
src/tauri-bridge.ts  detecta `window.__TAURI_INTERNALS__` e expõe save/open/store via plugins Tauri
src-tauri/       shell Rust + tauri.conf.json
```

- `core/` é **puro**: sem DOM, sem `window` (exceto `core/geometry/project.ts`, a única exceção autorizada para fazer lazy-lookup de `render.camera`, ver `docs/adr/0002-riscos-de-integracao.md` §1).
- `tools/` não toca DOM diretamente; emite via bus e aplica `Command` (do/undo) via `toolManager.commit(cmd)`.
- Conversão mm ↔ pixel vive em `render/camera.ts` e em lugar nenhum mais.

### Unidades e tipos

- **Milímetros são canônicos** em todo o documento, kernel, command line e exportação. Pixel só aparece em `render/camera`. Ângulos: graus na UI, **radianos no kernel/state** (ver `arc.startAngle`/`endAngle`).
- Tipos centrais em `src/core/types.ts`: `Vec2`, `Entity` (`LineEntity | CircleEntity | ArcEntity`), `AppState`, `Command`, `Tool`, `SnapResult`. Importar dali em vez de redefinir.
- `documentBounds` default `128×128 mm`; vira `viewBox` do `<svg>` e `width="...mm" height="...mm"` na exportação.

### Estado e mutação (contrato dura)

`src/app/state.ts` é o único singleton autorizado a mutar `AppState`. Regras (originais de `specs/_conventions/state-contract.md`, ainda vigentes):

- Atribuição externa direta (`state.activeTool = 'line'`) é **proibida**. Use setters: `state.setCamera`, `setViewportSize`, `setCursor`, `setActiveTool`, `setToolState`, `setToggle`, `setCommandInput`, `pushCommandHistory`, `setDocumentBounds`, `setSelection`.
- Mudanças em `entities`/`selection` passam por `state.applyCommand(cmd)` (ou `toolManager.commit(cmd)`), que empilha em `history` para Undo/Redo.
- `entity.id` segue `'e_<n>'`, contador interno em `core/document/commands.ts`.

### Event bus (lista canônica)

`src/app/event-bus.ts` mantém uma **lista fechada** de eventos; emitir/escutar fora dela imprime `console.warn`:

```
app:ready  viewport:resized  camera:changed  cursor:moved
tool:request  tool:armed  tool:cancel
command:submit  command:error  toggle:changed  bounds:changed
```

Adicionar eventos novos requer ADR (ver `docs/adr/`).

### Bootstrap ordem rígida

`src/app/bootstrap.ts` segue exatamente os passos 1–20 documentados no header do arquivo (originais de ADR 0002 §2). Não reordenar sem ADR: várias coisas dependem de `<svg>` montado antes de `getScreenCTM` ser chamado, listeners de pointer só ligam depois do mount, etc.

Hosts DOM esperados em `index.html`: `#menubar-host`, `#toolbar-host`, `#viewport-host`, `#commandline-host`, `#statusbar-host`. `bootstrap.start()` aborta com banner visível se algum faltar.

### Runtime split: web vs Tauri

`src/tauri-bridge.ts` é a única ponte. Em browser puro, todas as funções retornam `null`/`undefined` e o app cai para `localStorage` + download via Blob. Em Tauri, dynamic-import dos plugins (`@tauri-apps/plugin-dialog`, `-fs`, `-store`) faz save/open nativo. **Não importar plugins Tauri estaticamente** — só dentro de funções guardadas por `isTauri()` para manter o bundle web leve.

### Imports e aliases

- TS path alias `@/*` → `src/*` (ver `tsconfig.json` e `vite.config.ts`). Use `@/app/state.js` (com `.js`) em imports — Vite/TS resolvem o `.ts` correspondente.
- ESLint config (`eslint.config.js`) está em flat config; ignora `dist/`, `src-tauri/`, `.playwright-cli/`.

### Exportação SVG (compatibilidade LaserGRBL)

`src/io/export-svg.ts` segue o checklist de `docs/plan.md` (§"Exportação SVG para LaserGRBL"):

- `xmlns` no `<svg>` raiz; `width`/`height` em mm; `viewBox` em coordenadas de mundo
- `fill="none"` forçado; sem texto vivo; sem `filter`/`mask`/`clipPath`
- Um `<g>` por preset com cor: **cut** vermelho, **mark** azul, **engrave** verde; `stroke-width="0.1"` mm
- Arcos como `<path d="A">` (não `<path>` de bézier)

Mudar essas regras quebra import no LaserGRBL — confirmar antes.

## Documentação de referência

- `docs/plan.md` (~34 KB) — plano técnico congelado, fonte de verdade para regras de exportação e convenções.
- `docs/design.md` — design da UI (menubar, toolbar, statusbar, command line, máquina de estados).
- `docs/adr/0001-arquitetura-base.md` — SVG-first, mm, scripts clássicos (substituídos por ES modules após migração, mas regras de pureza/unidades continuam).
- `docs/adr/0002-riscos-de-integracao.md` — lazy lookup core→render, ordem do bootstrap, hosts DOM obrigatórios.
- `specs/_conventions/state-contract.md` — forma exata de `AppState`, setters autorizados, eventos canônicos. **Ler antes de mexer em state/bus.**
- `docs/atalhos.md` — atalhos de teclado (L/P/R/C/A para tools, F3/F7/F8 para toggles, Ctrl+Z/Y/S).
