# render/svg-root

## 1. Responsabilidade
Criar e manter o elemento `<svg>` raiz do viewport, com `viewBox` em mm e a ordem fixa de grupos de pintura (fundo → frente), expondo cada grupo nomeado para os demais modulos de render.

## 2. Dependencias
- runtime:
  - `window.LaserCAD.app.state` (le `documentBounds`, `camera`)
  - `window.LaserCAD.bus` (consome `viewport:resized`, `camera:changed`; nao emite eventos canonicos proprios)
  - `window.LaserCAD.render.camera` (consulta `get()` para projetar a janela visivel ao calcular o `viewBox`)
- ordem de carga: depois de `app.state`, `bus`, `render.camera`; antes de `render.grid`, `render.entityRenderers`, `render.overlays` (`specs/_conventions/namespace.md` linha 127).

## 3. API publica

Tudo sob `window.LaserCAD.render.svgRoot`.

```js
/**
 * Cria o <svg> raiz dentro do host informado (#viewport-host) e injeta os grupos
 * de pintura na ordem canonica. Idempotente: chamada subsequente reaproveita o no.
 * @param {HTMLElement} hostEl - elemento DOM que recebera o <svg> (filho unico)
 * @returns {SVGSVGElement}
 */
window.LaserCAD.render.svgRoot.mount(hostEl);

/**
 * Retorna o <svg> raiz ja montado, ou null se mount() ainda nao rodou.
 * @returns {SVGSVGElement|null}
 */
window.LaserCAD.render.svgRoot.getRoot();

/**
 * Acesso direto a um dos <g> nomeados; usado pelos demais modulos render.* para
 * apendar/limpar filhos da sua camada.
 * @param {'grid'|'axes'|'entities'|'preview'|'overlays'|'snaps'} layerId
 * @returns {SVGGElement}
 */
window.LaserCAD.render.svgRoot.getLayer(layerId);

/**
 * Atualiza o atributo viewBox com base no documentBounds e camera atuais.
 * Chamada como reacao a viewport:resized e camera:changed.
 */
window.LaserCAD.render.svgRoot.refreshViewBox();

/**
 * Atualiza as dimensoes CSS (width/height em pixel) do <svg> para preencher o host.
 * Chamada como reacao a viewport:resized.
 * @param {number} wPx
 * @param {number} hPx
 */
window.LaserCAD.render.svgRoot.setPixelSize(wPx, hPx);
```

### 3.1 Estrutura DOM resultante

```html
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <g id="grid"></g>
  <g id="axes"></g>
  <g id="entities"></g>
  <g id="preview"></g>
  <g id="overlays"></g>
  <g id="snaps"></g>
</svg>
```

## 4. Invariantes e tolerancias

- O atributo `xmlns="http://www.w3.org/2000/svg"` e **obrigatorio** no `<svg>` raiz (`plan.md` L9, L274, L293).
- Os filhos do `<svg>` aparecem **exatamente** nesta ordem (fundo → frente): `#grid`, `#axes`, `#entities`, `#preview`, `#overlays`, `#snaps`. Nenhum outro filho direto do `<svg>`.
- `viewBox` esta sempre em **mm**: `min-x min-y w h` derivados da camera (`cx`, `cy`, `zoom`) e do tamanho do viewport. Dimensoes CSS do `<svg>` (`width`/`height` em pixel) sao independentes e reflexivas do host (`plan.md` L9, L26).
- O `<svg>` nunca recebe `style="background"`; o fundo `--bg-canvas` e definido pelo host (`#viewport-host`) via CSS (`design.md` L163).
- Stroke padrao do `<svg>` nao e setado — cada grupo controla o seu (regra "fill=none" cabe ao exportador, nao a este modulo).
- Acesso a `getLayer('id-invalido')` lanca `Error` legivel; ids fora da lista de 6 sao proibidos.
- Esse modulo **nao** ouve pointer events — quem trata input e `tools` / `ui` / `render.overlays`.
- Esse modulo **nao** muta `state`; apenas le.

## 5. Exemplos de uso

```html
<!-- index.html: o host vazio que recebe o <svg> -->
<main id="viewport-host"></main>
```

```js
// app/bootstrap.js, depois do DOMContentLoaded:
const host = document.getElementById('viewport-host');
const svg = window.LaserCAD.render.svgRoot.mount(host);

// render/grid.js apende seus filhos no grupo grid:
const gridLayer = window.LaserCAD.render.svgRoot.getLayer('grid');
gridLayer.appendChild(/* ...elementos SVG do grid... */);

// inspecao no DevTools:
window.LaserCAD.render.svgRoot.getRoot().getAttribute('viewBox');
// → "0 0 128 128" para documento 128×128 mm com zoom 1 centrado
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html` por duplo-clique; no DevTools rodar `document.querySelector('#viewport-host > svg')` retorna um `SVGSVGElement` com atributo `xmlns="http://www.w3.org/2000/svg"`.
2. `Array.from(document.querySelector('#viewport-host > svg').children).map(c => c.id)` retorna exatamente `["grid","axes","entities","preview","overlays","snaps"]` nessa ordem.
3. Redimensionar a janela do navegador: o atributo `viewBox` muda em resposta a `viewport:resized` (verificavel inspecionando o atributo no DevTools); a largura/altura CSS do `<svg>` acompanham o tamanho do `#viewport-host`.
4. `window.LaserCAD.render.svgRoot.getLayer('snaps').parentNode === window.LaserCAD.render.svgRoot.getRoot()` retorna `true`.
5. Chamar `window.LaserCAD.render.svgRoot.getLayer('foo')` no console lanca `Error` com mensagem clara (nao retorna `null` silenciosamente).

## 7. Notas de implementacao

- Elementos SVG devem ser criados com `document.createElementNS('http://www.w3.org/2000/svg', 'g')` (`plan.md` L19); nao usar `document.createElement('g')`, que cria HTMLUnknownElement.
- `viewBox` em mm: ADR 0001 §2; `width`/`height` da exportacao tambem em mm, mas a definicao desses atributos de export e responsabilidade de `io.export-svg`, nao deste modulo.
- A ordem de grupos serve para o pintor `z-order` natural do SVG (sem `z-index`): `#grid` fica atras de tudo, `#snaps` na frente de tudo (`design.md` L174–185 trata os snap markers como "flutuando sobre o cursor").
- `#preview` carrega geometria tracejada da ferramenta ativa (`design.md` L168, L329).
- `#overlays` carrega crosshair, label de coordenadas e marcadores nao-snap (`design.md` L158–159, L184–185).
- O background `--bg-canvas` do viewport e pintado pelo host via CSS (`assets/css/app.md`), nao por um `<rect>` filho do `<svg>` — evita virar geometria exportavel.
