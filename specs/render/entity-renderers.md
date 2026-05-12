# render/entity-renderers

## 1. Responsabilidade
Mapear cada `Entity` do modelo em um no SVG dentro do grupo `#entities` do `<svg>` raiz, aplicando o estilo de geometria comitada (traco `--laser-450`). Sprint 1 entrega **apenas as assinaturas** — nao ha entidades reais ate Sprint Drawing.

## 2. Dependencias
- runtime:
  - `window.LaserCAD.render.svgRoot` (acessa `getLayer('entities')`)
  - `window.LaserCAD.core.document.schema` (referencia aos `@typedef` `LineEntity`, `CircleEntity`, `ArcEntity`)
  - `window.LaserCAD.core.geometry.vec2` (em renderizadores que precisem manipular pontos antes de serializar)
- ordem de carga: depois de `render.svgRoot`, `core.document.schema`; antes de `render.overlays` (`namespace.md` linha 129).

## 3. API publica

Tudo sob `window.LaserCAD.render.entityRenderers`.

```js
/**
 * @typedef {{ id:string, type:'line', a:{x:number,y:number}, b:{x:number,y:number} }} LineEntity
 * @typedef {{ id:string, type:'circle', c:{x:number,y:number}, r:number }} CircleEntity
 * @typedef {{ id:string, type:'arc',
 *             c:{x:number,y:number}, r:number,
 *             start:number, end:number,        // radianos (state-contract §1.2)
 *             ccw:boolean
 *          }} ArcEntity
 * @typedef {LineEntity|CircleEntity|ArcEntity} Entity
 *
 * @typedef {{
 *   parent: SVGGElement,                       // tipicamente svgRoot.getLayer('entities')
 *   classList?: string[]                       // classes CSS opcionais
 * }} RenderContext
 */

/**
 * Cria/atualiza um <line> filho de ctx.parent representando a entidade.
 * Sprint 1: apenas assinatura; corpo fica em sprints seguintes.
 * @param {LineEntity} entity
 * @param {RenderContext} ctx
 * @returns {SVGLineElement}
 */
window.LaserCAD.render.entityRenderers.renderLine(entity, ctx);

/**
 * Cria/atualiza um <circle> filho de ctx.parent representando a entidade.
 * @param {CircleEntity} entity
 * @param {RenderContext} ctx
 * @returns {SVGCircleElement}
 */
window.LaserCAD.render.entityRenderers.renderCircle(entity, ctx);

/**
 * Cria/atualiza um <path d="M ... A ..."> filho de ctx.parent representando o arco.
 * Mapeia: ponto inicial via M, e arco via comando A (rx ry x-axis-rotation
 * large-arc-flag sweep-flag x y) — plan.md L297.
 * @param {ArcEntity} entity
 * @param {RenderContext} ctx
 * @returns {SVGPathElement}
 */
window.LaserCAD.render.entityRenderers.renderArc(entity, ctx);

/**
 * Dispatcher utilitario: chama o renderer correto baseado em entity.type.
 * @param {Entity} entity
 * @param {RenderContext} ctx
 * @returns {SVGElement}
 */
window.LaserCAD.render.entityRenderers.render(entity, ctx);
```

### 3.1 Mapeamento tipo → tag SVG

| `entity.type` | Tag gerada | Atributos chave |
|---|---|---|
| `line`   | `<line>`   | `x1`, `y1`, `x2`, `y2` (mm) |
| `circle` | `<circle>` | `cx`, `cy`, `r` (mm) |
| `arc`    | `<path>`   | `d="M sx sy A r r 0 large sweep ex ey"` (mm; angulos em rad → endpoints calculados) |

## 4. Invariantes e tolerancias

- Cada no SVG gerado **recebe**: `data-id="<entity.id>"`, classe CSS `entity` e estilo de corte:
  - `stroke="var(--laser-450)"` (hex `#6E00FF`, `design.md` L30, L167)
  - `stroke-width="0.1"` (mm logico; render visual com minimo de 1 px e responsabilidade de CSS de viewport — fora do escopo deste modulo)
  - `fill="none"` (sempre — `plan.md` L298)
  - `stroke-linecap="round"`, `stroke-linejoin="round"` (estetica unica; `design.md` L304)
