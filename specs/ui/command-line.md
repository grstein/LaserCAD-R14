# ui.commandLine

## 1. Responsabilidade

Renderizar e operar a command line do LaserCAD R14: 3 linhas mono (historico de 2 linhas + input ativo na 3a), capturar foco automatico em entrada alfanumerica fora de inputs, parsear o subset de comandos da Sprint 1 e emitir os eventos canonicos (`command:submit`, `command:error`, `tool:request`, `toggle:changed`).

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus`
  - `window.LaserCAD.app.state` (le `commandHistory`, `commandInput`, `activeTool`, `toolState`; muta apenas via setters `setCommandInput`, `pushCommandHistory`)
- ordem de carga: posicao #23 em `specs/_conventions/namespace.md`. Carrega depois de `state`/`bus`, antes de `bootstrap`.

## 3. API publica

```js
window.LaserCAD.ui.commandLine = {
  /**
   * Monta a command line dentro do host (#commandline-host).
   * Cria o input de 3 linhas, registra subscribers de bus e listener global
   * de teclado (replicando o R14: alfanumerico foca aqui).
   * @param {HTMLElement} host
   * @returns {void}
   */
  mount(host) { /* ... */ },

  /**
   * Da foco ao input ativo (linha 3). Util em pos-`app:ready`.
   * @returns {void}
   */
  focus() { /* ... */ },

  /**
   * Limpa o input ativo e a linha de prompt (volta para o estado pos-cancel:
   * "Command: _"). Nao mexe no historico.
   * @returns {void}
   */
  clear() { /* ... */ },

  /**
   * Encaminha uma linha bruta para o parser, exatamente como se o usuario
   * tivesse digitado e apertado Enter. Util para testes e o atalho repete-ultimo.
   * @param {string} raw
   * @returns {void}
   */
  submit(raw) { /* ... */ }
};
```

### 3.1 Forma do payload `command:submit`

```js
{
  raw: '<string exata digitada>',
  parsed: <object | null>
}
```

`parsed` segue uma das formas abaixo (extensivel em sprints futuras; congelado para Sprint 1):

```js
// Pedido de troca de ferramenta:
{ kind: 'tool', toolId: 'line' | 'polyline' | 'rect' | 'circle' | 'arc'
                       | 'select' | 'trim' | 'extend' | 'move' | 'delete' }

// View/zoom/pan:
{ kind: 'view', action: 'pan' | 'zoomIn' | 'zoomOut' | 'zoomExtents' }

// Toggle:
{ kind: 'toggle', name: 'snap' | 'grid' | 'ortho' }

// Coordenada absoluta:
{ kind: 'pointAbsolute', x: number, y: number }      // em mm

