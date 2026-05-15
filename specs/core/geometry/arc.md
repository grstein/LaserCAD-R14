# arc

## 1. Responsabilidade

Definir a entidade arco (porção de círculo) e expor consultas puras: comprimento, ponto médio, pontos extremos (start/end) e bounding box correto mesmo quando o arco cruza eixos cardinais.

## 2. Dependências

- runtime: `window.LaserCAD.core.geometry.vec2`, `window.LaserCAD.core.geometry.epsilon`.
- ordem de carga: depois de `vec2` e `epsilon` (posição 5 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.arc`.

```text
@typedef {{
  type: 'arc',
  center: Vec2,
  r: number,
  startAngle: number,   // radianos
  endAngle: number,     // radianos
  ccw: boolean          // true = sentido anti-horário (CCW), false = horário (CW)
}} ArcEntity

@typedef {{ minX: number, minY: number, maxX: number, maxY: number }} BBox

make(center, r, startAngle, endAngle, ccw)  : ArcEntity   // lança se r <= EPS
startPoint(a: ArcEntity)                     : Vec2
endPoint(a: ArcEntity)                       : Vec2
sweep(a: ArcEntity)                          : number     // ângulo varrido em radianos, sempre 0..2π
length(a: ArcEntity)                         : number     // r * sweep
midpoint(a: ArcEntity)                       : Vec2       // ponto no arco a metade do sweep
bbox(a: ArcEntity)                           : BBox
contains(a: ArcEntity, angle: number)        : boolean    // true se `angle` (rad) está dentro do arco respeitando ccw
```

Contratos pré/pós:

- `make` valida: `center` finito; `r > EPS`; `startAngle` e `endAngle` finitos; `ccw` boolean. Lança caso contrário.
- `startAngle` e `endAngle` **não** precisam estar normalizados em `[0, 2π)`; o módulo normaliza internamente conforme necessário (ex.: para `contains`).
- `sweep` retorna sempre valor em `[0, 2π]`. Arco "completo" representado por `startAngle === endAngle` com `ccw === true` é interpretado como sweep = 0 (não como círculo completo) — para círculo, usar a entidade `circle`.
- `bbox` é **conservador no sentido geométrico exato**: inclui apenas os pontos efetivamente cobertos pelo arco; nunca a caixa do círculo inteiro (a menos que o arco cubra todos os eixos cardinais).

## 4. Invariantes e tolerâncias

- Forma fixa: `{ type: 'arc', center, r, startAngle, endAngle, ccw }`.
- Ângulos em **radianos**, sempre. Conversão de/para graus é responsabilidade exclusiva da camada UI (ADR 0001 §2; plan.md L218, L224).
- Convenção angular: 0 rad aponta para `+X`, `π/2` para `+Y`, `π` para `-X`, `3π/2` para `-Y`. CCW positivo (matemática padrão).
- `ccw === true` significa varrer de `startAngle` para `endAngle` no sentido anti-horário; `ccw === false` no sentido horário.
- `r > EPS` obrigatório.
- Pureza: **sem DOM, sem `app.state`, sem `bus`** (plan.md L222).

## 5. Exemplos de uso

```js
const A = window.LaserCAD.core.geometry.arc;
const PI = Math.PI;

// Quarto de círculo CCW de +X para +Y, centro em (10,10), raio 5
const a = A.make({ x: 10, y: 10 }, 5, 0, PI / 2, true);

A.startPoint(a); // {x: 15, y: 10}
A.endPoint(a); // {x: 10, y: 15}
A.sweep(a); // PI/2
A.length(a); // 5 * PI/2 ≈ 7.854
A.midpoint(a); // ponto em angle = PI/4

A.bbox(a); // {minX:10, minY:10, maxX:15, maxY:15}
// (não cruza 90°,180°,270° internamente)

// Arco que cruza 0° (CCW de 350° para 10°)
const wrap = A.make({ x: 0, y: 0 }, 1, (350 * PI) / 180, (10 * PI) / 180, true);
A.bbox(wrap);
// minX/maxX dominados pelos cantos do arco + ponto cardinal 0° (= (1,0))
// que está DENTRO do arco — então maxX = 1
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.geometry.arc.startPoint(arc.make({x:0,y:0}, 1, 0, Math.PI/2, true))` retorna `{x:1, y:0}` (dentro de `EPS`).
2. `window.LaserCAD.core.geometry.arc.sweep(arc.make({x:0,y:0}, 1, 0, Math.PI/2, true))` retorna `Math.PI/2`.
3. **bbox cruzando 0°:** `arc.bbox(arc.make({x:0,y:0}, 1, -Math.PI/4, Math.PI/4, true))` deve ter `maxX === 1` (o ponto cardinal `(1,0)` está dentro do arco), `maxY ≈ 0.7071`, `minY ≈ -0.7071`, `minX ≈ 0.7071`.
4. **bbox NÃO cobrindo cardinais:** `arc.bbox(arc.make({x:0,y:0}, 1, Math.PI/6, Math.PI/3, true))` (do primeiro quadrante, sem cruzar nenhum eixo) retorna a caixa dos endpoints — `maxX === cos(π/6) ≈ 0.866`, `maxY === sin(π/3) ≈ 0.866`.
5. `arc.contains(arc.make({x:0,y:0}, 1, 0, Math.PI, true), Math.PI/2)` retorna `true`; o mesmo arco mas com `ccw=false` faz `contains(..., Math.PI/2)` retornar `false`.
6. `arc.make({x:0,y:0}, 0, 0, Math.PI, true)` lança `Error`.

## 7. Notas de implementação

### Algoritmo de `bbox` (crítico)

Para um arco, a `bbox` é a **menor caixa axis-aligned** que contém todos os pontos varridos. O algoritmo:

1. Comece com `minX = min(start.x, end.x)`, `maxX = max(start.x, end.x)`, idem para Y. (Os endpoints sempre fazem parte do arco.)
2. Para cada um dos **quatro ângulos cardinais** — `0` (ponto `(cx+r, cy)`), `π/2` (`(cx, cy+r)`), `π` (`(cx-r, cy)`), `3π/2` (`(cx, cy-r)`) — testar se o ângulo está dentro do sweep do arco (via `contains`).
3. Se sim, expandir a bbox com aquele ponto cardinal.

A função `contains(angle)` normaliza `startAngle`, `endAngle` e `angle` para `[0, 2π)` e:

- Se `ccw === true`: ponto está dentro se `(angle - startAngle) mod 2π <= sweep`.
- Se `ccw === false`: ponto está dentro se `(startAngle - angle) mod 2π <= sweep`.

Esse algoritmo garante que arcos cruzando 0°, 90°, 180° ou 270° tenham bbox **exata**, não a bbox do círculo inteiro.

### Outras notas

- `sweep` calculado a partir de `startAngle`, `endAngle` e `ccw` — não armazenado redundante no objeto (estado derivado, plan.md L219 proíbe persistir).
- `midpoint` é `centerAt(start + sweep/2 * (ccw ? +1 : -1))`.
- Render: arco vai para `<path d="M ... A rx ry x-axis-rotation large-arc-flag sweep-flag x y">` em SVG (plan.md L246, L284). O cálculo de `large-arc-flag` e `sweep-flag` é responsabilidade de `render/entity-renderers.js`, não deste módulo.
- A `bbox` correta é essencial para zoom-extents (sprint Drawing/Precision) e seleção por janela.
- Decisão de ângulo: padrão matemático (0 em +X, CCW positivo), que é o oposto do "ângulo de relógio". A camada UI converte para o que o usuário entende (graus).
