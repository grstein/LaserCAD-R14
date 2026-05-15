# ui.toolbar

## 1. Responsabilidade

Renderizar a coluna vertical de 40px de largura com os 10 botoes de ferramenta (ícones SVG inline), gerenciar estados visuais (inativo, hover, ativo, desabilitado), emitir `tool:request` ao click e refletir `tool:armed`/`tool:cancel` no estado visual. Sprint 1: apenas `select` responde; demais botoes ficam desabilitados.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus` (`on`, `off`, `emit`)
  - `window.LaserCAD.app.state` (leitura de `activeTool` para estado visual inicial)
- ordem de carga: posicao #22 em `specs/_conventions/namespace.md`. Carrega depois de `bus`/`state` e antes de `bootstrap`.

## 3. API publica

```js
window.LaserCAD.ui.toolbar = {
  /**
   * Monta a toolbar dentro do elemento host (#toolbar-host).
   * Idempotente: chamadas subsequentes nao re-montam.
   * @param {HTMLElement} host - Container previsto pelo index.html.
   * @returns {void}
   */
  mount(host) {
    /* ... */
  },

  /**
   * Forca a re-leitura do state e atualiza o estado visual de todos os botoes.
   * Util apos eventos como tool:armed/tool:cancel. Normalmente chamado pelos
   * proprios subscribers registrados no mount().
   * @returns {void}
   */
  refresh() {
    /* ... */
  },
};
```

### 3.1 Lista de botoes (Sprint 1)

A ordem visual segue design.md L131–152, com separador antes de `select`:

| #   | toolId      | Glifo conceitual        | Atalho | Estado Sprint 1 |
| --- | ----------- | ----------------------- | ------ | --------------- |
| 1   | `line`      | `\` diagonal            | `L`    | `disabled`      |
| 2   | `polyline`  | zigzag de 3 segmentos   | `P`    | `disabled`      |
| 3   | `rect`      | quadrado vazado         | `R`    | `disabled`      |
| 4   | `circle`    | circulo + ponto central | `C`    | `disabled`      |
| 5   | `arc`       | arco 180° + endpoints   | `A`    | `disabled`      |
|     | _separador_ |                         |        |                 |
| 6   | `select`    | seta cursor diagonal    | `S`    | **`enabled`**   |
| 7   | `trim`      | tesoura                 | `T`    | `disabled`      |
| 8   | `extend`    | linha + seta            | `E`    | `disabled`      |
| 9   | `move`      | cruz com 4 setas        | `M`    | `disabled`      |
| 10  | `delete`    | × diagonal              | `Del`  | `disabled`      |

### 3.2 Formato de cada botao

Cada botao eh um `<button>` HTML (nao SVG `<g>`), 40px de largura por 32px de altura (design.md L122 declara 40×32; o icone interno eh 24×24 viewBox dentro de 18×18 efetivos com stroke 1.5px — design.md L302–306). Atributos:

- `data-tool-id="<toolId>"` para selecao.
- `aria-label="<Nome> (<atalho>)"` (ex.: `"Select (S)"`).
- `disabled` quando state visual eh `disabled`.
- Filho `<svg viewBox="0 0 24 24" aria-hidden="true">` com o glifo (referencia ao asset SVG inline — implementacao em WS-B/render ou inline literal no arquivo da toolbar; cada workstream decide a fonte do glifo desde que cumpra os parametros de design.md L302–306).
- Sem texto visivel. Tooltip nativo via `title="<Nome> (<atalho>)"` aparece apos ~400ms de hover (design.md L138, controlado pelo navegador via atributo `title`).

## 4. Invariantes e tolerancias

- **4 estados visuais.** Conforme design.md L124–129:
  - Inativo: cor de stroke `--text-secondary` (#8E7CB8) sobre fundo `--bg-chrome`.
  - Hover: stroke `--laser-glow` (#9D4DFF), fundo `--bg-elevated`.
  - Ativo: stroke `--text-primary` (#E8DDFF), fundo `--laser-dim` (#3D0099), barra vertical de 2px `--laser-450` (#6E00FF) na borda esquerda.
  - Desabilitado: stroke `--text-disabled` (#4A3E66), sem hover, sem cursor pointer, click ignorado.
- **Coluna 40px exatos.** Estilo CSS fixa `width: 40px`. Cada botao eh `40px × 32px`. Sem padding lateral assimetrico que altere a largura efetiva.
- **Apenas `select` clickavel.** Botoes `disabled` na Sprint 1 nao emitem `tool:request`. Visualmente cinza-disabled. Hover sobre eles **nao** muda cor.
- **Tooltip via atributo `title`.** Nao usar tooltip customizado animado — o navegador entrega tooltip nativo apos hover sustentado (latencia ~400ms eh comportamento padrao do SO).
- **Sincronia com `state.activeTool`.** Apos `tool:armed`, o botao correspondente recebe estado `ativo`; o anterior volta para `inativo` ou `disabled`. Apos `tool:cancel`, retorna para o que diz `state.activeTool` (em Sprint 1, sempre `'select'` armado de novo, conforme `select-tool.md`).
- **Sem icones decorativos do app.** Design.md L116: nada de logo, branding ou separador grafico alem do `<hr>` simples entre o botao 5 (arc) e o botao 6 (select), em `--border-subtle`.
- **Acessibilidade.** Cada botao recebe `aria-label` distinto e eh focavel via `Tab` (acessibilidade descrita em design.md L339: `Tab` percorre toolbar → command line → status bar toggles → menubar).

## 5. Exemplos de uso

```js
// O bootstrap chama:
LaserCAD.ui.toolbar.mount(document.getElementById('toolbar-host'));

