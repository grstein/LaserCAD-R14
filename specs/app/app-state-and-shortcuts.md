# app-state-and-shortcuts

Consolida as specs de `src/app/state.js`, `src/app/config.js` e `src/app/shortcuts.js`.

## 1. Responsabilidade

- **`app.state`**: singleton de estado global, único módulo autorizado a mutar `window.LaserCAD.app.state.*`; valida entradas e emite eventos canônicos do bus.
- **`app.config`**: constantes de configuração default (área do documento, viewport inicial, limites, política de auto-fit no boot).
- **`app.shortcuts`**: tradutor de eventos `keydown` em **pedidos** ao bus (`tool:request`, `toggle:changed`, etc.). Nunca muta `state` direto.

## 2. Dependências

- `app.state`: `window.LaserCAD.bus`, `window.LaserCAD.core.document.schema`, `window.LaserCAD.core.document.commands`, `window.LaserCAD.core.document.history`, `window.LaserCAD.core.document.validators`.
- `app.config`: nenhuma.
- `app.shortcuts`: `window.LaserCAD.bus`, `window.LaserCAD.app.config`.
- ordem de carga: `state` posição 12, `config` posição 13, `shortcuts` posição 14 (após `bus`/posição 11 — ver `specs/_conventions/namespace.md` §3).

## 3. API pública

### 3.1 `window.LaserCAD.app.state`

A **forma** do singleton é dada por `specs/_conventions/state-contract.md` §1.1 — **não duplicar aqui**. Esta spec descreve apenas a **API de mutação** (state-contract.md §2.3 dá o esboço; congelado abaixo).

```text
// Setters explícitos — único caminho oficial de mutação
setActiveTool(toolId: string)                  : void
  // valida toolId ∈ {'select','line','polyline','rect','circle','arc',
  //                  'trim','extend','move','delete'}
  // muta state.activeTool e state.toolState = 'armed'
  // emite bus.emit('tool:armed', { toolId })

setToolState(name: 'idle'|'armed'|'preview'|'commit'|'cancel')  : void
  // muta state.toolState

setCamera({ cx, cy, zoom })                     : void
  // valida via validators.isFinitePoint + zoom > 0
  // muta state.camera.cx/cy/zoom (NÃO toca viewportW/H)
  // emite bus.emit('camera:changed', { cx, cy, zoom })

setViewportSize(w, h)                           : void
  // muta state.camera.viewportW e state.camera.viewportH
  // (NÃO emite — o emitter de viewport:resized é o bootstrap/ResizeObserver)

setCursor({ worldX, worldY, screenX, screenY }) : void
  // muta state.cursor.*
  // emite bus.emit('cursor:moved', {...})

setToggle(name: 'snap'|'grid'|'ortho', value: boolean)  : void
  // muta state.toggles[name]
  // emite bus.emit('toggle:changed', { name, value })

setCommandInput(str: string)                    : void
  // muta state.commandInput (sem emitir; UI re-renderiza por outro caminho)

pushCommandHistory(raw: string)                 : void
  // muta state.commandHistory (limita pelo config.maxCommandHistory)

setDocumentBounds({ w, h })                     : void
  // valida w > 0 && h > 0
  // muta state.documentBounds

applyCommand(cmd: Command)                      : void
  // chama cmd.do(state)
  // chama core.document.history.push(state, cmd)
  // NÃO emite evento genérico (state-contract.md §3.2 omite 'state:changed')
```

Contratos gerais:

- **Toda** mutação passa por um destes setters. Atribuição direta (`state.activeTool = 'line'`) é proibida (state-contract.md §2.2).
- Cada setter valida entrada via `core.document.validators.*`. Em caso de inválido, lança `Error` legível.
- Os campos do state são **leitura livre** de qualquer módulo; mutação é exclusiva do `app.state` (state-contract.md §2.1).

### 3.2 `window.LaserCAD.app.config`

Constantes congeladas (`Object.freeze`):

```text
DOCUMENT_DEFAULT_BOUNDS = { w: 128, h: 128 }     // plan.md L346
VIEWPORT_INITIAL        = { w: 1600, h: 900 }    // valor de seed antes do primeiro ResizeObserver
AUTO_FIT_ON_BOOT        = true                    // câmera auto-encaixa documentBounds no viewport ao app:ready
MAX_COMMAND_HISTORY     = 50                      // state-contract.md §1.1
HISTORY_LIMIT           = 200                     // history.md §4 — eco do limite
DEFAULT_TOGGLES         = { snap: true, grid: true, ortho: false }
ZOOM_STEP               = 1.2                     // multiplicador por clique/scroll
ZOOM_MIN, ZOOM_MAX      = 0.01, 1000              // limites de zoom
```

Não há setters em `config`; todas as constantes são leitura. Mudança exige edição direta do arquivo.

### 3.3 `window.LaserCAD.app.shortcuts`

