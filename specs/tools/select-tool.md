# tools.selectTool

## 1. Responsabilidade

Implementar a ferramenta `select` em sua forma minima de Sprint 1: estados `idle` e `armed` apenas. Existe para validar o contrato do `tool-manager` (registro, transicoes, prompts) antes da Sprint 3, onde ela ganhara hit-test, box-select e seleção de entidades reais.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.tools.toolManager` (sera o consumidor que `register` esta ToolDef)
  - `window.LaserCAD.bus` (via `ctx.bus`)
  - `window.LaserCAD.app.state` (via `ctx.state`, leitura apenas)
- ordem de carga: posicao #21 em `specs/_conventions/namespace.md`. Depois de `tools/tool-manager.js`, antes de toda `ui/*`.

## 3. API publica

```js
/**
 * Definicao da ferramenta select. Conforma com a ToolDef descrita em
 * specs/tools/tool-manager.md §3.
 */
window.LaserCAD.tools.selectTool = {
  id: 'select',
  prompt: 'Select objects:',

  /**
   * Chamado pelo tool-manager na transicao -> 'armed'.
   * Sprint 1: limpa selecao "visualmente" via emissao opcional (NAO emite — nao ha
   * evento canonico de 'selection:changed'; state.selection ja eh subido). Simplesmente
   * confirma a entrada em 'armed' e nao faz mais nada.
   * @param {ToolContext} ctx
   * @returns {void}
   */
  onArm(ctx) {
    /* ... */
  },

  /**
   * Chamado pelo tool-manager na transicao -> 'cancel'.
   * Sprint 1: no-op alem de qualquer cleanup interno (variaveis locais).
   * @param {ToolContext} ctx
   * @returns {void}
   */
  onCancel(ctx) {
    /* ... */
  },

  /**
   * Chamado pelo tool-manager quando ui.commandLine emite command:submit
   * enquanto select esta ativa. Sprint 1: ignora qualquer comando que nao
   * seja uma troca de ferramenta (o tool-manager ja roteia tool:request).
   * Comandos de coordenada ou desenho disparam command:error com a mensagem
   * "Sprint 1" — mas esse erro ja foi emitido pela commandLine no parse.
   * Aqui simplesmente nao fazemos nada.
   * @param {ToolContext} ctx
   * @param {{ raw: string, parsed: object|null }} payload
   * @returns {void}
   */
  onCommandSubmit(ctx, payload) {
    /* ... */
  },

  /**
   * Pointer events sao roteados por render.overlays (WS-B) para a ferramenta ativa.
   * Sprint 1: select NAO reage a clique no viewport — nao ha entidades para selecionar.
   * Tres handlers existem como no-ops para validar o contrato.
   * @param {ToolContext} ctx
   * @param {PointerEvent} ev
   * @param {{x:number, y:number}} world - Coordenadas em mm.
   */
  onPointerDown(ctx, ev, world) {
    /* no-op em Sprint 1 */
  },
  onPointerMove(ctx, ev, world) {
    /* no-op em Sprint 1 */
  },
  onPointerUp(ctx, ev, world) {
    /* no-op em Sprint 1 */
  },
};
```

### 3.1 Estados implementados na Sprint 1

| Estado    | Implementado | Comportamento                                                                                                                                                                                 |
| --------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `idle`    | sim          | Quando o tool-manager mantem `'select'` como `activeTool` mas `toolState === 'idle'`. Nada acontece. Improvavel de ocorrer apos bootstrap (que arma `select`).                                |
| `armed`   | sim          | Estado padrao do app. `onArm` foi chamado. Cursor crosshair em `--laser-glow` (responsabilidade visual de WS-B). Linha 2 da command line mostra `Select objects:`. Click no viewport = no-op. |
| `preview` | **NAO**      | Sprint 3 (drag para box-select).                                                                                                                                                              |
| `commit`  | **NAO**      | Sprint 3.                                                                                                                                                                                     |
| `cancel`  | sim          | Quando o usuario aperta Esc. `onCancel` invocado. O tool-manager re-arma `select` automaticamente (politica §4.4 de `tool-manager.md`).                                                       |

### 3.2 Prompt na command line

A linha 2 da command line mostra `Select objects:` enquanto `state.activeTool === 'select' && state.toolState === 'armed'`. Esse prompt vem do campo `prompt` da `ToolDef` lida pelo `ui.commandLine` via `tool:armed` (que carrega `toolId`) seguido de lookup em `LaserCAD.tools.toolManager.registered['select'].prompt` (ou exposto por API equivalente — `tool-manager.md` §3 nao fixa o nome do getter; aceita que `commandLine` mantenha um pequeno mapa `toolId → prompt` se preferir).

## 4. Invariantes e tolerancias

- **Sem hit-test, sem box-select.** Click no viewport eh ignorado pelo `select` na Sprint 1. Quem implementa o hit-test eh a Sprint 3.
- **Sem mutacao de `state.selection`.** O array fica vazio durante toda a Sprint 1 (state-contract.md §1.1 default eh `[]`). Nenhum codigo de Sprint 1 adiciona ou remove ids.
- **Sem chamadas a `core.document.commands`.** Plan.md L226 garante que selecao/desselecao virariam comandos eventualmente; Sprint 1 nao toca este caminho.
- **`onArm`/`onCancel` sao puros em efeito visivel.** Nao manipulam DOM diretamente. Em Sprint 3+, podem agendar limpezas via `ctx.setState('preview')` ou similar.
- **`prompt` literal.** `'Select objects:'`. Maiusculas e minusculas exatamente assim.
- **Nenhum evento emitido pela select-tool diretamente na Sprint 1.** Os eventos `tool:armed`/`tool:cancel` saem do `tool-manager`, nao da tool.

## 5. Exemplos de uso

```js
// Registro (feito por bootstrap):
LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool);

