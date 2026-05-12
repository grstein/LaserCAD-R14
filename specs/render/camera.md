# render/camera

## 1. Responsabilidade
Manter o estado da câmera 2D (centro em mm + zoom + dimensões do viewport em px) e converter coordenadas entre `screen` (px) e `world` (mm), preservando o ponto sob o cursor em operações de zoom.

## 2. Dependências
- runtime:
  - `window.LaserCAD.app.state` (leitura de `camera`, escrita via `setCamera` / `setViewportSize`)
  - `window.LaserCAD.bus` (consome `viewport:resized`; emite `camera:changed`)
  - `window.LaserCAD.core.geometry.vec2` (operações pontuais sobre `{x,y}`)
  - `window.LaserCAD.render.svgRoot` (apenas para obter o elemento `<svg>` ao calcular `getScreenCTM()`)
- ordem de carga: depois de `app.state`, `bus`, `core.geometry.vec2`; antes de `render.svgRoot`, `render.grid`, `render.overlays` (ver `specs/_conventions/namespace.md` linha 126)

## 3. API pública

Todas as funções penduradas em `window.LaserCAD.render.camera`.

```js
/**
 * @typedef {{ x:number, y:number }} Vec2
 * @typedef {{ cx:number, cy:number, zoom:number, viewportW:number, viewportH:number }} CameraState
 */

/**
 * Retorna uma copia rasa do estado atual da camera.
 * @returns {CameraState}
 */
window.LaserCAD.render.camera.get();

/**
 * Converte um ponto em coordenadas de tela (px, relativo ao client rect do <svg>)
 * para coordenadas de mundo (mm). Usa getScreenCTM() do <svg> raiz (plan.md L19, L269).
 * @param {Vec2} screenPoint - {x, y} em pixels
 * @returns {Vec2} ponto em mm
 */
window.LaserCAD.render.camera.worldFromScreen(screenPoint);

/**
 * Converte um ponto em coordenadas de mundo (mm) para tela (px).
 * @param {Vec2} worldPoint - {x, y} em mm
 * @returns {Vec2} ponto em pixels
 */
window.LaserCAD.render.camera.screenFromWorld(worldPoint);

/**
 * Aplica pan: desloca cx/cy em mm (delta em world space) e emite camera:changed.
 * @param {number} dxWorld - delta x em mm
 * @param {number} dyWorld - delta y em mm
 */
window.LaserCAD.render.camera.panBy(dxWorld, dyWorld);

/**
 * Aplica zoom mantendo o screenPoint indicado fixo (mesmo ponto-mundo permanece sob o cursor).
 * Algoritmo:
 *   1. worldBefore = worldFromScreen(screenPoint)
 *   2. zoom = clamp(zoom * factor, 0.01, 1000)
 *   3. worldAfter = worldFromScreen(screenPoint) // com novo zoom
 *   4. cx += (worldBefore.x - worldAfter.x); cy += (worldBefore.y - worldAfter.y)
 *   5. emit camera:changed
 * @param {number} factor - multiplicador de zoom (ex.: 1.1 = zoom-in)
 * @param {Vec2} screenPoint - ponto-pivot em coordenadas de tela (px)
 */
window.LaserCAD.render.camera.zoomAt(factor, screenPoint);

/**
 * Reinicia camera para um enquadramento absoluto (zoom extents, zoom 1, etc.).
 * @param {{ cx:number, cy:number, zoom:number }} cam
 */
window.LaserCAD.render.camera.setAbsolute(cam);
```

## 4. Invariantes e tolerâncias