// Coordenada relativa:
{ kind: 'pointRelative', dx: number, dy: number }    // em mm
```

Quando o parser nao reconhece a linha, `parsed === null` e a command line emite `command:error` em vez de `command:submit`.

## 4. Invariantes e tolerancias

- **Layout 3 linhas, mono.** Tipografia `--font-mono` 12px, line-height 1.0 (design.md L74). Cada linha 22px. Total 66px (design.md L111).
- **Cores conforme design.md L211–212:**
  - Prompt ativo (linha 2 ou 3 com prompt): `--text-primary` (#E8DDFF), peso 600.
  - Historico (linha 1; eventual linha 2 quando ja virou historico): `--text-secondary` (#8E7CB8), peso 400.
  - Erros: `--status-error` (#FF4D6D), prefixo `! ` literal.
- **Foco automatico (design.md L206).** Listener global em `document` para `keydown`: se a tecla eh alfanumerica (`/^[a-zA-Z0-9]$/` para `e.key`), o `event.target` nao eh um `<input>`/`<textarea>`/`[contenteditable]`, e nenhum modificador (Ctrl/Meta) esta ativo, entao o input da command line recebe foco **antes** de processar a tecla, e a tecla eh injetada no campo. Tecla nao alfanumerica nao rouba foco.
- **Confirmacao (design.md L207–L208).** `Enter` e `Espaco` enquanto o foco esta no input confirmam a linha atual: chamam `submit(raw)`.
- **Cancelamento (design.md L209).** `Esc` no input limpa o campo, emite `tool:cancel` (apenas se ha ferramenta ativa em `armed`/`preview`), exibe `*Cancel*` por 1 frame e volta a "Command: _". Note: `tool:cancel` na lista canonica eh emitido pelo `tool-manager`, nao pela commandLine. Em vez disso, a commandLine emite `tool:request` com `{ toolId: 'select' }` ou — preferencialmente — apenas dispara um pedido que o `tool-manager` resolva. **Decisao de Sprint 1:** o Esc na commandLine chama uma API do `tool-manager` (`LaserCAD.tools.toolManager.cancel()`) que emite `tool:cancel` corretamente. Isso evita inventar evento novo.
- **Historico ↑/↓ (design.md L210).** Quando o input esta vazio e o usuario pressiona `↑`, le `state.commandHistory[k]` (k cresce do mais recente). `↓` decrementa k. Quando o input nao esta vazio, ↑/↓ nao navegam historico (comportamento padrao de input).
- **Cap do historico.** N maximo definido em `app.config` (sugerido 50, conforme state-contract.md §1.1). Quando atinge, o mais antigo eh descartado por `state.pushCommandHistory`.
- **Caret piscando.** Sprint 1 entrega caret nativo (`<input>` ou `<textarea>` com `caret-color: --laser-450`). Respeitar `@media (prefers-reduced-motion: reduce)`: caret estatico (design.md L340).
- **Sem auto-complete.** `autocomplete="off"`, `spellcheck="false"`, `autocorrect="off"`.

### 4.1 Parser de Sprint 1

A entrada eh **case-insensitive**. Trim de whitespace antes/depois antes do match.

#### 4.1.1 Comandos reconhecidos e executados

| Entrada (regex case-insensitive) | parsed             | Acao colateral                                  |
|----------------------------------|--------------------|-------------------------------------------------|
| `pan`                            | `{kind:'view', action:'pan'}`         | emite `command:submit`                |
| `zoom\s+in` ou `zi`              | `{kind:'view', action:'zoomIn'}`      | emite `command:submit`                |
| `zoom\s+out` ou `zo`             | `{kind:'view', action:'zoomOut'}`     | emite `command:submit`                |
| `zoom\s+extents` ou `ze`         | `{kind:'view', action:'zoomExtents'}` | emite `command:submit`                |
| `grid`                           | `{kind:'toggle', name:'grid'}`        | dispara `toggle:changed` via state.setToggle (consumido pela statusbar/render.grid) |
| `snap`                           | `{kind:'toggle', name:'snap'}`        | idem                                  |
| `ortho`                          | `{kind:'toggle', name:'ortho'}`       | idem                                  |

Para `grid`/`snap`/`ortho`, a commandLine **inverte** o valor atual (`state.toggles[name]`) chamando `LaserCAD.app.state.setToggle(name, !state.toggles[name])`. O setter emite `toggle:changed` conforme state-contract.md §2.3.

#### 4.1.2 Pedidos de ferramenta (Sprint 1: apenas `select` arma de fato)

| Entrada                             | parsed                                | Acao                                           |
|-------------------------------------|---------------------------------------|------------------------------------------------|
| `line` ou `l`                       | `{kind:'tool', toolId:'line'}`        | emite `command:error` (`! Not available in Sprint 1`) |
| `polyline` ou `p`                   | `{kind:'tool', toolId:'polyline'}`    | idem                                           |
| `rect` ou `r`                       | `{kind:'tool', toolId:'rect'}`        | idem                                           |
| `circle` ou `c`                     | `{kind:'tool', toolId:'circle'}`      | idem                                           |
| `arc` ou `a`                        | `{kind:'tool', toolId:'arc'}`         | idem                                           |
| `select` ou `s`                     | `{kind:'tool', toolId:'select'}`      | emite `tool:request` `{toolId:'select'}` (ja eh a ativa, mas mantem fluxo) |
| `trim` ou `t`                       | `{kind:'tool', toolId:'trim'}`        | emite `command:error`                          |
| `extend` ou `e`                     | `{kind:'tool', toolId:'extend'}`      | idem                                           |
| `move` ou `m`                       | `{kind:'tool', toolId:'move'}`        | idem                                           |
| `delete` (sem alias `d`)            | `{kind:'tool', toolId:'delete'}`      | idem                                           |

**Importante:** para os 9 toolIds nao disponíveis, **ainda assim** o parser produz `parsed` valido (objeto) e emite **`command:error`** com `message: '! Not available in Sprint 1'`. Nao emite `command:submit` nesses casos. Isso valida o pipeline parser/erro sem implementar a ferramenta.

#### 4.1.3 Coordenadas

Formato absoluto: `/^(-?\d+(?:\.\d+)?),\s*(-?\d+(?:\.\d+)?)$/` → `parsed = {kind:'pointAbsolute', x, y}`.
Formato relativo: `/^@(-?\d+(?:\.\d+)?),\s*(-?\d+(?:\.\d+)?)$/` → `parsed = {kind:'pointRelative', dx, dy}`.

**Sprint 1:** ambos sao reconhecidos sintaticamente, **mas** a acao colateral eh emitir `command:error` com `message: '! Not available in Sprint 1'`. O parser valida o formato (testado por `state-contract.md` indiretamente; consumo real em Sprint 3 quando ferramentas de desenho entrarem).

#### 4.1.4 Erro de parse

Qualquer entrada que nao bate em nenhum padrao acima → `parsed = null` e emite `command:error` com `message: '! Unknown command: ' + raw`.

#### 4.1.5 Linha vazia

`Enter` ou `Espaco` com input vazio: em R14 isso repete o ultimo comando. Sprint 1: se `commandHistory` nao esta vazio, chama `submit(commandHistory[0])` recursivamente. Se vazio, nao faz nada.

### 4.2 Renderizacao das 3 linhas

| Linha | Conteudo                                                                       |
|-------|--------------------------------------------------------------------------------|
| 1     | Eco do penultimo comando ou ultimo resultado (ex.: `Command: line`)            |
| 2     | Prompt da ferramenta ativa, gerado pelo `tool-manager` ou ferramenta corrente. Sprint 1: `Command:` ou `LINE  Specify first point:` (prompts estaticos definidos em select-tool.md/tool-manager.md). |
| 3     | Input vivo: `<prompt corrente> <input do usuario><caret>`                      |

A commandLine consome `tool:armed` para atualizar a linha 2 com o prompt da ferramenta armada (recuperado de `tool-def.prompt` ou similar; ver `tool-manager.md`). Para `select` em Sprint 1, o prompt eh `Select objects:` (mesmo que clique nao faca nada — design.md L329 mostra padrao).

## 5. Exemplos de uso

```js
// Via teclado: usuario digita "snap" + Enter
// 1) ui.commandLine.submit('snap')
// 2) parser → { kind:'toggle', name:'snap' }
// 3) chama LaserCAD.app.state.setToggle('snap', !state.toggles.snap)
// 4) state emite 'toggle:changed' { name:'snap', value:false }
// 5) ui.commandLine emite 'command:submit' { raw:'snap', parsed:{...} }
// 6) Linha 1 vira "snap" (eco), linha 3 esvazia.

