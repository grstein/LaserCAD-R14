# app.bootstrap

## 1. Responsabilidade

Orquestrar a sequencia exata de inicializacao do LaserCAD R14: validar pre-condicoes do namespace, dimensionar e injetar o `<svg>` raiz, montar grid/axes/overlays, renderizar a chrome (toolbar, menubar, command line, statusbar), registrar atalhos globais, registrar a ferramenta `select` no `tool-manager` e emitir `app:ready`.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.core.geometry.vec2` (e demais entradas listadas em §3.1 abaixo) — validacao obrigatoria
  - `window.LaserCAD.app.state` (setters de viewport, toggles, activeTool)
  - `window.LaserCAD.app.config`, `window.LaserCAD.app.shortcuts`
  - `window.LaserCAD.bus`
  - `window.LaserCAD.render.camera`, `render.svgRoot`, `render.grid`, `render.entityRenderers`, `render.overlays`
  - `window.LaserCAD.tools.toolManager`, `tools.selectTool`
  - `window.LaserCAD.ui.toolbar`, `ui.commandLine`, `ui.statusbar`, `ui.menubar`, `ui.dialogs`
  - DOM: elemento com `id="viewport-host"` (criado por WS-B em `specs/index-html.md`); elementos host da chrome: `#menubar-host`, `#toolbar-host`, `#commandline-host`, `#statusbar-host` (mesma origem).
- ordem de carga: posicao #27 em `specs/_conventions/namespace.md` (penultimo). Carrega depois de toda `ui/*` e antes de `main.js`.

## 3. API publica

```js
window.LaserCAD.app.bootstrap = {
  /**
   * Executa a sequencia de inicializacao. Idempotente: chamadas subsequentes
   * sao no-op (loga aviso). Chamado apenas por src/main.js.
   * @returns {void}
   * @throws {Error} Quando uma pre-condicao critica falha e nao ha caminho de recuperacao.
   */
  start() {
    /* ... */
  },

  /**
   * Renderiza uma mensagem de erro fatal no DOM (ocupando #viewport-host
   * ou o body como fallback) e loga no console. Usado quando uma dependencia
   * obrigatoria falta e o app nao pode prosseguir.
   * @param {string} message
   * @param {Error} [cause]
   * @returns {void}
   */
  showFatalError(message, cause) {
    /* ... */
  },
};
```

### 3.1 Sequencia exata executada por `start()`

A ordem abaixo eh **normativa**. Ela existe para mitigar o risco "ordem de scripts quebrada" descrito em ADR 0001 §3.4.

1. **Guard de re-entrada.** Se ja rodou (flag interno), loga `[bootstrap] already started` e retorna.
2. **Validar presenca dos sub-namespaces de WS-A.** Verificar que existem como funcao/objeto:
   - `LaserCAD.core.geometry.epsilon`, `.vec2`, `.line`, `.circle`, `.arc`, `.project`
   - `LaserCAD.core.document.schema`, `.validators`, `.commands`, `.history`
   - `LaserCAD.app.state` (com setters: `setActiveTool`, `setToolState`, `setCamera`, `setViewportSize`, `setCursor`, `setToggle`, `setCommandInput`, `pushCommandHistory`, `applyCommand`, `setDocumentBounds`)
   - `LaserCAD.app.config`, `LaserCAD.app.shortcuts`
   - `LaserCAD.bus.on`, `.off`, `.emit`
   - **WS-B:** `LaserCAD.render.camera`, `.svgRoot`, `.grid`, `.entityRenderers`, `.overlays`
   - **WS-C (proprio):** `LaserCAD.tools.toolManager`, `.selectTool`; `LaserCAD.ui.toolbar`, `.commandLine`, `.statusbar`, `.menubar`, `.dialogs`
   - Em caso de falta: chama `showFatalError("Missing dependency: <caminho>", err)` e **lanca**.
