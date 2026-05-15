# Contrato de estado e eventos — `window.LaserCAD.app.state` + `window.LaserCAD.bus`

Este documento **congela** a forma do singleton de estado global e a lista canônica de eventos do bus. Os três entregáveis da Sprint 1 (WS-A/B/C) e todas as sprints subsequentes leem este contrato como verdade. Alterações exigem novo ADR.

Dependências de leitura:

- `docs/adr/0001-arquitetura-base.md` (decisões SVG-first, mm, namespace global)
- `specs/_conventions/namespace.md` (padrão IIFE e ordem de scripts)
- `plan.md` L17–30 (base técnica), L215–226 (convenções), L342–350 (questões em aberto)
- `design.md` L80–118 (regiões da UI), L186–235 (command line e status bar), L321–331 (máquina de estados)

---

## 1. Forma exata do singleton `window.LaserCAD.app.state`

```js
window.LaserCAD.app.state = {
  schemaVersion: 1,
  units: 'mm',
  documentBounds: { w: 128, h: 128 },
  entities: [],
  selection: [],
  camera: { cx: 0, cy: 0, zoom: 1, viewportW: 0, viewportH: 0 },
  activeTool: 'select',
  toolState: 'idle',
  cursor: { worldX: 0, worldY: 0, screenX: 0, screenY: 0 },
  toggles: { snap: true, grid: true, ortho: false },
  commandHistory: [],
  commandInput: '',
};
```

Esta é a única forma autorizada do estado. Nenhum campo extra; nenhum campo a menos. Adições exigem ADR.

### 1.1 Catálogo de campos

| Campo            | Tipo                                               | Valor inicial                                           | Descrição                                                                                                                                                                                                                                                | Fonte                            |
| ---------------- | -------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `schemaVersion`  | `number` (inteiro)                                 | `1`                                                     | Versão do schema do documento para migrações futuras. Persistido junto com `entities` em export/autosave.                                                                                                                                                | plan.md L317                     |
| `units`          | `'mm'` (literal)                                   | `'mm'`                                                  | Unidade canônica. Travada em `'mm'` no MVP; existe como campo para suportar interrogação programática e migrações futuras.                                                                                                                               | plan.md L217; ADR 0001 §2        |
| `documentBounds` | `{ w: number, h: number }`                         | `{ w: 128, h: 128 }`                                    | Área lógica do documento em mm. Define o `viewBox` do `<svg>` raiz e o `width`/`height` da exportação. 128×128 é referência recorrente (não obrigatória; será configurável em sprints futuras).                                                          | plan.md L346                     |
| `entities`       | `Array<Entity>`                                    | `[]`                                                    | Lista de entidades do documento. Cada item: `{ id: 'e_<n>', type: 'line'\|'polyline'\|'rect'\|'circle'\|'arc', ...campos específicos do tipo }`. Schema detalhado vive em `core/document/schema.js` (JSDoc `@typedef`).                                  | plan.md L93, L170–174            |
| `selection`      | `Array<string>`                                    | `[]`                                                    | Lista de `id` das entidades atualmente selecionadas. Subconjunto de `entities[*].id`.                                                                                                                                                                    | plan.md L94, L120                |
| `camera`         | `{ cx, cy, zoom, viewportW, viewportH }`           | `{ cx: 0, cy: 0, zoom: 1, viewportW: 0, viewportH: 0 }` | Estado da câmera. `cx`/`cy` em mm (centro do viewport em world space); `zoom` adimensional (px-por-mm × constante); `viewportW`/`viewportH` em px. `viewportW`/`H` começam em `0` e são preenchidos pelo `bootstrap` após o primeiro `viewport:resized`. | plan.md L19, L26, L269           |
| `activeTool`     | `string` (id)                                      | `'select'`                                              | Id da ferramenta ativa. Valores do MVP: `'select'`, `'line'`, `'polyline'`, `'rect'`, `'circle'`, `'arc'`, `'trim'`, `'extend'`, `'move'`, `'delete'`.                                                                                                   | design.md L131–152, L242–249     |
| `toolState`      | `'idle'\|'armed'\|'preview'\|'commit'\|'cancel'`   | `'idle'`                                                | Sub-estado da máquina de ferramentas (plan.md L223). `commit` e `cancel` são transitórios (1 frame).                                                                                                                                                     | plan.md L223; design.md L321–331 |
| `cursor`         | `{ worldX, worldY, screenX, screenY }`             | `{ worldX: 0, worldY: 0, screenX: 0, screenY: 0 }`      | Posição atual do cursor. `world*` em mm; `screen*` em px. Atualizado a cada `cursor:moved`.                                                                                                                                                              | design.md L117, L185, L230       |
| `toggles`        | `{ snap: boolean, grid: boolean, ortho: boolean }` | `{ snap: true, grid: true, ortho: false }`              | Estados dos toggles globais. Refletem `F3`/`F7`/`F8` e os controles da status bar.                                                                                                                                                                       | design.md L225, L231–236         |
| `commandHistory` | `Array<string>`                                    | `[]`                                                    | Últimas N entradas da command line (N = 50 sugerido; cap em `app.config`). Navegação `↑`/`↓` consome esta lista.                                                                                                                                         | design.md L210                   |
| `commandInput`   | `string`                                           | `''`                                                    | Conteúdo atual do campo de entrada da command line (linha 3 do bloco).                                                                                                                                                                                   | design.md L195–203               |