// Via DevTools, simulando entrada:
LaserCAD.ui.commandLine.submit('zoom extents');
LaserCAD.ui.commandLine.submit('line');         // -> command:error '! Not available in Sprint 1'
LaserCAD.ui.commandLine.submit('124.5,87.3');   // -> command:error '! Not available in Sprint 1'
LaserCAD.ui.commandLine.submit('@10,0');        // -> command:error '! Not available in Sprint 1'
LaserCAD.ui.commandLine.submit('xyzfoo');       // -> command:error '! Unknown command: xyzfoo'

// Foco programatico:
LaserCAD.ui.commandLine.focus();

// Subscrever erros:
LaserCAD.bus.on('command:error', (p) => console.error(p.message, p.raw));

// Subscrever submissoes:
LaserCAD.bus.on('command:submit', (p) => console.log('submit', p));
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Renderizacao 3 linhas.** Apos `app:ready`, `#commandline-host` contem 3 elementos de linha visiveis com altura total ~66px. Inspecionar: `document.querySelectorAll('#commandline-host > *').length >= 3`.
2. **Foco automatico.** Com foco no `<body>` (clicar fora da chrome), pressionar `s` no teclado: a tecla aparece dentro do input da command line. Pressionar `Tab`: foco sai sem injecao automatica.
3. **Comando `zoom extents`.** Digitar `zoom extents` + Enter. Bus emite `command:submit { raw:'zoom extents', parsed:{kind:'view', action:'zoomExtents'} }`.
4. **Toggle por comando.** `state.toggles.snap === true` antes. Digitar `snap` + Enter. Bus emite `toggle:changed { name:'snap', value:false }`. `state.toggles.snap === false` depois.
5. **Comando de ferramenta nao-Sprint-1.** Digitar `line` + Enter. Bus emite `command:error { raw:'line', message:'! Not available in Sprint 1' }`. Linha 1 da command line mostra o eco em `--text-secondary`; o erro aparece com prefixo `! ` em `--status-error`.
6. **Coordenada absoluta reconhecida sintaticamente.** Digitar `124.5,87.3` + Enter. Bus emite `command:error` com a mensagem `! Not available in Sprint 1`. Confirmar que **nao** eh `! Unknown command:` — ou seja, o parser reconheceu o formato.
7. **Coordenada relativa reconhecida.** `@50,0` + Enter → `command:error` com mensagem de Sprint 1. Idem.
8. **Comando desconhecido.** `xyzfoo` + Enter → `command:error { message:'! Unknown command: xyzfoo' }`.
9. **Esc cancela.** Com input nao vazio, pressionar `Esc`: input esvazia e o `tool-manager` recebe pedido de cancel (verificavel via `LaserCAD.bus.on('tool:cancel', console.log)` se houver ferramenta ativa).
10. **Historico ↑/↓.** Depois de submeter `snap`, `zoom extents`, `grid` (nessa ordem), focar o input vazio, pressionar `↑` 1x: input mostra `grid`. `↑` de novo: `zoom extents`. `↓`: `grid`. `↓`: vazio.
11. **`Espaco` confirma como Enter.** Digitar `snap`, pressionar `Espaco` (sem newline): comando submetido identico a Enter.
12. **Eco em `--text-secondary`.** Apos um comando submetido, a linha 1 (historico) usa cor `--text-secondary`. Conferir via `getComputedStyle`.