3. **Validar DOM.** `document.getElementById('viewport-host')` precisa existir; idem para `menubar-host`, `toolbar-host`, `commandline-host`, `statusbar-host`. Falta → `showFatalError` + throw.
4. **Garantir/validar state inicial.** `LaserCAD.app.state` ja foi instanciado na carga do proprio `state.js`. Bootstrap apenas le `state` para sanity-check (ex.: `state.units === 'mm'`, `state.activeTool === 'select'`). Se incoerente, log warning e seguir.
5. **Ler dimensoes do viewport.** `const host = document.getElementById('viewport-host'); const rect = host.getBoundingClientRect(); const w = Math.max(1, Math.round(rect.width)); const h = Math.max(1, Math.round(rect.height));`. Chamar `LaserCAD.app.state.setViewportSize(w, h)`.
6. **Instalar `ResizeObserver` no `#viewport-host`.** A cada mudanca, emitir `viewport:resized` com `{ w, h }` em px e atualizar via `state.setViewportSize`. (O bootstrap eh o emissor canonico de `viewport:resized`, conforme state-contract.md §3 tabela.)
7. **Instanciar `<svg>` raiz e injetar.** Chamar `LaserCAD.render.svgRoot.mount(host)`. Este modulo (responsabilidade WS-B) cria o elemento `<svg>`, define `viewBox` a partir de `state.documentBounds`, e devolve a referencia. Bootstrap nao manipula o SVG diretamente.
8. **Montar camadas de render.** Em ordem: `render.grid.mount()`, `render.entityRenderers.mount()`, `render.overlays.mount()`. Cada um insere o proprio `<g>` no `<svg>` raiz e se inscreve nos eventos relevantes (`camera:changed`, `viewport:resized`). Bootstrap nao se preocupa com a ordem interna alem dessa chamada externa.
9. **Renderizar a chrome (UI).** Em ordem:
   - `LaserCAD.ui.menubar.mount(document.getElementById('menubar-host'))`
   - `LaserCAD.ui.toolbar.mount(document.getElementById('toolbar-host'))`
   - `LaserCAD.ui.commandLine.mount(document.getElementById('commandline-host'))`
   - `LaserCAD.ui.statusbar.mount(document.getElementById('statusbar-host'))`
   - `LaserCAD.ui.dialogs.mount(document.body)` (registra container, nao abre nenhum dialog)
10. **Registrar atalhos globais.** `LaserCAD.app.shortcuts.attach(window)` — instala listener de teclado responsavel por `F3` (snap), `F7` (grid), `F8` (ortho), `Esc` (cancel), letras de ferramenta. Os despachos saem como eventos canonicos (`toggle:changed`, `tool:request`, `tool:cancel`).
11. **Registrar ferramenta `select` no `tool-manager`.** `LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool)`. Ferramentas adicionais ficam para Sprint 3.
12. **Armar a ferramenta inicial.** `LaserCAD.bus.emit('tool:request', { toolId: 'select' })`. O `tool-manager` consome e confirma com `tool:armed`.
13. **Emitir `app:ready`.** `LaserCAD.bus.emit('app:ready', {})` — sinaliza a `ui.*`/`render.*` que podem fazer ajustes pos-load (ex.: focar a command line).
14. **Marcar bootstrap como iniciado** (flag interno) para idempotencia.

### 3.2 Tratamento de erros

- Falta de dependencia em §3.1 passo 2 ou DOM em passo 3 → `showFatalError` com mensagem clara (`Missing: <path>`), DOM ocupado por uma `<div>` overlay em `--status-error` sobre `--bg-canvas`, console mostra stack.
- Excecao em qualquer passo de 5 a 13 eh capturada e re-emitida como `showFatalError`, sem swallowing silencioso.
- `showFatalError` **lanca** depois de exibir, garantindo que `main.js` interrompa.

## 4. Invariantes e tolerancias

- **Idempotencia.** `start()` chamado mais de uma vez nao re-executa a sequencia.
- **Sem mutacao direta do state.** Bootstrap so usa setters publicos: `state.setViewportSize`, etc. Nunca atribui campos diretamente.
- **Nao desenha SVG.** Toda a manipulacao do `<svg>` eh delegada a `render.*`. Bootstrap apenas chama `mount`.
- **Nao implementa matematica.** Coordenadas, conversoes e grids ficam em `core/`, `render/camera`, `render/grid`.
- **Eventos emitidos por bootstrap (e somente por ele) na Sprint 1:** `viewport:resized`, `app:ready`. Tambem emite `tool:request` no passo 12 (permitido — `tool:request` tem multiplos emissores: `ui.toolbar`, `ui.commandLine`, `ui.menubar` per state-contract.md §3; bootstrap eh kick-off equivalente).
- **Nenhum evento fora da lista canonica** de `specs/_conventions/state-contract.md` §3.
- **Sem `setTimeout`/`setInterval`.** Toda a sequencia eh sincrona, exceto o `ResizeObserver` (callback assincrono que so emite eventos depois de inicializado).
- **Pre-condicao DOM:** o `index.html` (WS-B) precisa garantir que os hosts existam **antes** de qualquer script. Se nao existirem, bootstrap aborta — nao tenta cria-los.

