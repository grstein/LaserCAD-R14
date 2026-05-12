# tools.toolManager

## 1. Responsabilidade

Implementar a maquina de estados de ferramentas (`idle | armed | preview | commit | cancel`), gerenciar o registro de ferramentas concretas, coordenar transicoes ao receber `tool:request` e `command:submit`, e emitir os eventos canonicos `tool:armed`/`tool:cancel`.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus`
  - `window.LaserCAD.app.state` (`setActiveTool`, `setToolState`)
  - `window.LaserCAD.core.document.commands`, `window.LaserCAD.core.document.history` (consumidos pelas tools, mas o manager passa referencias quando aciona `tool.commit()`)
- ordem de carga: posicao #20 em `specs/_conventions/namespace.md`. Depois de `app.state` e `core.document.*`, antes das tools concretas (`select-tool` #21) e antes da UI (#22+).

## 3. API publica

```js
/**
 * @typedef {Object} ToolDef
 * @property {string} id                 - Id canonico (ex.: 'select', 'line').
 * @property {string} prompt             - Texto inicial mostrado na linha 2 da command line ao armar (ex.: 'Select objects:').
 * @property {(ctx: ToolContext) => void} [onArm]
 *           - Chamado na transicao -> 'armed'. ctx contem helpers para emitir e ler state.
 * @property {(ctx: ToolContext) => void} [onCancel]
 *           - Chamado na transicao -> 'cancel'.
 * @property {(ctx: ToolContext, payload: any) => void} [onCommandSubmit]
 *           - Chamado quando ui.commandLine emite command:submit ENQUANTO esta ferramenta esta ativa.
 *             A ferramenta consome ou ignora; o tool-manager nao interpreta.
 * @property {(ctx: ToolContext, ev: PointerEvent, world: {x:number, y:number}) => void} [onPointerDown]
 * @property {(ctx: ToolContext, ev: PointerEvent, world: {x:number, y:number}) => void} [onPointerMove]
 * @property {(ctx: ToolContext, ev: PointerEvent, world: {x:number, y:number}) => void} [onPointerUp]
 *           - Sao roteados pela camada de render/overlays (WS-B) que mapeia screen→world
 *             e chama o handler ativo. Sprint 1: nenhum desses eh efetivamente acionado por select.
 */

/**
 * @typedef {Object} ToolContext
 * @property {(stateName: 'idle'|'armed'|'preview'|'commit'|'cancel') => void} setState
 *           - Solicita transicao. O manager valida e chama state.setToolState.
 * @property {Object} state           - Referencia somente leitura a window.LaserCAD.app.state.
 * @property {Object} bus             - Atalho para window.LaserCAD.bus.
 */

