# project

## 1. Responsabilidade

Publicar as **assinaturas oficiais** das funções de conversão entre coordenadas-mundo (mm) e coordenadas-tela (px). A implementação efetiva vive em `render/camera`; este módulo apenas re-expõe a interface para callers fora de `render/` que não devem depender diretamente de `render.*` (p.ex. um helper futuro em `core/geometry/snap.js` que precise saber se um ponto-mundo está dentro do viewport).

> **Atenção arquitetural:** este módulo **não duplica** a matemática da câmera. Ele é um ponto de entrada limitado que delega — ou, equivalentemente, descreve o contrato que `render/camera` precisa cumprir. Evita duas fontes de verdade (plan.md L269 — risco de "divergência tela↔documento").

## 2. Dependências

- runtime de assinatura: `window.LaserCAD.core.geometry.vec2` (forma `Vec2`).
- runtime de execução: `window.LaserCAD.render.camera` precisa estar carregado **antes** da primeira chamada. Como `render.camera` carrega na posição 15 e `project` na 6 (`specs/_conventions/namespace.md` §3), as chamadas só são válidas após `app:ready`.
- ordem de carga: depois de `vec2`, `line`, `circle`, `arc` (posição 6). A função em si pode ser exposta com lazy lookup (não captura a referência da câmera no load).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.geometry.project`.

```text
@typedef {{ cx: number, cy: number, zoom: number, viewportW: number, viewportH: number }} Camera

worldFromScreen(point: Vec2, camera: Camera)  : Vec2   // px -> mm
screenFromWorld(point: Vec2, camera: Camera)  : Vec2   // mm -> px
```

Contratos:

- `point` é `{x, y}` finito.
- `camera` é o `state.camera` (forma fixada em `specs/_conventions/state-contract.md` §1.1).
- Inversibilidade aproximada: `worldFromScreen(screenFromWorld(p, cam), cam) ≈ p` dentro de `EPS`.
- **Pureza relativa:** funções não tocam `app.state` nem disparam eventos; só leem `camera` recebido por parâmetro.

## 4. Invariantes e tolerâncias

- Origem do mundo: `(0, 0)` em mm. Eixo Y do mundo aponta **para cima** matematicamente; a inversão para o eixo Y de tela (que aponta para baixo) é responsabilidade de `render/camera` na implementação.
- Origem da tela: canto superior-esquerdo do `<svg>`, `(0, 0)` em px.
- `camera.zoom` é adimensional (px-por-mm × constante de calibração). `zoom > 0` obrigatório.
- `camera.viewportW`/`H` em px. Se ainda forem `0` (antes do primeiro `viewport:resized`), as funções retornam valores indefinidos — o caller é responsável por chamar depois de `app:ready`.
- **Duas fontes de verdade proibidas:** se este módulo reimplementar a matemática, ele estará em **violação** desta spec. A implementação correta é delegação a `window.LaserCAD.render.camera.worldFromScreen(...)` / `screenFromWorld(...)` ou uma referência compartilhada à mesma função.
- Pureza relativa ao kernel: este é o **único** módulo de `core/geometry` que toca outro sub-namespace (`render.camera`). É exceção explícita.

## 5. Exemplos de uso

```js
const P = window.LaserCAD.core.geometry.project;
const cam = window.LaserCAD.app.state.camera;

// O cursor está em (480, 270) px na tela. Onde está no mundo?
const w = P.worldFromScreen({ x: 480, y: 270 }, cam);
// → algo como {x: 64.000, y: 36.000} dependendo do zoom/pan

// Roundtrip
const s = P.screenFromWorld(w, cam);
// → {x: ~480, y: ~270}
```

## 6. Critérios de aceitação testáveis manualmente

1. `typeof window.LaserCAD.core.geometry.project.worldFromScreen === 'function'` retorna `true`.
2. `typeof window.LaserCAD.core.geometry.project.screenFromWorld === 'function'` retorna `true`.
3. **Roundtrip:** dado o `state.camera` corrente, `project.worldFromScreen(project.screenFromWorld({x:50, y:50}, cam), cam)` retorna ponto a menos de `EPS` de `{x:50, y:50}`.
4. Chamar `project.worldFromScreen` retorna o **mesmo resultado** que `window.LaserCAD.render.camera.worldFromScreen` (ou equivalente exposto pela câmera) com os mesmos argumentos — confirmando que não há duas fontes de verdade.
5. Após `app:ready`, com viewport 1600×900 e `state.camera.zoom === 1`, o centro da tela mapeia para algo coerente com `state.camera.cx/cy` (verificação de sanidade: pan/zoom default → centro do viewport ≈ `(cx, cy)`).

## 7. Notas de implementação

- A justificativa para este módulo existir em vez de callers usarem `render.camera` diretamente: manter `core/geometry/snap.js` (sprint futura) sem dependência declarativa em `render/*`. O contrato vive em `core`, a implementação em `render`.
- Implementação sugerida: a IIFE expõe `worldFromScreen(p, cam)` que faz `return window.LaserCAD.render.camera.worldFromScreen(p, cam);` — lookup tardio para evitar problema de ordem de carga.
- Alternativa: `render.camera` registra suas funções aqui no carregamento, com um pequeno `register` no load. Sprint 1 decide pela primeira (lookup tardio) para minimizar acoplamento.
- Plan.md L269: o risco "divergência tela↔documento" é exatamente o que este módulo combate ao recusar reimplementação.
- Este módulo **não** define a representação de `Camera` — essa é fixada em `specs/_conventions/state-contract.md` §1.1.