## 7. Notas de implementacao

- Design.md L186–219 eh a especificacao base.
- O foco automatico (design.md L206) eh o detalhe mais delicado: o listener tem que executar em fase `capture` ou checar o `event.target` para nao roubar foco de outros inputs (ex.: dialogs futuros, statusbar toggles via teclado). Implementar como `document.addEventListener('keydown', handler, true)` em fase capture.
- O parser pode ser uma tabela de regex testada em ordem. Para Sprint 1, a tabela cabe em 15 entradas.
- `command:error` aparece visualmente, mas tambem eh emitido no bus para que outros consumidores (ex.: futuros logs) reajam.
- Plan.md L223 (maquina `idle/armed/preview/commit/cancel`) eh respeitada via consumo de `tool:armed`/`tool:cancel` para alternar prompts da linha 2. Nao mexer em `state.toolState` diretamente — caminho oficial passa pelo `tool-manager`.
- A regra "Esc → cancel" delega ao `tool-manager` para nao inventar evento. State-contract.md §3 nao tem evento `command:cancel`; usar `LaserCAD.tools.toolManager.cancel()` (API descrita em `specs/tools/tool-manager.md`).
- Design.md L341 (`prefers-reduced-motion: reduce`) eh consultado pela CSS, nao pelo JS, para o caret.
- Plan.md L268–270 (autosave/corrupcao) nao se aplica a Sprint 1 — a commandLine nao persiste nada alem do que `app.state` ja persiste em memoria.
- `aria-live="polite"` pode ser aplicado ao slot de erros (linha 1) para que leitores de tela anunciem mensagens `! ...` (acessibilidade).
- Eventos canonicos usados: `command:submit`, `command:error`, `tool:request`, `tool:armed` (consumo), `tool:cancel` (consumo apenas; emissao oficial vem do `tool-manager`). Nenhum evento fora da lista.