## 5. Exemplos de uso

```js
// Chamada normal (feita por main.js):
LaserCAD.app.bootstrap.start();

// Re-chamada manual no DevTools (idempotente):
LaserCAD.app.bootstrap.start();
// [bootstrap] already started

// Inspecionar pos-bootstrap:
LaserCAD.app.state.activeTool;
// → 'select'
LaserCAD.app.state.toolState;
// → 'armed'
LaserCAD.app.state.camera.viewportW;
// → ex.: 1840
document.getElementById('viewport-host').querySelector('svg');
// → <svg viewBox="0 0 128 128" ...>

// Forcar erro fatal:
LaserCAD.app.bootstrap.showFatalError('Test error', new Error('demo'));
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Abertura limpa.** Abrir `index.html` por duplo-clique. Console nao apresenta `Uncaught`. `LaserCAD.app.state` reflete: `units==='mm'`, `activeTool==='select'`, `toolState==='armed'`, `camera.viewportW > 0`, `camera.viewportH > 0`.
2. **`app:ready` foi emitido.** No DevTools, antes de recarregar, executar: `LaserCAD.bus.on('app:ready', () => console.log('ok'))` e depois `F5`. Apos o reload, no console deve constar tanto a flag de idempotencia (`already started` so se chamar de novo) quanto a evidencia indireta: `document.querySelector('#viewport-host svg')` retorna elemento valido.
3. **Resize emite evento.** Diminuir/aumentar a janela do navegador. `LaserCAD.app.state.camera.viewportW`/`H` se atualizam em poucos frames. Subscriber `LaserCAD.bus.on('viewport:resized', console.log)` recebe payloads.
4. **Dependencia ausente, falha legivel.** No DevTools, antes da chamada (hard refresh com cache desabilitado), simular: `delete LaserCAD.core.geometry.vec2; LaserCAD.app.bootstrap.start();` — exibe overlay vermelho `Missing dependency: core.geometry.vec2` e console mostra stack. (Em produto: renomear `vec2.js` no `index.html` reproduz o mesmo cenario.)
5. **Idempotencia.** Chamar `LaserCAD.app.bootstrap.start()` no console depois do load. Resultado: log `[bootstrap] already started`, sem segundo `<svg>`, sem efeito visual.
6. **Ferramenta `select` registrada e armada.** `LaserCAD.tools.toolManager` reconhece `'select'` como ferramenta registrada (interface a definir em `tool-manager.md`); `state.activeTool === 'select'` e `state.toolState === 'armed'`.

## 7. Notas de implementacao

- Mitigacao explicita do risco "Divergencia tela↔documento" (plan.md L269): a leitura de viewport eh feita uma unica vez no bootstrap e o `ResizeObserver` mantem `state.camera.viewportW/H` sincronizado dali em diante.
- Mitigacao do risco "ordem de scripts" (ADR 0001 §3.4 e namespace.md §3.2): o passo 2 detecta falta de qualquer um dos 17+ pontos de presenca antes de qualquer manipulacao de DOM, evitando estouros mais tarde com stack confuso.
- Plan.md L117 ("levantar a base do editor") eh exatamente o que bootstrap executa.
- Plan.md L223 (maquina `idle/armed/preview/commit/cancel`) eh respeitada via passo 12: o `tool:request` sai e o `tool-manager` decide a transicao (`armed` neste caso, conforme `select-tool.md`).
- Design.md L80–118 (4 regioes da chrome) eh refletido no passo 9: cada `mount` ocupa exatamente um dos 4 hosts do DOM.
- Design.md L321–331 (estados de ferramenta) fundamenta o passo 12, que dispara a transicao `idle → armed` ao final do bootstrap.
- A sequencia eh deliberadamente sequencial (sem `Promise.all`) para que erros apontem o passo exato.
- `app.config` (carregado em #13) eh consultado por outros modulos para constantes como cap do `commandHistory` (50) e debounce — bootstrap nao reescreve config; apenas confia que esta presente.