- Coordenadas sao escritas em **mm**, sem sufixo, sem unidade embutida (ADR 0001 §2; `plan.md` L26).
- Angulos em `ArcEntity` estao em **radianos** (`state-contract.md` §1.2). O renderer converte para endpoints `(sx,sy)` / `(ex,ey)` antes de montar o `d`.
- O arco **deve** usar `<path>` com comando `A` (plan.md L297) — nao decompor em segmentos ou aproximar com bezier.
- `large-arc-flag` derivado de `|end - start|` normalizado em `[0, 2π)`; `sweep-flag` derivado de `ccw`.
- Idempotencia por id: se ja existe um `<*>` com `data-id` igual em `ctx.parent`, atualizar atributos no lugar (nao remover/recriar). Se nao existe, criar e apender.
- Nenhuma chamada a `state` mutavel; renderers sao funcoes puras DOM-side: `(entity, ctx) → SVGElement`.
- **Nao** ha render de selecao/hover/preview aqui — esses estados sao `render.overlays`. Este modulo so trata "geometria comitada".

## 5. Exemplos de uso

```js
// Entidade hipotetica
const line = { id: 'e_1', type: 'line', a: {x:10, y:10}, b: {x:60, y:10} };

// Em algum loop futuro (Sprint Drawing):
const parent = window.LaserCAD.render.svgRoot.getLayer('entities');
window.LaserCAD.render.entityRenderers.render(line, { parent });

// Resultado no DOM:
// <line data-id="e_1" class="entity" x1="10" y1="10" x2="60" y2="10"
//       stroke="var(--laser-450)" stroke-width="0.1" fill="none"
//       stroke-linecap="round" stroke-linejoin="round" />
```

```js
// Arco quarto de circulo (90deg), CCW, centro em (50,50), raio 20
const arc = { id:'e_2', type:'arc', c:{x:50,y:50}, r:20,
              start: 0, end: Math.PI/2, ccw: true };
window.LaserCAD.render.entityRenderers.render(arc, { parent });
// → <path data-id="e_2" class="entity" d="M 70 50 A 20 20 0 0 1 50 70" ... />
```

## 6. Criterios de aceitacao testaveis manualmente

1. No DevTools, `typeof window.LaserCAD.render.entityRenderers.renderLine === 'function'` e analogo para `renderCircle`, `renderArc`, `render`.
2. Sprint 1: como nao ha entidades, `document.querySelector('#entities')` existe e esta vazio; a presenca dos renderers e verificavel por inspecao do namespace (`Object.keys(window.LaserCAD.render.entityRenderers)`).
3. (Sprint Drawing) Apos criar uma linha por comando, `document.querySelector('#entities > line[data-id="e_1"]')` retorna nao-nulo e tem `stroke` resolvido para `rgb(110, 0, 255)`.
4. (Sprint Drawing) Apos criar um arco, o `d` do `<path>` gerado contem exatamente um comando `M` seguido de um comando `A` (regex `/^M [^A]+A [^MZ]+$/`).
5. Renderizar a mesma entidade duas vezes nao duplica nos no DOM: o segundo `render(entity, ctx)` atualiza o existente (mesmo `data-id`).

## 7. Notas de implementacao

- Mapeamento e justificativas: `plan.md` L246 ("`line` vira `<line>`, `circle` vira `<circle>`, arco vira `<path>`") e L297 ("Arcos: exportar como `path` com comando `A` ou equivalente"). O renderer interno reaproveita a mesma forma que o exportador, garantindo consistencia "o que se ve = o que se exporta" (ADR 0001 §1.4).
- Cor de corte `--laser-450`: `design.md` L29-L30, L167-L167 e secao "Paleta laser 450nm".
- `stroke-width` logico `0.1 mm` e fixo no MVP (`design.md` L167; ADR 0001 §2.4); o efeito visual em pixel varia com zoom — a regra "minimo 1 px" e tratada pelo CSS do viewport (vector-effect ou stroke-width minimo), nao por este modulo.
- Sprint 1 deixa a **implementacao** vazia (stubs JSDoc + IIFE com `ns.entityRenderers = { renderLine, renderCircle, renderArc, render }` retornando documentacao "TODO"). Isso satisfaz a ordem de scripts e a inspecao no console sem entregar geometria que nao existe ainda.
- Schema das entidades: detalhado em `core/document/schema.md` (WS-A). Esta spec apenas referencia os `@typedef` por nome.
- Pureza: este modulo nao chama `state.applyCommand` nem emite eventos do bus — e estritamente DOM-side.
