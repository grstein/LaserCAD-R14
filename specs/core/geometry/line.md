# line

## 1. Responsabilidade
Definir a entidade geométrica linha (segmento) e expor consultas puras sobre ela: comprimento, direção, ponto médio, bounding box.

## 2. Dependências
- runtime: `window.LaserCAD.core.geometry.vec2`, `window.LaserCAD.core.geometry.epsilon`.
- ordem de carga: depois de `vec2` e `epsilon` (posição 3 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.line`.

```text
@typedef {{ type: 'line', p1: Vec2, p2: Vec2 }} LineEntity
@typedef {{ minX: number, minY: number, maxX: number, maxY: number }} BBox

make(p1: Vec2, p2: Vec2)        : LineEntity   // construtor canônico (sem id; commands cuidam de id)
length(l: LineEntity)           : number       // distância p1↔p2 em mm
direction(l: LineEntity)        : Vec2         // vetor unitário p1→p2; lança se degenerada
midpoint(l: LineEntity)         : Vec2         // (p1+p2)/2
bbox(l: LineEntity)             : BBox         // caixa axis-aligned
```

Contratos pré/pós:
- `make` valida que `p1` e `p2` são pontos finitos. Não checa degeneração (linha de comprimento zero é representável, mas degenerada para `direction`).
- `length` retorna `>= 0`. Vale `0` para `p1 === p2` (dentro de `EPS`).
- `direction` lança `Error('line.direction: degenerate line')` quando `length(l) < EPS`.
- `bbox` sempre satisfaz `minX <= maxX` e `minY <= maxY`.
- Nenhuma função muta `l`.

## 4. Invariantes e tolerâncias
- Forma fixa do objeto: `{ type: 'line', p1: Vec2, p2: Vec2 }`. Campos extras são ignorados pelas consultas mas devem ser preservados (ex.: `id` que `core.document.commands` adiciona).
- `type: 'line'` é literal — validadores futuros (em `core.document.validators`) conferem.
- Unidades: `p1`, `p2`, `length`, `midpoint`, `bbox` em **mm**. `direction` é adimensional (unitário).
- Estado proibido: linha com `p1` ou `p2` contendo `NaN`/`Infinity`. Detecção fica em `core.document.validators`; este módulo confia em entrada saneada.
- Pureza: **sem DOM, sem `app.state`, sem `bus`** (plan.md L222).

## 5. Exemplos de uso

```js
const L = window.LaserCAD.core.geometry.line;

const l = L.make({x: 10, y: 10}, {x: 60, y: 10});

L.length(l);                       // 50
L.direction(l);                    // {x: 1, y: 0}
L.midpoint(l);                     // {x: 35, y: 10}
L.bbox(l);                         // {minX: 10, minY: 10, maxX: 60, maxY: 10}

// linha diagonal
const d = L.make({x: 0, y: 0}, {x: 3, y: 4});
L.length(d);                       // 5
L.direction(d);                    // {x: 0.6, y: 0.8}
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.geometry.line.length(line.make({x:0,y:0},{x:3,y:4})) === 5`.
2. `window.LaserCAD.core.geometry.line.midpoint(line.make({x:0,y:0},{x:10,y:20}))` retorna `{x:5, y:10}`.
3. `window.LaserCAD.core.geometry.line.bbox(line.make({x:60,y:60},{x:10,y:10}))` retorna `{minX:10, minY:10, maxX:60, maxY:60}` (ordenação correta independe da ordem dos pontos).
4. `window.LaserCAD.core.geometry.line.direction(line.make({x:5,y:5},{x:5,y:5}))` lança `Error`.
5. A entidade retornada por `make` tem `type === 'line'` (literal) e os componentes são objetos novos (`l.p1 !== p1Original`).

## 7. Notas de implementação
- A forma `{type, p1, p2}` é coerente com o entity model em `state-contract.md` §1.1: `entities[*]` tem `id` + `type` + campos específicos do tipo. `make` produz a parte sem `id`; `commands.add` adiciona `id: 'e_<n>'`.
- `direction` é calculada como `vec2.normalize(vec2.sub(p2, p1))` — herda o lançamento de `normalize` (vec2.md §3).
- `bbox` para linha é trivial mas implementa o contrato compartilhado com `circle.bbox` e `arc.bbox`, usado por seleção por janela e zoom-extents.
- `length` usa `vec2.len(vec2.sub(p2, p1))`.
- Não precisa do `epsilon` para nenhuma operação direta — depende apenas porque `vec2.normalize` (via `direction`) consulta `EPS`. Listado como dependência por transitive clarity.
- Render: a transformação em `<line x1 y1 x2 y2>` é trabalho de `render/entity-renderers.js` (plan.md L246), não deste módulo.