### 1.2 Notas sobre tipagem

- Todos os campos numéricos são `number` JavaScript (ponto flutuante 64 bits). Não usar `BigInt` no MVP.
- `id` de entidade segue o padrão `'e_<n>'` onde `<n>` é um inteiro crescente. O contador vive privado em `core.document.commands` (não no estado).
- Ângulos eventualmente armazenados em entidades (`arc.start`, `arc.end`) são **radianos** no estado, conforme convenção do kernel (plan.md L218; ADR 0001 §2.4).
- `documentBounds.w`/`h` são sempre positivos. Validação em `core.document.validators`.

---

## 2. Quem pode mutar o estado

> **Regra dura:** somente o módulo `LaserCAD.app.state` muta o estado. Todos os outros módulos **leem** e **emitem eventos**.

### 2.1 Caminhos autorizados

- **Mutação:** chamada explícita a setters/comandos expostos por `LaserCAD.app.state`. Ex.: `LaserCAD.app.state.setActiveTool('line')`, `LaserCAD.app.state.applyCommand(cmd)`.
- **Leitura:** acesso direto a `LaserCAD.app.state.<campo>` é permitido **somente leitura**. Para manter previsibilidade, módulos preferem reagir a eventos do bus em vez de pollar campos.

### 2.2 Anti-padrões proibidos

- Atribuição direta de fora: `LaserCAD.app.state.activeTool = 'line'` — **proibido**.
- Mutação de objetos aninhados: `LaserCAD.app.state.toggles.snap = false` — **proibido**.
- `push`/`splice` direto em `entities` ou `selection` a partir de fora de `app.state` — **proibido**.
- Manter cópia local prolongada do estado em outro módulo (snapshot stale) — **evitar**; preferir reagir aos eventos.

### 2.3 Modelo de mutação

`LaserCAD.app.state` expõe uma API estreita (a ser detalhada por WS-A na Sprint 1):

```js
// Sketch da API — assinaturas definitivas ficam em src/app/state.js + JSDoc
LaserCAD.app.state.setActiveTool(toolId); // emite tool:armed
LaserCAD.app.state.setToolState(stateName);
LaserCAD.app.state.setCamera({ cx, cy, zoom }); // emite camera:changed
LaserCAD.app.state.setViewportSize(w, h); // emite viewport:resized (indireto)
LaserCAD.app.state.setCursor({ worldX, worldY, screenX, screenY }); // emite cursor:moved
LaserCAD.app.state.setToggle(name, value); // emite toggle:changed
LaserCAD.app.state.setCommandInput(str);
LaserCAD.app.state.pushCommandHistory(raw);
LaserCAD.app.state.applyCommand(cmd); // mutação de entities/selection via core.document.commands
LaserCAD.app.state.setDocumentBounds({ w, h });
```

Comandos do documento (add/update/delete de entidade, mudança de seleção) passam obrigatoriamente por `core.document.commands` + `core.document.history` antes de tocar o estado — isso garante undo/redo correto (plan.md L226).

---

## 3. Lista canônica fechada de eventos do bus

O `window.LaserCAD.bus` é um pub/sub simples: `on(evt, fn)`, `off(evt, fn)`, `emit(evt, payload)`. A lista abaixo é **fechada**: nenhuma spec, subagente ou módulo pode inventar eventos fora desta tabela. Adições exigem ADR.