// Ativacao via toolbar / commandLine / menubar:
LaserCAD.bus.emit('tool:request', { toolId: 'select' });
// tool-manager: state.activeTool='select', state.toolState='armed'
// emit 'tool:armed' { toolId:'select' }
// ui.commandLine: linha 2 vira "Select objects:"

// Cancelar (Esc):
LaserCAD.tools.toolManager.cancel();
// emit 'tool:cancel' { toolId:'select' }
// onCancel(ctx) chamado.
// tool-manager re-arma 'select' (politica §4.4 de tool-manager.md)
// emit 'tool:armed' { toolId:'select' } de novo.

// Click no viewport (Sprint 1):
// render.overlays mapeia para world e chama onPointerDown(ctx, ev, world).
// Resultado: nada visivel acontece. state.selection permanece [].

// Inspecionar a tool registrada:
LaserCAD.tools.toolManager.list();
// → ['select']
LaserCAD.tools.selectTool.prompt;
// → 'Select objects:'
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Existe no namespace.** `typeof LaserCAD.tools.selectTool === 'object' && LaserCAD.tools.selectTool.id === 'select'`.
2. **Prompt correto.** `LaserCAD.tools.selectTool.prompt === 'Select objects:'`.
3. **Apos `app:ready`, esta armada.** `LaserCAD.app.state.activeTool === 'select'` e `LaserCAD.app.state.toolState === 'armed'`.
4. **Linha 2 da command line mostra prompt.** Inspecao DOM em `#commandline-host`: texto contem `Select objects:` (case-sensitive).
5. **Click no viewport eh no-op.** Click em coordenadas centrais do `<svg>`. Antes: `state.selection === []`. Depois: `state.selection === []`. Nada lancado no console.
6. **`tool:cancel` re-arma `select`.** `LaserCAD.bus.on('tool:armed', (p) => console.log('armed', p.toolId)); LaserCAD.tools.toolManager.cancel();` → log `armed select` aparece (re-arme automatico). `state.toolState === 'armed'`.
7. **Re-request idempotente.** Emitir `LaserCAD.bus.emit('tool:request', { toolId:'select' })` 3x. `state.selection` permanece `[]`; nada quebra; console mostra 3 eventos `tool:armed`.
8. **`onPointerDown/Move/Up` nao lancam.** Chamar diretamente (com `ctx` fake) no DevTools: `LaserCAD.tools.selectTool.onPointerDown({state:LaserCAD.app.state, bus:LaserCAD.bus, setState:()=>{}}, null, {x:50,y:50});` — sem excecao.

## 7. Notas de implementacao

- Briefing do WS-C: "Sprint 1 implementa apenas os estados `idle` e `armed`. Clique no viewport em modo armed nao faz nada (nao ha entidades). Existe para validar o contrato do `tool-manager` antes da Sprint 3." — esta spec cumpre exatamente.
- Plan.md L223 lista a maquina; `select-tool` so visita 2 dos 5 estados na Sprint 1.
- Plan.md L189 reserva `select-tool.js` no diretorio `tools/`; namespace.md §3 fixa a ordem #21.
- Design.md L143 mostra Select como uma das 10 ferramentas da toolbar; design.md L321–331 estabelece visual por estado — Sprint 1 entrega apenas o estado visual `armed`, exibido pelo cursor crosshair em `--laser-glow` (responsabilidade visual de WS-B).
- Por que ter os handlers de pointer como no-ops: garante que o contrato entre `render.overlays` (WS-B) e `tools.*` (WS-C) seja exercitado, mesmo sem efeito visivel. Sprint 3 simplesmente preenche os no-ops.
- `state.selection` em Sprint 1 eh um array vazio garantido. Modulos que dependem de selecao (futura `delete-tool`, `move-tool`) ainda nao existem.
- Nao emite `selection:changed` (nao existe na lista canonica — state-contract.md §3.2 explicita a omissao). Sprint 3 reabre essa discussao via ADR se necessario.
- O `commandLine` consumidor do prompt pode ler `LaserCAD.tools.toolManager.list()`/getter equivalente ou manter um mapa local; tanto faz para a Sprint 1 — o prompt eh estatico aqui.
