# Convenção de namespace global — `window.LaserCAD`

Este documento define o vocabulário, as regras de declaração e a ordem de carregamento dos scripts clássicos do LaserCAD R14. Decorre diretamente de `docs/adr/0001-arquitetura-base.md` (decisão 3: scripts clássicos sob `window.LaserCAD`).

> **Regra mestra:** nenhum arquivo do projeto declara variáveis globais soltas. Tudo entra por `window.LaserCAD.<sub-namespace>.<nome>`.

---

## 1. Mapa de sub-namespaces

O estado global é uma única árvore. Cada sub-namespace corresponde a uma camada arquitetural (plan.md L17–30, L144–212).

```js
window.LaserCAD = {
  core: {
    geometry: {
      /*…*/
    },
    document: {
      /*…*/
    },
  },
  render: { camera, svgRoot, grid, entityRenderers, overlays },
  tools: { toolManager, selectTool /*…*/ },
  ui: { toolbar, commandLine, statusbar, menubar, dialogs },
  io: {}, // vazio na Sprint 1
  app: { state, config, shortcuts, bootstrap },
  bus: { on, off, emit },
};
```

### 1.1 Responsabilidades por sub-namespace

| Sub-namespace   | Conteúdo previsto                                                                  | Pureza                                                           | Origem na `plan.md` |
| --------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------- |
| `core.geometry` | `epsilon`, `vec2`, `line`, `circle`, `arc`, `project`, futuros `intersect`, `snap` | Sem DOM, sem `window`, sem eventos                               | L161–169, L222      |
| `core.document` | `schema`, `validators`, `commands`, `history`                                      | Sem DOM                                                          | L170–174            |
| `render`        | `camera`, `svgRoot`, `grid`, `entityRenderers`, `overlays`                         | Toca o DOM SVG; lê o `state`; emite eventos                      | L175–180            |
| `tools`         | `toolManager`, `selectTool` (Sprint 1); demais nas sprints seguintes               | Máquina de estados; sem DOM diretamente                          | L181–190, L223      |
| `ui`            | `toolbar`, `commandLine`, `statusbar`, `menubar`, `dialogs`                        | Toca o DOM HTML (chrome); emite eventos                          | L196–200            |
| `io`            | Vazio na Sprint 1; futura sede de `export-svg`, `file-download`, `autosave`        | —                                                                | L191–195            |
| `app`           | `state` (singleton), `config`, `shortcuts`, `bootstrap`                            | Mutação do estado é exclusiva de `app.state`                     | L155–159            |
| `bus`           | `on(evt, fn)`, `off(evt, fn)`, `emit(evt, payload)`                                | Lista canônica fechada em `specs/_conventions/state-contract.md` | —                   |

**Sprint 1 (WS-A/B/C):** apenas `core.geometry`, `core.document`, `app.state`, `bus` e a casca mínima de `render`/`tools`/`ui` necessária para abrir o app. `io` permanece vazio.

---

## 2. Padrão de declaração obrigatório (IIFE)

Cada arquivo abre uma IIFE, anexa coisas ao namespace e nada mais. O cabeçalho de bootstrap (`||`) garante que carregar arquivos fora de ordem não estoure imediatamente — a falha real aparece no acesso à propriedade ausente, durante o teste manual no DevTools ou no `bootstrap.js`.

```js
// src/core/geometry/vec2.js
(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  /** @typedef {{x:number, y:number}} Vec2 */

  ns.vec2 = {
    /**
     * Soma dois vetores 2D.
     * @param {Vec2} a
     * @param {Vec2} b
     * @returns {Vec2}
     */
    add(a, b) {
      /* ... */
    },

    // ... demais funções públicas (sub, scale, dot, len, normalize, ...)
  };
})(
  (window.LaserCAD = window.LaserCAD || {
    core: { geometry: {}, document: {} },
    render: {},
    tools: {},
    ui: {},
    io: {},
    app: {},
    bus: {},
  }),
);
```

### 2.1 Regras do padrão

