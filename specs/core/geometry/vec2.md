# vec2

## 1. Responsabilidade

Fornecer a álgebra elementar de vetores 2D em milímetros — operações puras sobre objetos literais `{x, y}`, sem estado nem mutação.

## 2. Dependências

- runtime: `window.LaserCAD.core.geometry.epsilon` (apenas para `eq` em `equals`).
- ordem de carga: depois de `epsilon` (posição 2 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.vec2`.

```text
@typedef {{x: number, y: number}} Vec2

add(a: Vec2, b: Vec2)              : Vec2     // {x: a.x+b.x, y: a.y+b.y}
sub(a: Vec2, b: Vec2)              : Vec2     // a - b
scale(a: Vec2, k: number)          : Vec2     // {x: a.x*k, y: a.y*k}
dot(a: Vec2, b: Vec2)              : number   // a.x*b.x + a.y*b.y
cross(a: Vec2, b: Vec2)            : number   // a.x*b.y - a.y*b.x (escalar 2D)
len(a: Vec2)                       : number   // sqrt(dot(a,a))
normalize(a: Vec2)                 : Vec2     // a/|a|; lança se |a| < EPS
equals(a: Vec2, b: Vec2, tol?: number) : boolean
```

Contratos pré/pós:

- Todas as entradas precisam ter `x` e `y` finitos (`Number.isFinite`). Caso contrário, comportamento indefinido (não silenciar — preferir lançar em modo dev).
- **Imutabilidade obrigatória**: nenhuma função altera `a` ou `b`. Cada operação retorna um **novo** objeto literal `{x, y}`.
- `normalize` lança `Error('vec2.normalize: zero-length vector')` quando `len(a) < EPS`.
- `equals` usa `epsilon.eq` por componente; `tol` default = `EPS`.

## 4. Invariantes e tolerâncias

- Unidades: `x`, `y` em **mm** quando o vetor representa ponto-mundo; adimensional quando representa direção normalizada.
- Estado proibido: este módulo não armazena nada. Não há cache, não há pool de vetores.
- Pureza absoluta: **sem DOM, sem `app.state`, sem `bus`, sem `setTimeout`** (plan.md L222).
- `equals` compara por componente independente — não usa magnitude da diferença.
- `cross` retorna escalar (componente Z do produto vetorial 3D) — sinal indica orientação: positivo = `b` à esquerda de `a`.

## 5. Exemplos de uso

```js
const v = window.LaserCAD.core.geometry.vec2;

const a = { x: 1, y: 2 };
const b = { x: 3, y: 4 };

v.add(a, b); // {x: 4, y: 6}
v.sub(b, a); // {x: 2, y: 2}
v.scale(a, 2); // {x: 2, y: 4}
v.dot(a, b); // 11
v.cross(a, b); // -2  (b está à direita de a)
v.len({ x: 3, y: 4 }); // 5
v.normalize({ x: 3, y: 4 }); // {x: 0.6, y: 0.8}
v.equals({ x: 1, y: 1 }, { x: 1.0000000001, y: 1 }); // true
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.geometry.vec2.add({x:1,y:2},{x:3,y:4})` retorna `{x:4, y:6}` e **não** modifica os argumentos (verificar `a.x === 1` após chamada).
2. `window.LaserCAD.core.geometry.vec2.len({x:3, y:4}) === 5` é verdadeiro (sem erro de FP perceptível).
3. `window.LaserCAD.core.geometry.vec2.normalize({x:0, y:0})` lança um `Error`.
4. `window.LaserCAD.core.geometry.vec2.cross({x:1,y:0},{x:0,y:1}) === 1` (rotação CCW positiva).
5. `window.LaserCAD.core.geometry.vec2.equals({x:0.1+0.2, y:0},{x:0.3, y:0})` retorna `true` (usa `epsilon.eq`).
6. Verificar que `Object.isFrozen(window.LaserCAD.core.geometry.vec2)` é `true` (ou que o módulo não permite redefinir membros).

## 7. Notas de implementação

- A representação `{x, y}` (objeto literal) é fixada deliberadamente — não usar arrays `[x, y]` nem classes. Razão: serialização JSON direta para o estado/exportação (plan.md L219) e acesso `.x`/`.y` legível em snapshots de debug.
- Imutabilidade é uma escolha de simplicidade KISS (plan.md L9), não uma performance crítica. Para Sprint 1, o overhead de alocação é irrelevante (< centenas de operações por frame). Se o profiler indicar gargalo na Sprint Precision (plan.md L120), revisitar com pool ou API in-place separada.
- `cross` retorna escalar (não `Vec2`) por convenção 2D — é o componente Z do produto vetorial 3D embutido em XY.
- A função `equals` aceita `tol` para casos como snap, onde a tolerância é maior que `EPS` (ex.: 0.001 mm).
- `normalize` lançar em vez de retornar `{x:0,y:0}` evita erros silenciosos rio abaixo (direção zero corrompe `line.direction`, `arc.bbox`, etc.).
