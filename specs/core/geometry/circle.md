# circle

## 1. Responsabilidade

Definir a entidade círculo completo (360°) e expor consultas puras: bbox e circunferência.

## 2. Dependências

- runtime: `window.LaserCAD.core.geometry.vec2`, `window.LaserCAD.core.geometry.epsilon`.
- ordem de carga: depois de `vec2` e `epsilon` (posição 4 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.circle`.

```text
@typedef {{ type: 'circle', center: Vec2, r: number }} CircleEntity
@typedef {{ minX: number, minY: number, maxX: number, maxY: number }} BBox

make(center: Vec2, r: number)   : CircleEntity   // lança se r <= EPS
bbox(c: CircleEntity)           : BBox           // [cx-r, cy-r, cx+r, cy+r]
circumference(c: CircleEntity)  : number         // 2 * PI * r
```

Contratos pré/pós:

- `make` valida que `center` é ponto finito e `r > EPS`. Lança `Error('circle.make: radius must be > 0')` caso contrário.
- `bbox` sempre satisfaz `minX < maxX` e `minY < maxY` (porque `r > 0`).
- `circumference` retorna `> 0`.
- Nenhuma função muta `c`.

## 4. Invariantes e tolerâncias

- Forma fixa: `{ type: 'circle', center: Vec2, r: number }`.
- `type: 'circle'` é literal — validadores conferem em `core.document.validators`.
- `r` é estritamente positivo (`> EPS`). Círculo com raio zero é proibido — usar ponto/seleção em vez disso.
- Unidades: `center.x`, `center.y`, `r`, `bbox`, `circumference` em **mm**.
- Pureza: **sem DOM, sem `app.state`, sem `bus`** (plan.md L222).

## 5. Exemplos de uso

```js
const C = window.LaserCAD.core.geometry.circle;

const c = C.make({ x: 64, y: 64 }, 8);

C.bbox(c); // {minX:56, minY:56, maxX:72, maxY:72}
C.circumference(c); // ~50.265 (2 * PI * 8)

// erro proibido
C.make({ x: 0, y: 0 }, 0); // lança Error
C.make({ x: 0, y: 0 }, -1); // lança Error
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.geometry.circle.make({x:0,y:0}, 0)` lança um `Error`.
2. `window.LaserCAD.core.geometry.circle.bbox(circle.make({x:64,y:64}, 8))` retorna `{minX:56, minY:56, maxX:72, maxY:72}`.
3. `window.LaserCAD.core.geometry.circle.circumference(circle.make({x:0,y:0}, 1))` retorna aproximadamente `6.283185307...` (verificável com `epsilon.eq`).
4. A entidade retornada por `make` tem `type === 'circle'` (literal).
5. `make` não modifica o objeto `center` passado (`center.x` inalterado após chamada).

## 7. Notas de implementação

- Forma coerente com `state-contract.md` §1.1 (entities com `id` + `type` + campos específicos). `commands.add` injeta o `id`.
- `bbox` é trivial para círculo completo (raio é o "extent" em ambos os eixos). Para arcos, o cálculo é mais elaborado — ver `arc.md` §7.
- Validação de `r` parecida com `vec2.normalize`: lançar é preferível a propagar uma entidade degenerada que corrompe seleção, render e exportação.
- Plan.md L246 indica que `circle` vira `<circle cx cy r>` em SVG — concretização em `render/entity-renderers.js`, não aqui.
- `circumference` é exposta para uso por status bar e dimensões dinâmicas (design.md L185, exemplo `R 18.000 mm` para círculo em preview).
