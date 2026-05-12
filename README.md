# LaserCAD R14

Micro-CAD 2D no navegador. Sem servidor, sem build, sem framework: apenas HTML, CSS e JavaScript clássico carregando direto via `file://`.

## Uso

1. Clone o repositório (ou copie a pasta).
2. **Duplo-clique em `index.html`** — o app abre no navegador.
3. Use a toolbar à esquerda, a command line no rodapé ou os atalhos de teclado.

Não há dependências externas. Não rode `npm install` — não há `package.json`.

## O que está pronto (MVP)

- Viewport SVG infinito em milímetros, com pan/zoom (botão do meio ou `Espaço`+drag, scroll para zoom).
- Grid 1mm/10mm + eixos X/Y.
- Ferramentas de desenho: **Line**, **Polyline**, **Rectangle**, **Circle**, **Arc**.
- Edição: **Select**, **Move**, **Trim**, **Extend**, **Delete**.
- Snaps: **endpoint**, **midpoint**, **center**, **intersection** (marcadores coloridos, toggle com F3).
- Ortho lock com **Shift** ou toggle F8.
- Undo/Redo (`Ctrl+Z` / `Ctrl+Y`), até 200 níveis.
- Entrada por command line: comandos (`line`, `circle`…), coordenadas absolutas `124.5, 87.3` e relativas `@50, 0`, distância pura para tools com preview.
- Export **plain SVG** compatível com LaserGRBL: presets **cut** (vermelho), **mark** (azul), **engrave** (verde). `Ctrl+S` salva o preset `cut`.
- Autosave em `localStorage` (restaurado no boot).

## Atalhos

Ver [`docs/atalhos.md`](docs/atalhos.md). Resumo:

| Tecla | Ação |
|---|---|
| `L P R C A` | Line / Polyline / Rect / Circle / Arc |
| `S M T E` | Select / Move / Trim / Extend |
| `Del` | Apaga seleção |
| `Esc` | Cancela ferramenta |
| `F3 F7 F8` | Toggle Snap / Grid / Ortho |
| `Ctrl+Z / Ctrl+Y` | Undo / Redo |
| `Ctrl+S` | Salvar SVG (preset cut) |

## Arquitetura

- **Stack**: vanilla JS (ES2015+), SVG nativo, CSS Grid, IIFE com namespace `window.LaserCAD.*`.
- **Sem ES modules**: o app abre via `file://`; ver `docs/adr/0001-arquitetura-base.md`.
- **Ordem de scripts**: definida em `specs/_conventions/namespace.md` e replicada em `index.html`.
- **Sem mutação direta do state**: tudo passa por `LaserCAD.app.state.set*` ou por `tools.toolManager.commit(cmd)`; ver `specs/_conventions/state-contract.md`.

## Estrutura

```
index.html              — shell + ordem de scripts
assets/css/             — reset, app, theme (paleta laser 450nm)
src/core/               — kernel puro (geometria + documento)
src/render/             — pipeline SVG (camera, grid, overlays, entity-renderers)
src/tools/              — máquina de ferramentas + 7 tools
src/ui/                 — menubar, toolbar, command-line, statusbar, dialogs
src/app/                — state, config, shortcuts, event-bus, bootstrap
src/io/                 — export-svg, file-download, autosave
specs/                  — specs por módulo (produzidas na Sprint 1)
docs/adr/               — decisões arquiteturais
docs/examples/          — SVGs de exemplo
docs/release/           — release notes
```

## Inspeção via DevTools

Tudo é inspecionável em `window.LaserCAD`:

```js
LaserCAD.app.state                              // estado completo
LaserCAD.core.geometry.vec2.add({x:1,y:2}, {x:3,y:4})  // → {x:4,y:6}
LaserCAD.render.camera.get()                    // câmera atual
LaserCAD.bus.on('camera:changed', p => console.log(p))
LaserCAD.io.exportSvg.serialize(LaserCAD.app.state, {preset:'cut'})
```

## Compatibilidade com LaserGRBL

O exportador segue o checklist da `plan.md` §"Exportação SVG para LaserGRBL":

- `xmlns` no `<svg>` raiz
- `width`/`height` em **mm**, `viewBox` em coordenadas de mundo
- `fill="none"` forçado; sem texto vivo; sem `filter`/`mask`/`clipPath`
- 1 grupo por preset (`<g id="CUT" stroke="#ff0000" stroke-width="0.1">`)
- Arcos como `<path d="A">`