1. **`'use strict'`** sempre, na primeira linha da IIFE.
2. **Parâmetro formal `LaserCAD`** na assinatura da IIFE — nunca usar `window.LaserCAD` diretamente dentro do corpo.
3. **O default literal de namespace** (segundo argumento de invocação) é **idêntico em todos os arquivos** — copiar e colar o template. Isso garante que qualquer arquivo possa ser o primeiro a executar sem quebrar.
4. **Atribuir apenas o que o arquivo oferece.** Um arquivo `vec2.js` cria `ns.vec2`; nunca toca `ns.line` ou `LaserCAD.render.*`.
5. **Sem variáveis globais soltas.** Constantes locais à IIFE são permitidas (`const TWO_PI = Math.PI * 2;`), nunca anexadas a `window`.
6. **JSDoc com `@typedef`** para entidades, payloads e formas do estado (plan.md L224). Tipos vivem ao lado da implementação, prefixados pelo arquivo que os possui (ex.: `Vec2`, `LineEntity`, `CameraState`).
7. **Pureza de `core/`:** nenhum arquivo em `src/core/` referencia `document`, `window` (além de `window.LaserCAD`), `setTimeout`, eventos, DOM, ou outros sub-namespaces fora de `core` (plan.md L222).
8. **Dependências dentro de `core.geometry`:** acessadas via `ns` capturado no topo da IIFE. Ex.: dentro de `line.js`, `const v = ns.vec2;` no topo da IIFE, então `v.add(...)` no corpo.
9. **Verificação leve em dev:** o bootstrap (`src/app/bootstrap.js`) pode assertar a presença das chaves mínimas (`LaserCAD.core.geometry.vec2`, `LaserCAD.app.state`, `LaserCAD.bus.emit`) e abortar com erro legível se faltar alguma — facilita diagnóstico de ordem quebrada.

### 2.2 Anti-padrões proibidos

```js
// PROIBIDO: variável global solta
var EPS = 1e-6;

// PROIBIDO: tipo "module" — falha em file:// (ADR 0001 §3)
<script type="module" src="./src/main.js"></script>;

// PROIBIDO: anexar coisas em window fora do namespace
window.addLine = function () {
  /* ... */
};

// PROIBIDO: import/export
import { vec2 } from './vec2.js';
export const line = {
  /* ... */
};

// PROIBIDO: mutação do state a partir de fora do app.state
window.LaserCAD.app.state.activeTool = 'line';
```

---

## 3. Ordem canônica dos `<script src="...">` em `index.html`

Carregar das **folhas para as raízes** — quem ninguém depende vai primeiro; quem orquestra o app vai por último. Esta lista é normativa: qualquer novo arquivo precisa ser inserido na posição correta e a tabela atualizada.

|   # | Caminho                           | Sub-namespace que popula   | Depende de                                                            |
| --: | --------------------------------- | -------------------------- | --------------------------------------------------------------------- |
|   1 | `src/core/geometry/epsilon.js`    | `core.geometry.epsilon`    | (nenhuma)                                                             |
|   2 | `src/core/geometry/vec2.js`       | `core.geometry.vec2`       | `epsilon`                                                             |
|   3 | `src/core/geometry/line.js`       | `core.geometry.line`       | `vec2`, `epsilon`                                                     |
|   4 | `src/core/geometry/circle.js`     | `core.geometry.circle`     | `vec2`, `epsilon`                                                     |
|   5 | `src/core/geometry/arc.js`        | `core.geometry.arc`        | `vec2`, `epsilon`                                                     |
|   6 | `src/core/geometry/project.js`    | `core.geometry.project`    | `vec2`, `line`, `circle`, `arc`                                       |
|   7 | `src/core/document/schema.js`     | `core.document.schema`     | (nenhuma — só `@typedef`s)                                            |
|   8 | `src/core/document/validators.js` | `core.document.validators` | `schema`, `core.geometry.*`                                           |
|   9 | `src/core/document/commands.js`   | `core.document.commands`   | `schema`, `validators`                                                |
|  10 | `src/core/document/history.js`    | `core.document.history`    | `commands`                                                            |
|  11 | `src/app/event-bus.js`            | `bus`                      | (nenhuma)                                                             |
|  12 | `src/app/state.js`                | `app.state`                | `bus`, `core.document.schema`                                         |
|  13 | `src/app/config.js`               | `app.config`               | (nenhuma)                                                             |
|  14 | `src/app/shortcuts.js`            | `app.shortcuts`            | `bus`, `app.config`                                                   |
|  15 | `src/render/camera.js`            | `render.camera`            | `app.state`, `bus`, `core.geometry.vec2`                              |
|  16 | `src/render/svg-root.js`          | `render.svgRoot`           | `app.state`, `bus`, `render.camera`                                   |
|  17 | `src/render/grid.js`              | `render.grid`              | `render.svgRoot`, `render.camera`, `bus`                              |
|  18 | `src/render/entity-renderers.js`  | `render.entityRenderers`   | `render.svgRoot`, `core.document.schema`                              |
|  19 | `src/render/overlays.js`          | `render.overlays`          | `render.svgRoot`, `render.camera`, `bus`                              |
|  20 | `src/tools/tool-manager.js`       | `tools.toolManager`        | `app.state`, `bus`, `core.document.commands`, `core.document.history` |
|  21 | `src/tools/select-tool.js`        | `tools.selectTool`         | `tools.toolManager`, `core.geometry.*`                                |
|  22 | `src/ui/toolbar.js`               | `ui.toolbar`               | `bus`, `app.state`                                                    |
|  23 | `src/ui/command-line.js`          | `ui.commandLine`           | `bus`, `app.state`                                                    |
|  24 | `src/ui/statusbar.js`             | `ui.statusbar`             | `bus`, `app.state`                                                    |
|  25 | `src/ui/menubar.js`               | `ui.menubar`               | `bus`, `app.shortcuts`                                                |
|  26 | `src/ui/dialogs.js`               | `ui.dialogs`               | `bus`, `app.state`                                                    |
|  27 | `src/app/bootstrap.js`            | `app.bootstrap`            | todos os anteriores                                                   |
|  28 | `src/main.js`                     | (kick-off)                 | `app.bootstrap`                                                       |