| Evento             | Payload                                                                | Quem emite                                                            | Quem consome                                     |
| ------------------ | ---------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------ |
| `app:ready`        | `{}`                                                                   | `app.bootstrap`                                                       | `ui.*`, `render.*`                               |
| `viewport:resized` | `{ w: number, h: number }` (px)                                        | `app.bootstrap` (via `ResizeObserver` no contêiner do `<svg>`)        | `render.camera`, `render.svgRoot`                |
| `camera:changed`   | `{ cx: number, cy: number, zoom: number }`                             | `render.camera`                                                       | `render.grid`, `render.overlays`, `ui.statusbar` |
| `cursor:moved`     | `{ worldX: number, worldY: number, screenX: number, screenY: number }` | `render.overlays` (após mapear o pointer event para world via câmera) | `ui.statusbar`                                   |
| `tool:request`     | `{ toolId: string }`                                                   | `ui.toolbar`, `ui.commandLine`, `ui.menubar`                          | `tools.toolManager`                              |
| `tool:armed`       | `{ toolId: string }`                                                   | `tools.toolManager`                                                   | `ui.toolbar`, `ui.commandLine`                   |
| `tool:cancel`      | `{ toolId: string }`                                                   | `tools.toolManager`                                                   | `ui.toolbar`, `ui.commandLine`                   |
| `command:submit`   | `{ raw: string, parsed: object\|null }`                                | `ui.commandLine`                                                      | `tools.toolManager`                              |
| `command:error`    | `{ raw: string, message: string }`                                     | `ui.commandLine`, `tools.*`                                           | `ui.commandLine`                                 |
| `toggle:changed`   | `{ name: 'snap'\|'grid'\|'ortho', value: boolean }`                    | `ui.statusbar`, `ui.menubar` (via atalho `F3`/`F7`/`F8`)              | `render.grid`, `tools.*`                         |

### 3.1 Notas por evento

- **`app:ready`** — disparado uma única vez, depois que `bootstrap` montou o `<svg>` raiz e registrou subscribers iniciais.
- **`viewport:resized`** — `w`/`h` em pixels do contêiner do `<svg>`. `render.camera` atualiza `state.camera.viewportW/H` em resposta; `render.svgRoot` ajusta atributos do elemento `<svg>` (não o `viewBox`, que é definido pelo `documentBounds`/câmera).
- **`camera:changed`** — emitido após pan/zoom. Não inclui `viewportW`/`H` (esses não mudam em pan/zoom).
- **`cursor:moved`** — fonte oficial: `render.overlays` (que escuta `pointermove` no `<svg>`). Status bar consome para atualizar coordenadas; tools consomem indiretamente via `app.state.cursor`.
- **`tool:request`** — pedido para trocar de ferramenta (ex.: clique na toolbar, comando `line` digitado). É um pedido, não um fato; o `tool:armed` confirma.
- **`tool:armed`** — confirma a troca; `state.activeTool` já foi atualizado e `state.toolState === 'armed'`.
- **`tool:cancel`** — usuário apertou `Esc`, ou outra ferramenta tomou o foco. `state.toolState === 'cancel'` por 1 frame e volta para `'idle'`.
- **`command:submit`** — `raw` é exatamente o que o usuário digitou; `parsed` é a forma estruturada (ex.: `{ kind: 'absolute', x: 124.5, y: 87.3 }`). Quando o parse falha, `parsed === null` e segue-se um `command:error`.
- **`command:error`** — exibido na command line com prefixo `!` em `--status-error` (design.md L212).
- **`toggle:changed`** — único caminho oficial para alternar `snap`/`grid`/`ortho`. Tanto o clique na status bar quanto a tecla (`F3`/`F7`/`F8` via `app.shortcuts` → `ui.menubar`/`ui.statusbar`) terminam aqui.

### 3.2 Eventos NÃO incluídos no MVP

Os seguintes eventos foram considerados e deliberadamente **omitidos** desta lista canônica. Caso alguma sprint precise, deverá abrir ADR de adição:

- `state:changed` genérico — descartado em favor de eventos específicos com payload tipado.
- `entity:added` / `entity:removed` / `entity:updated` — no MVP, `tools.*` reage ao próprio `command:submit` e re-render é disparado por `camera:changed` ou re-leitura na próxima `requestAnimationFrame`. Reavaliar na Sprint Drawing.
- `selection:changed` — coberto implicitamente pelo `applyCommand` (que persiste a seleção) e por re-render. Reavaliar quando `selectTool` for implementada.
- `autosave:*` — pertence à `io.autosave`, que está vazia na Sprint 1.

### 3.3 Regra de extensão

Qualquer subagente que sentir necessidade de evento novo **para** e **escala**: abre uma proposta de ADR descrevendo (a) o evento, (b) o payload, (c) emissor, (d) consumidores, (e) por que os 10 eventos existentes não cobrem o caso. A lista da §3 só muda por ADR aprovado.

---

## 4. Resumo executivo

- Uma única árvore de estado: `LaserCAD.app.state`, com 12 campos, formas fixadas em §1.1.
- Mutação centralizada em `LaserCAD.app.state.*`; leitura livre; ninguém atribui direto de fora.
- 10 eventos canônicos no bus, listados na §3. Fim da história até que um ADR diga o contrário.
- Comandos do documento (entities/selection) passam por `core.document.commands` + `core.document.history` antes de virar mutação.
- Ângulos no estado em radianos; UI converte de/para graus na borda.
- Tudo inspecionável em `window.LaserCAD.app.state` no DevTools (ver `specs/_conventions/namespace.md` §4).