// Clique no botao "select" produz:
// 1) button.click() nativo
// 2) handler interno -> LaserCAD.bus.emit('tool:request', { toolId: 'select' })
// 3) tool-manager confirma -> emit 'tool:armed'
// 4) handler de tool:armed na toolbar -> chama refresh()
// 5) botao 'select' recebe classe/estilo 'ativo'

// Forcar refresh manual no DevTools:
LaserCAD.ui.toolbar.refresh();

// Observar clique no botao "line" (disabled em Sprint 1):
// nenhum evento eh emitido. button.disabled === true.

// Inspecionar botoes:
document.querySelectorAll('#toolbar-host button[data-tool-id]').length;
// → 10

document.querySelector('#toolbar-host button[data-tool-id="select"]').disabled;
// → false

document.querySelector('#toolbar-host button[data-tool-id="line"]').disabled;
// → true
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Render correto.** Apos `app:ready`, `document.querySelectorAll('#toolbar-host button[data-tool-id]').length === 10`. O `select` esta `enabled`; os outros 9 estao `disabled`.
2. **Largura 40px.** `getComputedStyle(document.getElementById('toolbar-host')).width === '40px'` (ou `getBoundingClientRect().width === 40`).
3. **Click em `select` emite `tool:request`.** Subscriber `LaserCAD.bus.on('tool:request', console.log)` ouve `{ toolId: 'select' }` apos click. Click em qualquer outro botao nao produz nada.
4. **Estado ativo visual.** Apos `LaserCAD.bus.emit('tool:request', { toolId: 'select' })` -> recebe `tool:armed` -> botao `select` tem fundo `--laser-dim` (verificavel via `getComputedStyle` ou via `classList.contains('is-active')`).
5. **Hover em disabled nao muda cor.** Mover o mouse sobre `line` (disabled): cor permanece `--text-disabled`. Mover sobre `select`: cor de stroke vira `--laser-glow`.
6. **Tooltip aparece.** Hover sustentado por ~1s sobre `select` exibe tooltip nativo `Select (S)`. Conferir pelo `getAttribute('title')` que vale `Select (S)`.
7. **`aria-label` presente em todos.** Loop em DevTools: `[...document.querySelectorAll('#toolbar-host button[data-tool-id]')].map(b => b.getAttribute('aria-label'))` retorna 10 strings nao-vazias.

## 7. Notas de implementacao

- Plan.md L196–200 reconhece `ui/toolbar.js` como modulo de chrome — pode tocar DOM HTML.
- Design.md L120–155 eh a fonte autoritativa do visual. Ler integralmente antes de implementar.
- Design.md L127 fixa o estado "Ativo" com barra de 2px na esquerda — implementar via pseudo-elemento `::before` ou borda CSS para evitar mudar o layout.
- Design.md L138: tooltip apos 400ms eh expectativa de comportamento, nao especificacao do navegador — o atributo `title` entrega isso "de graca", embora alguns navegadores variem o delay. Tooltip customizado pode entrar em uma sprint posterior se necessario.
- O glifo SVG inline pode ser duplicado entre toolbar/menubar/cursor; design.md L304 obriga estilo unico (stroke 1.5px, sem fill). Reuso futuro pode ser feito via `<symbol>` em `<defs>` mas nao eh obrigatorio na Sprint 1.
- Plan.md L223 e design.md L321–331: o estado visual `ativo` da toolbar reflete `state.activeTool` quando `state.toolState in ('armed','preview')` — em `cancel` o botao desativa por 1 frame e volta. Sprint 1 simplifica: refresh integral em qualquer `tool:armed`/`tool:cancel`.
- `tool:request` eh um pedido, nao um fato (state-contract.md §3.1). A toolbar nao muda visual ao clicar; espera o `tool:armed` para refletir. Isso garante consistencia se o `tool-manager` rejeitar a transicao.