window.LaserCAD.tools.toolManager = {
  /**
   * Registra uma ferramenta. Sobrescreve se id ja existe (loga aviso).
   * @param {string} toolId
   * @param {ToolDef} toolDef
   * @returns {void}
   */
  register(toolId, toolDef) { /* ... */ },

  /**
   * Pedido oficial para armar uma ferramenta. Chamado em resposta a tool:request,
   * ou diretamente em casos especiais (bootstrap).
   *   - Se a ferramenta `toolId` nao esta registrada, emite command:error e nao transita.
   *   - Se ja eh a ferramenta ativa em estado 'armed', re-arma (idempotente, mas re-emite tool:armed).
   *   - Se outra ferramenta esta em 'preview', cancela-a antes de armar a nova.
   * @param {string} toolId
   * @returns {void}
   */
  request(toolId) { /* ... */ },

  /**
   * Solicita transicao para um estado. Chamado tipicamente por tools via ctx.setState.
   * Valida a transicao contra a tabela §4.2.
   * @param {'idle'|'armed'|'preview'|'commit'|'cancel'} stateName
   * @returns {void}
   */
  enter(stateName) { /* ... */ },

  /**
   * Cancela a ferramenta ativa: transita para 'cancel' (1 frame) e em seguida 'idle'.
   * Emite tool:cancel. Re-arma 'select' como ferramenta padrao apos volta a idle.
   * @returns {void}
   */
  cancel() { /* ... */ },

  /**
   * Retorna o id da ferramenta atualmente ativa (= state.activeTool).
   * @returns {string}
   */
  active() { /* ... */ },

  /**
   * Retorna lista de toolIds registrados. Util para diagnostico.
   * @returns {string[]}
   */
  list() { /* ... */ }
};
```

### 3.1 Subscrições do bus instaladas em `mount`/`init`

O `tool-manager` se inscreve em:

- `tool:request` → chama `request(payload.toolId)`.
- `command:submit` → encaminha para `toolDef.onCommandSubmit` da ferramenta ativa (se a ferramenta a tiver), passando `payload`.

Nao se inscreve em pointer events diretamente; isso eh papel de `render.overlays` (WS-B), que ja despacha para a ferramenta ativa via API a definir entre WS-B e WS-C (Sprint 3+).

### 3.2 Forma do `ToolContext`

O `ctx` passado para callbacks da `ToolDef` eh **estavel** (mesma referencia ao longo do ciclo de vida da ferramenta), com:

- `ctx.setState(name)` — atalho para `toolManager.enter(name)`, mas escopado para a ferramenta corrente (ignorado se a ferramenta ja nao estiver ativa).
- `ctx.state` — `window.LaserCAD.app.state` (somente leitura conforme state-contract §2).
- `ctx.bus` — `window.LaserCAD.bus`.

## 4. Invariantes e tolerancias

### 4.0 Invariantes principais

- **Unica fonte da verdade do `toolState` eh `app.state.toolState`.** O manager le e muta via setters; ninguem mais muta diretamente.
- **`commit` e `cancel` sao transitorios.** O manager agenda a saida via `queueMicrotask` ou `requestAnimationFrame`. Tools podem assumir que o estado `commit`/`cancel` nao persiste.
- **Re-request da mesma ferramenta eh idempotente em efeito visual.** Mas re-emite `tool:armed` (proposital — permite a UI re-renderizar/limpar prompts).
- **Sem ferramentas com side-effects globais alem do estado/bus.** Tools nao tocam DOM diretamente (delegam a `render.*`).
- **Nenhum evento fora da lista canonica** de state-contract.md §3.
- **`register` chamado duas vezes para o mesmo `toolId`** sobrescreve com warning. Em Sprint 1 isso so acontece se `bootstrap` for chamado mais de uma vez (idempotente, ver bootstrap.md §3.1.1).

### 4.1 Maquina de estados — Estados

| Estado    | Significado                                                                  | Duracao            |
|-----------|------------------------------------------------------------------------------|--------------------|
| `idle`    | Nenhuma ferramenta armada. `state.activeTool` ainda eh valido, mas a ferramenta nao esta capturando entrada. | persistente |
| `armed`   | Ferramenta armada; pronta para receber primeiro input.                       | persistente        |
| `preview` | Ferramenta capturou input parcial e exibe preview (linha tracejada, etc.).   | persistente        |
| `commit`  | Transitorio (1 frame). Sinaliza que a operacao foi confirmada; o documento sera mutado via `core.document.commands`. Apos o frame, volta a `armed` (para proximo segmento) ou `idle`. | 1 frame            |
| `cancel`  | Transitorio (1 frame). Sinaliza que o usuario abortou (Esc) ou outra ferramenta tomou foco. Apos o frame, volta a `idle`. | 1 frame            |

### 4.2 Tabela de transicoes permitidas

```text
   ┌────────┐                                ┌─────────┐
   │  idle  │ ─── request(toolId) ────────▶  │  armed  │
   └────────┘                                └────┬────┘
        ▲                                         │
        │                                         │ tool input (1st click/keys)
        │                                         ▼
        │                                    ┌─────────┐
        │                                    │ preview │ ◀─┐
        │                                    └────┬────┘   │ (loop p/ proximos pontos
        │                                         │        │  em line/polyline; sai
        │                  ┌──────────────────────┤        │  por commit ou cancel)
        │                  ▼                      │        │
        │             ┌─────────┐                 │        │
        │             │ commit  │ (1 frame) ──────┴────────┘
        │             └────┬────┘
        │                  │                                 (volta a 'armed' para
        │                  ▼                                  proximo elemento, ou
        │             ┌─────────┐                             a 'idle' se concluiu)
        │             │   ?     │ (idle ou armed)
        │             └─────────┘
        │
        │
        │   Esc / cancel() / tool:request de outra
        │   ┌──────────────────────┐
        └───┤  cancel  │ (1 frame) │
            └──────────┴───────────┘
                 ▲
                 │
            de qualquer 'armed'/'preview'
```

| De \ Para | idle | armed | preview | commit | cancel |
|---|---|---|---|---|---|
| **idle**    | (re-emite request) | OK via `request(id)` | — | — | — |
| **armed**   | OK via `cancel()` | OK (re-arm) | OK (1o input) | — | OK (Esc, novo `request`) |
| **preview** | — | OK (volta apos commit incompleto) | (re-emite) | OK (confirma) | OK (Esc) |
| **commit**  | OK (operacao concluida — fim do desenho) | OK (operacao parcial — proximo segmento) | — | (instantaneo) | — |
| **cancel**  | OK (apos 1 frame) | — | — | — | (instantaneo) |

Transicoes nao listadas como OK sao **invalidas**: `enter` loga `console.warn('[tool-manager] invalid transition X → Y')` e nao muta `state.toolState`.

### 4.3 Eventos emitidos

- **`tool:armed`** com `{ toolId }` em qualquer transicao para `armed`.
- **`tool:cancel`** com `{ toolId }` em qualquer entrada em `cancel`.
- Nada para `preview`/`commit` na Sprint 1 (state-contract §3.2 deliberadamente omite `state:changed`).

### 4.4 Politica de re-arme apos `cancel`

Apos `cancel` (e o frame transitorio), o manager re-arma `'select'` como ferramenta default:
1. `state.setToolState('idle')`
2. `state.setActiveTool('select')`
3. `request('select')` (que vira a `armed` e emite `tool:armed`).

Isso garante que o app esteja sempre numa "posicao de descanso operacional" (`select` armada), nunca em `idle` morto.

## 5. Exemplos de uso

```js
// Bootstrap registra select:
LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool);

