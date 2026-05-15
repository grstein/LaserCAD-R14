# epsilon

## 1. Responsabilidade

Publicar constantes de tolerância e helpers de comparação tolerante a ponto-flutuante usados por todo o kernel geométrico e pelo motor de snap.

## 2. Dependências

- runtime: nenhum sub-namespace anterior — é a primeira folha de `core.geometry`.
- ordem de carga: primeiro arquivo da sequência (posição 1 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.epsilon`.

```text
EPS              : number       // 1e-9, em mm
SNAP_TOLERANCE_PX: number       // 8, em pixels de tela
eq(a, b, tol?)   : boolean      // |a - b| <= tol (default EPS)
lt(a, b, tol?)   : boolean      // a < b - tol
gt(a, b, tol?)   : boolean      // a > b + tol
lte(a, b, tol?)  : boolean      // a <= b + tol
gte(a, b, tol?)  : boolean      // a >= b - tol
```

JSDoc de exemplo:

```text
@param {number} a
@param {number} b
@param {number} [tol=EPS]
@returns {boolean}
```

Contratos pré/pós:

- `a` e `b` precisam ser `Number.isFinite`. Comportamento para `NaN`/`Infinity` é "retorna `false`" sem lançar.
- `tol` precisa ser `>= 0`. Se ausente, usa `EPS`.
- Nenhuma das funções aloca objeto novo.

## 4. Invariantes e tolerâncias

- `EPS = 1e-9` é expressa em **milímetros** — coerente com a unidade canônica do kernel (ADR 0001 §2; plan.md L217, L225).
- `SNAP_TOLERANCE_PX = 8` é o único valor expresso em **pixels** neste módulo e existe somente porque o snap é avaliado em espaço-tela (consumido por `tools/`/`render/overlays` em sprints futuras).
- Helpers são **simétricos** quando aplicável: `eq(a,b) === eq(b,a)`, `lt(a,b) === gt(b,a)`.
- Nenhum estado mutável vive aqui — o módulo é um conjunto de funções puras e constantes congeladas.
- Pureza obrigatória: este módulo **não toca DOM, não lê `app.state`, não emite eventos** (plan.md L222).

## 5. Exemplos de uso

```js
const { EPS, eq, lt, SNAP_TOLERANCE_PX } = window.LaserCAD.core.geometry.epsilon;

eq(0.1 + 0.2, 0.3); // true
eq(1.0000001, 1, 1e-6); // true
lt(0.999999999, 1); // false (dentro de EPS)
SNAP_TOLERANCE_PX; // 8

// Uso típico no kernel
if (window.LaserCAD.core.geometry.epsilon.eq(line.length(), 0)) {
  throw new Error('Degenerate line');
}
```

## 6. Critérios de aceitação testáveis manualmente

1. No DevTools: `window.LaserCAD.core.geometry.epsilon.EPS === 1e-9` retorna `true`.
2. `window.LaserCAD.core.geometry.epsilon.eq(0.1 + 0.2, 0.3)` retorna `true` (caso clássico de erro de ponto flutuante).
3. `window.LaserCAD.core.geometry.epsilon.lt(1, 1)` retorna `false` e `epsilon.lte(1, 1)` retorna `true`.
4. `window.LaserCAD.core.geometry.epsilon.eq(NaN, NaN)` retorna `false` (NaN nunca é igual a nada).
5. `window.LaserCAD.core.geometry.epsilon.SNAP_TOLERANCE_PX === 8` retorna `true`.

## 7. Notas de implementação

- Convenção de tolerâncias centralizadas: plan.md L225 ("tudo passa por constantes centralizadas (`EPS`, `SNAP_TOLERANCE`)").
- A escolha de `EPS = 1e-9` (em mm) é mais estrita que `1e-6` mencionada no ADR 0001 §2.4 como exemplo — uso `1e-9` por padrão; quem precisar de tolerância mais larga passa explicitamente (ex.: `eq(a, b, 1e-6)`).
- `SNAP_TOLERANCE_PX` vive aqui porque é universal o suficiente para ser referenciada por `render/overlays` e `tools/select-tool` sem criar dependência em outro lugar.
- Congelar o objeto exportado com `Object.freeze(...)` é recomendado para impedir mutação acidental do constante por outros módulos.
