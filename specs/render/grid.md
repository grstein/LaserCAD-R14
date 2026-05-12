# render/grid

## 1. Responsabilidade
Desenhar o grid (minor 1 mm, major 10 mm) e os eixos X/Y do documento dentro dos grupos `#grid` e `#axes` do `<svg>` raiz, aplicando a regra de decimacao por zoom para evitar grid ilegivel.

## 2. Dependencias
- runtime:
  - `window.LaserCAD.render.svgRoot` (acessa `getLayer('grid')` e `getLayer('axes')`, `getRoot()`)
  - `window.LaserCAD.render.camera` (le `get()` para conhecer `zoom`, `viewportW/H`)
  - `window.LaserCAD.app.state` (le `documentBounds`, `toggles.grid`)
  - `window.LaserCAD.bus` (consome `camera:changed`, `viewport:resized`, `toggle:changed`)
- ordem de carga: depois de `render.svgRoot`, `render.camera`, `bus`; antes de `render.entityRenderers`, `render.overlays` (`namespace.md` linha 128).

## 3. API publica

Funcoes anexadas a `window.LaserCAD.render.grid`.

```js
/**
 * Inicializa subscribers do grid (camera:changed, viewport:resized, toggle:changed)
 * e dispara o primeiro render. Chamado uma vez por app.bootstrap.
 */
window.LaserCAD.render.grid.init();

/**
 * Reconstroi os filhos dos grupos #grid e #axes com base no estado atual
 * (camera + documentBounds + toggles.grid). Idempotente.
 */
window.LaserCAD.render.grid.render();

/**
 * Liga ou desliga a visibilidade do grid (sem desmontar — apenas display).
 * Chamado em resposta a toggle:changed { name: 'grid' }.
 * @param {boolean} visible
 */
window.LaserCAD.render.grid.setVisible(visible);
```

## 4. Invariantes e tolerancias

### 4.1 Tamanhos e cores

| Camada | Passo (mm) | Cor (CSS var) | Hex | Forma | Largura |
|---|---:|---|---|---|---:|
| Grid minor | 1 | `--grid-minor` | `#1E1232` | Pontos | 1 px |
| Grid major | 10 | `--grid-major` | `#2E1A4D` | Linhas continuas | 0.5 px |
| Eixos X / Y (em 0,0) | — | `--grid-axis` | `#5B2DD1` | Linhas continuas | 1 px |

(`design.md` L23-L26 e L164-L166).

### 4.2 Regra de decimacao por zoom

Detec por **passo projetado em tela**: `stepPx = stepMm * pxPerMm`, onde `pxPerMm = (viewportW / viewBoxWidth)` — equivalentemente `zoom` × constante de calibracao. Implementacao pratica: medir `viewBox` atual e converter via `getScreenCTM()` (ja garantida em `render.camera`).

- **Minor escondido se `stepMinorPx < 6` px** (1 mm projetado em menos de 6 px = poluicao visual).
- **Major escondido se `stepMajorPx < 4` px** (10 mm projetado em menos de 4 px = grid colapsado).
- Eixos X/Y nunca somem por decimacao — sao referencia de origem; so somem se `toggles.grid === false`.

### 4.3 Outras invariantes

- O grid e desenhado **apenas** dentro da janela visivel: nao percorrer todo o documento ilimitado, e sim o retangulo world coberto pelo `viewBox` corrente.
- Pontos do minor podem ser `<circle r="0.5" />` em world space ou `<rect width="1" height="1"/>` — implementacao livre desde que o resultado visual seja 1 px ponto solido cor `--grid-minor`.
- Major em `<line>` com `stroke="var(--grid-major)" stroke-width="0.5"`. Eixos em `<line>` com `stroke="var(--grid-axis)" stroke-width="1"`.
- `toggles.grid === false` (`F7` ou clique na statusbar) esconde **todos** os filhos de `#grid` e `#axes` (CSS `display:none` no grupo, nao remocao de DOM).
- Re-render imediato — nao debounced — em `camera:changed` e `viewport:resized`. O custo cabe no MVP (grid e simples e o motor SVG do navegador absorve).
- Esse modulo **nao** muta `state` e **nao** ouve pointer events.

## 5. Exemplos de uso

```js
// Em app/bootstrap.js, apos render.svgRoot.mount():
window.LaserCAD.render.grid.init();

// Toggle programatico via bus (caminho oficial):
window.LaserCAD.bus.emit('toggle:changed', { name: 'grid', value: false });
// → render.grid.setVisible(false) e chamado pelo subscriber interno

// Inspecionar geometria desenhada
document.querySelectorAll('#grid > *').length; // depende do zoom; > 0 com zoom 1
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html` por duplo-clique: dentro do viewport e visivel um grid pontilhado fino (1 mm) sobre linhas mais fortes a cada 10 mm; duas linhas roxo-mais-claras (`#5B2DD1`) cruzam-se no (0,0).
2. Apertar `F7` (ou clicar em "GRID" na status bar): o grid e os eixos somem; apertar de novo: voltam — sem reload.
3. Com a roda do mouse, dar zoom-out continuo: a partir do momento em que 1 mm passa a ocupar < 6 px na tela, os pontos minor desaparecem mas o major continua visivel; com mais zoom-out, quando 10 mm < 4 px, o major tambem desaparece; os eixos permanecem.
4. Com zoom-in extremo, conferir que o grid minor reaparece automaticamente.
5. No DevTools, `getComputedStyle(document.querySelector('#grid > line, #grid > circle')).stroke` (ou `fill`) retorna o valor resolvido de `--grid-minor` (`rgb(30, 18, 50)`).
6. `document.querySelector('#axes')` contem exatamente 2 `<line>` (eixo X e eixo Y), nao mais.

## 7. Notas de implementacao

- Tokens de cor estao listados em `design.md` L17-L39 e fixados em `assets/css/theme.md`. Este modulo **referencia** as custom properties via `stroke="var(--grid-minor)"` (etc.), nao recodifica hex.
- A formula `stepPx = stepMm * pxPerMm` resolve a decimacao sem chamar `getScreenCTM()` em loop apertado: basta consultar `state.camera.zoom` e `state.documentBounds` (ou `viewportW`/`viewBox.w`).
- Limites duros 6 px (minor) e 4 px (major) foram escolhidos para preservar legibilidade em monitores 1×; em DPI maior o usuario percebe pontos mais densos antes do limite — aceitavel no MVP.
- Eixos X/Y a 1 px (`design.md` L25, L165) sao a unica geometria de grid que pode ficar mais grossa que o passo proprio: e referencia, nao decoracao.
- O grid e **nao-exportavel**: vive em `#grid`/`#axes`, fora do `#entities`. O exportador (`io.export-svg`, futuro) so serializa `#entities`.
- A unica fonte de visibilidade legitima e `state.toggles.grid` via `toggle:changed`; o modulo nao oferece atalho proprio.