// Solicitar troca por API direta:
LaserCAD.tools.toolManager.request('select');
// → emite 'tool:armed' { toolId:'select' }
// → state.activeTool = 'select', state.toolState = 'armed'

// Pelo bus (caminho da toolbar/commandLine/menubar):
LaserCAD.bus.emit('tool:request', { toolId: 'select' });
// idem.

// Cancelar:
LaserCAD.tools.toolManager.cancel();
// → emite 'tool:cancel' { toolId:'select' }
// → state.toolState = 'cancel' (1 frame) → 'idle' → re-armed 'select' → 'armed'

// Listar ferramentas registradas:
LaserCAD.tools.toolManager.list();
// → ['select']    (Sprint 1)

// Ferramenta nao registrada:
LaserCAD.bus.emit('tool:request', { toolId: 'line' });
// → emite 'command:error' com payload: { raw: 'tool-manager: line', message: '! Tool not registered: line' }
//   (o campo raw eh livre — descreve a origem do erro; nao eh nome de evento)
// → estado nao muda.
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Registro inicial.** Apos `app:ready`, `LaserCAD.tools.toolManager.list()` retorna `['select']`.
2. **Re-arm explicito emite `tool:armed`.** `LaserCAD.bus.on('tool:armed', console.log); LaserCAD.bus.emit('tool:request', { toolId:'select' });` → console mostra `{ toolId:'select' }`.
3. **`request` para nao registrada.** `LaserCAD.bus.on('command:error', console.log); LaserCAD.bus.emit('tool:request', { toolId:'line' });` → recebe `command:error`. `state.activeTool` permanece `'select'`. `state.toolState` permanece `'armed'`.
4. **Cancel produz fluxo cancel→idle→armed.** Subscriber `LaserCAD.bus.on('tool:cancel', console.log); LaserCAD.tools.toolManager.cancel();` → recebe `{ toolId:'select' }`. Apos um microtick, `state.toolState === 'armed'` de novo (re-arm de `select`).
5. **Transicao invalida loga warning.** `LaserCAD.tools.toolManager.enter('preview');` quando state.toolState eh `'idle'` (e nao havia ferramenta armada) — console mostra `[tool-manager] invalid transition idle → preview`. Estado nao muda.
6. **`command:submit` eh roteado.** Registrar `selectTool` com `onCommandSubmit = (ctx, p) => console.log('got', p.raw)`. Emitir `LaserCAD.bus.emit('command:submit', { raw:'foo', parsed:null })` — console mostra `got foo`.
7. **`active()` reflete state.** `LaserCAD.tools.toolManager.active() === LaserCAD.app.state.activeTool`.
8. **Inspecionar via DevTools.** Ver `LaserCAD.app.state.activeTool` e `LaserCAD.app.state.toolState` sincronizados com o manager. Ex.: depois de `request('select')`, ambos = `'select'` e `'armed'`.

## 7. Notas de implementacao

- Plan.md L223 fixa a maquina `idle / armed / preview / commit / cancel`. Esta spec reflete exatamente.
- Design.md L321–331 traz o mapeamento UI ↔ estado, util para entender o por que de cada transicao.
- O briefing do WS-C exige diagrama de transicoes em ASCII — entregue em §4.2. Tambem entregue tabela exaustiva.
- A politica de re-arme em `cancel` (§4.4) eh decisao deliberada para que o app nunca fique sem ferramenta — mantem ergonomia do R14 onde `select` eh sempre o "estado base".
- Nao confundir `state.activeTool` (id, persiste mesmo em `cancel`/`idle`) com `state.toolState` (transito da maquina). O briefing menciona "select-tool... clique nao faz nada" (`select-tool.md`) — o `activeTool` continua `'select'` o tempo todo.
- A interface `ToolContext` eh estavel para evitar que tools recriem closures por chamada — Sprint 3+ desenhara tools com preview que dependem disso.
- Pre-commit (Sprint 3+): `commit` aciona `core.document.commands.add(...)` que vai por `core.document.history.push(...)` para suportar undo (plan.md L266).
- Eventos: emite `tool:armed`, `tool:cancel`, ocasionalmente `command:error` (para "tool not registered"). Nada fora da lista.
- O `tool-manager` **NAO** se inscreve em `cursor:moved`. Pointer events brutos chegam por `render.overlays` (WS-B), que entao chama os handlers `onPointerDown/Move/Up` da tool ativa. Esta linha de despacho entre WS-B e WS-C eh contratual: em Sprint 1, com select sem comportamento de pointer, ela pode ser ignorada — basta a presenca da API.