```text
install()                          : void
  // registra um único listener 'keydown' em window.
  // descrito como idempotente (chamadas extras viram no-op).

uninstall()                        : void
  // remove o listener. Útil para testes/teardown.
```

`install()` é chamado por `app.bootstrap`. O listener mapeia teclas para **eventos do bus** (lista canônica em `state-contract.md` §3); nunca muta `state` diretamente.

## 3.4 Tabela de atalhos (exaustiva)

Coberta a partir de `design.md` L131–152 (toolbar), L209–211 (command line) e L242–249 (menubar) e plan.md L223 (estados).

| Tecla                       | Contexto                                           | Evento do bus emitido                                                   | Payload                                                                | Origem               |
| --------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------- | -------------------- |
| `L`                         | viewport (sem focus em input)                      | `tool:request`                                                          | `{ toolId: 'line' }`                                                   | design.md L133, L247 |
| `P`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'polyline' }`                                               | design.md L135, L247 |
| `R`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'rect' }`                                                   | design.md L137, L247 |
| `C`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'circle' }`                                                 | design.md L139, L247 |
| `A`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'arc' }`                                                    | design.md L141, L247 |
| `S`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'select' }`                                                 | design.md L143, L248 |
| `T`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'trim' }`                                                   | design.md L145, L248 |
| `E`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'extend' }`                                                 | design.md L147, L248 |
| `M`                         | viewport                                           | `tool:request`                                                          | `{ toolId: 'move' }`                                                   | design.md L149, L248 |
| `Delete`                    | viewport (com seleção)                             | `tool:request`                                                          | `{ toolId: 'delete' }`                                                 | design.md L151, L248 |
| `F3`                        | global                                             | `toggle:changed`                                                        | `{ name: 'snap', value: !current }`                                    | design.md L231, L246 |
| `F7`                        | global                                             | `toggle:changed`                                                        | `{ name: 'grid', value: !current }`                                    | design.md L232, L246 |
| `F8`                        | global                                             | `toggle:changed`                                                        | `{ name: 'ortho', value: !current }`                                   | design.md L233, L246 |
| `Esc`                       | global                                             | `tool:cancel`                                                           | `{ toolId: state.activeTool }`                                         | design.md L209, L331 |
| `Enter`                     | command line focada **ou** input vazio no viewport | `command:submit`                                                        | `{ raw: commandInput, parsed: <parse>\|null }`                         | design.md L207       |
| `Space`                     | command line focada                                | `command:submit`                                                        | `{ raw: commandInput, parsed: ... }` (alternativa a Enter; compat R14) | design.md L208       |
| `Space`                     | viewport (não focado em input)                     | (pan inicia — sem evento de bus na Sprint 1; ver §7)                    | —                                                                      | design.md L293       |
| `↑`                         | command line, input vazio                          | (consulta `state.commandHistory[idx-1]`; chama `state.setCommandInput`) | —                                                                      | design.md L210       |
| `↓`                         | command line, input vazio                          | (consulta `state.commandHistory[idx+1]`)                                | —                                                                      | design.md L210       |
| qualquer tecla alfanumérica | viewport (sem focus em input)                      | (foco automático na command line; sem evento de bus)                    | —                                                                      | design.md L206       |

### Atalhos da menubar (fora do escopo Sprint 1, listados para referência)

`Ctrl+N`, `Ctrl+O`, `Ctrl+S`, `Ctrl+Z` (undo), `Ctrl+Y` (redo), `Ctrl+X/C/V`, `Ctrl+A`, `+`/`-`, `Z E`/`Z W` (chord), `F1` — são reservados; `shortcuts.install` na Sprint 1 pode **registrar** os handlers vazios ou ignorar; ativação completa entra na Sprint Drawing/Edit. (design.md L244–249).

> **Restrição:** somente os eventos da lista canônica em `state-contract.md` §3 podem ser emitidos. Atalhos que não tenham evento dedicado (como navegação `↑`/`↓` no histórico, foco automático, pan por Space) atuam via setters de `state` (`setCommandInput`, etc.), não via bus.

## 4. Invariantes e tolerâncias

- **`state` é o único mutator.** Atalho/toolbar/menu **emitem pedidos** ao bus; quem decide e muta é `app.state` (state-contract.md §2).
- Validação no setter, antes de mutar. Lançar é preferível a silenciar.
- Auto-fit no boot: ao receber `app:ready` + primeiro `viewport:resized`, a câmera ajusta `zoom`/`cx`/`cy` para enquadrar `documentBounds` com margem. Ativado por `app.config.AUTO_FIT_ON_BOOT`.
- `app.config` é congelado — mutação acidental por outro módulo lança em strict mode.
- Pureza relativa:
  - `state` toca `bus` (emite eventos) — esperado.
  - `config` não toca nada externo.
  - `shortcuts` toca `window` (listener) e `bus` — esperado.
- Ângulos no state em radianos (state-contract.md §1.2; ADR 0001 §2.4). UI converte na borda.

## 5. Exemplos de uso

```js
const state = window.LaserCAD.app.state;
const cfg = window.LaserCAD.app.config;
const bus = window.LaserCAD.bus;