- `zoom` esta sempre em `[0.01, 1000]` (clamp duro em `zoomAt` e `setAbsolute`).
- `cx`, `cy`, dimensoes do `viewBox` derivado e qualquer conversao para o estado sao em **mm** (ADR 0001 §2). Apenas `viewportW`, `viewportH` e os pontos de tela sao em **px**.
- `worldFromScreen` e `screenFromWorld` sao **exatamente inversos** dentro de `EPS_SCREEN = 1e-3` px / `EPS_WORLD = 1e-6` mm.
- A unica fonte de verdade da transformacao para conversao screen↔world e `getScreenCTM()` do elemento `<svg>` raiz (plan.md L19, L269). A camera nao recalcula matriz propria.
- Pan: `panBy` desloca o ponto-mundo no centro do viewport. Disparado por **Espaco+drag** ou **botao do meio** (design.md L293).
- Zoom via wheel: invocado por `ui` / `tools` chamando `zoomAt(factor, screenPoint)` onde `screenPoint` e a posicao do cursor no momento do wheel — isso e o que mantem o ponto sob o cursor estavel.
- Toda mutacao bem-sucedida emite `camera:changed` com `{ cx, cy, zoom }` (ver `state-contract.md` §3).
- A camera **nao** muta `state.camera` diretamente: chama `LaserCAD.app.state.setCamera({...})`, que e quem dispara o evento e respeita a regra dura de mutacao centralizada (`state-contract.md` §2).
- Resposta a `viewport:resized` chama `LaserCAD.app.state.setViewportSize(w, h)`; a camera nao re-centraliza nem re-zooma automaticamente.

## 5. Exemplos de uso

```js
// Inspecionar estado
window.LaserCAD.render.camera.get();
// → { cx: 64, cy: 64, zoom: 1, viewportW: 1600, viewportH: 900 }

// Pan programatico de 10 mm para a direita
window.LaserCAD.render.camera.panBy(-10, 0);

// Zoom 10% in com cursor em (800, 450) px
window.LaserCAD.render.camera.zoomAt(1.10, { x: 800, y: 450 });

// Conversao de teste
const w = window.LaserCAD.render.camera.worldFromScreen({ x: 100, y: 100 });
const s = window.LaserCAD.render.camera.screenFromWorld(w);
// s deve voltar a {x: 100, y: 100} dentro de EPS
```

## 6. Criterios de aceitacao testaveis manualmente

1. No DevTools, `window.LaserCAD.render.camera.get()` retorna um objeto com exatamente as chaves `cx, cy, zoom, viewportW, viewportH` e zoom inicial `1`.
2. Apertando `Espaco` e arrastando o mouse, o conteudo do viewport segue o cursor sem latencia visual; soltar `Espaco` interrompe o pan.
3. Apos `panBy(50, 0)` no console, a status bar (alimentada por `camera:changed`) reflete que o centro se moveu 50 mm; a geometria visivel desloca para a esquerda na tela.
4. Apontando o cursor sobre um ponto reconhecivel (ex.: intersecao do grid em (0,0)) e girando a roda do mouse, o **mesmo ponto-mundo** permanece sob o cursor antes e depois — o (0,0) nao desliza pela tela.
5. Tentar `setAbsolute({cx:0,cy:0,zoom:5000})` resulta em `zoom === 1000` apos a chamada (clamp confirmado por `get()`).
6. Subscrever `window.LaserCAD.bus.on('camera:changed', console.log)` e mover/zoom — cada acao loga exatamente um payload `{cx, cy, zoom}`.

## 7. Notas de implementacao

- Conversao baseada em `getScreenCTM()`: `plan.md` L19 ("`getScreenCTM()` e adequado para a conversao entre coordenadas de tela e coordenadas do SVG") e L269 ("Divergencia tela↔documento ... centralizar camera e conversoes world/screen com `getScreenCTM()`").
- `viewBox` em mm: `plan.md` L9, L26.
- Range de zoom `[0.01, 1000]`: protege contra perda de precisao em float64 sem limitar usos plausiveis (1 mm/px = 0.1× a 100×, com folga em ambas as pontas).
- Pan: gatilhos definidos em `design.md` L293 (Espaco+drag ou botao do meio); cursor "grab" do sistema durante pan.
- A maquinaria de Pointer Events (mousedown/move/up + wheel) **nao** vive aqui — vive em `tools` / `ui`; este modulo so expoe `panBy` e `zoomAt` como pontos de entrada.
- Re-render do grid e overlays e acionado por subscribers de `camera:changed`, nao por chamadas diretas da camera.