### 3.1 Convenções derivadas

- **Folha primeiro, raiz por último.** O critério de ordenação é "se A usa B em chamadas, A vem depois de B".
- **`event-bus.js` antes de `state.js`.** O estado emite `state:changed` no construtor, então o bus precisa existir.
- **`bootstrap.js` é o penúltimo;** ele cria o `<svg>` raiz, registra subscribers e dispara `app:ready`.
- **`main.js` é o último** e contém apenas a chamada `LaserCAD.app.bootstrap.start()` (ou equivalente) — sem lógica adicional.
- **Folhas dentro do mesmo sub-namespace seguem ordem alfabética** quando não há dependência (ex.: `circle.js` antes de `arc.js` é convenção visual, não funcional — ambos só dependem de `vec2`/`epsilon`).

### 3.2 Adicionando um novo arquivo

1. Identificar dependências reais (quais sub-namespaces o arquivo lê no topo da IIFE).
2. Inserir o `<script src>` em `index.html` **depois** de todas as dependências e **antes** de qualquer dependente.
3. Atualizar a tabela acima.
4. Se a dependência criar ciclo, parar — refatorar antes; ciclos quebram o modelo "folhas → raízes".

---

## 4. Convenção de teste manual no DevTools

A consequência direta de "tudo sob `window.LaserCAD`" é que **todo módulo é inspecionável e testável no console do DevTools**, sem ferramentas adicionais. Esta é uma promessa operacional do projeto, não uma curiosidade.

### 4.1 Exemplos canônicos

```js
// Inspecionar o estado completo
window.LaserCAD.app.state;

// Soma vetorial pura
window.LaserCAD.core.geometry.vec2.add({ x: 1, y: 2 }, { x: 3, y: 4 });
// → {x:4, y:6}

// Mudar de ferramenta via bus (caminho oficial)
window.LaserCAD.bus.emit('tool:request', { toolId: 'line' });

// Inspecionar a câmera atual
window.LaserCAD.render.camera.get();
// → { cx: 0, cy: 0, zoom: 1, viewportW: 1600, viewportH: 900 }

// Assinar um evento ad-hoc para depuração
window.LaserCAD.bus.on('cursor:moved', (p) => console.log(p));
```

### 4.2 Regras

- Toda função pública precisa ser chamável a partir do console com argumentos literais.
- Funções puras (`core.geometry.*`, `core.document.commands`) **retornam** valores; não escrevem no estado.
- Mutações no estado passam por `LaserCAD.app.state.<setter>()` ou por comandos disparados pelo bus — **nunca** atribuição direta de fora.
- Smoke manual ao final de cada PR: abrir `index.html` por duplo-clique, rodar 3–5 das chamadas acima no console, verificar que não há erro.
- Os testes unitários do kernel (plan.md L242–249, Sprint Geometry Core) usam exatamente o mesmo caminho de chamada: `LaserCAD.core.geometry.vec2.add(...)` etc., com `window.LaserCAD` montado em um shim Node/JSDOM.