// Trocar ferramenta — caminho oficial via bus → state
bus.emit('tool:request', { toolId: 'line' });
// (tool-manager escuta, valida, chama state.setActiveTool('line'))

// Mutação direta NÃO é permitida — proibida pelo contrato
// state.activeTool = 'line';  // ANTI-PADRÃO

// Acessar config
cfg.DOCUMENT_DEFAULT_BOUNDS; // { w: 128, h: 128 }

// Aplicar comando (camera)
const cmd = window.LaserCAD.core.document.commands.setCamera({ cx: 64, cy: 64, zoom: 2 });
state.applyCommand(cmd);

// Apertar L (sem ter focus em input) emite tool:request
// — comportamento testável via DevTools:
document.dispatchEvent(new KeyboardEvent('keydown', { key: 'l' }));
```

## 6. Critérios de aceitação testáveis manualmente

1. `Object.keys(window.LaserCAD.app.state)` inclui exatamente os 12 campos listados em `state-contract.md` §1.1 (`schemaVersion`, `units`, `documentBounds`, `entities`, `selection`, `camera`, `activeTool`, `toolState`, `cursor`, `toggles`, `commandHistory`, `commandInput`).
2. `window.LaserCAD.app.config.DOCUMENT_DEFAULT_BOUNDS` retorna `{w:128, h:128}` e `Object.isFrozen(window.LaserCAD.app.config)` é `true`.
3. Após `app:ready` em viewport 1600×900, `state.camera.zoom` está ajustado para que `documentBounds` 128×128 caiba com margem (auto-fit) — `zoom` aproximadamente `6` a `7` (1600/128 ≈ 12.5; com margem, ~5–8).
4. **Atalho L:** simular `keydown` com `key:'l'` no `document` (sem focus em input) faz `state.activeTool === 'line'` e `state.toolState === 'armed'`.
5. **Atalho F3:** dois `keydown` com `key:'F3'` deixam `state.toggles.snap` no valor original (toggle duplo). Subscriber em `toggle:changed` recebe dois eventos.
6. **Esc cancela:** com ferramenta ativa `line`, `keydown` `Escape` → bus emite `tool:cancel` com `{toolId:'line'}` e `state.toolState` passa por `'cancel'` para `'idle'`.
7. **Mutação direta lançar (em modo dev):** se houver um proxy/`Object.defineProperty` configurado, `state.activeTool = 'line'` deve lançar; caso contrário, esta verificação é apenas documental (ver §7).
8. `state.setCamera({cx:NaN, cy:0, zoom:1})` lança um `Error` e **não** emite `camera:changed`.
9. `state.commandHistory.length` é estritamente `<= app.config.MAX_COMMAND_HISTORY` após qualquer número de `pushCommandHistory`.

## 7. Notas de implementação

### Atalhos sem evento dedicado

- `Space` para pan: não há evento canônico de "pan" na lista §3 do state-contract. Sprint 1 deixa a tecla **inerte** ou registra um handler interno que aciona `render.camera` diretamente. Decisão: **inerte** na Sprint 1; pan por mouse-middle é suficiente para abrir/zoom-extents/atalho.
- Foco automático no command line ao digitar alfanumérico fora de input (design.md L206) é um efeito puramente de UI; `shortcuts.install` move `document.activeElement` para o input da command line e propaga a tecla.

### Mutação direta

- Sprint 1 **não** instala Proxy/freeze profundo sobre `state`. A regra "ninguém atribui direto" é convenção (state-contract.md §2.2) e violações aparecem só em revisão de código. Considerar Proxy em sprint futura se conferir benefício (provavelmente custo > benefício para perf no MVP).

### Auto-fit

- `app.config.AUTO_FIT_ON_BOOT = true`. `bootstrap.js` (outro WS) escuta `viewport:resized` e chama `state.applyCommand(commands.setCamera(...))` com `cx = w/2`, `cy = h/2` (centro do documento), `zoom = computeFitZoom(documentBounds, viewport, margin)`. Sprint 1 trava `margin = 0.9` (10% de margem total).

### Outras notas

- Plan.md L223: máquina de estados `idle|armed|preview|commit|cancel` é manipulada por `tools/`. `state.setToolState` é o setter, mas quem decide a transição é a ferramenta ativa.
- Plan.md L226: undo/redo via histórico. `state.applyCommand` é o único caminho para empurrar no `past`.
- A lista de atalhos cobre **todos** os mencionados em design.md L209–211 e L245–250 que sejam relevantes para Sprint 1 (chord `Z E`/`Z W` e Ctrl+letras ficam stub).
- `state-contract.md` §3 enumera os **10 eventos canônicos**; este módulo emite somente os apropriados para mutações que executa (`tool:armed`, `camera:changed`, `cursor:moved`, `toggle:changed`). Demais eventos vêm de outros módulos.
