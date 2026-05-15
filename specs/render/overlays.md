# render/overlays

## 1. Responsabilidade

Renderizar o **crosshair full-bleed** e o **label de coordenadas** dentro do grupo `#overlays` do `<svg>` raiz; manter slot para marcadores de snap em `#snaps` e expor a API `showSnapMarker` para uso futuro; capturar `pointermove` no `<svg>` e emitir `cursor:moved` no bus.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.render.svgRoot` (acessa `getLayer('overlays')`, `getLayer('snaps')`, `getRoot()`)
  - `window.LaserCAD.render.camera` (converte coordenadas via `worldFromScreen`)
  - `window.LaserCAD.app.state` (le `cursor`, `activeTool`, `toolState`)
  - `window.LaserCAD.bus` (consome `camera:changed`, `viewport:resized`; emite `cursor:moved`)
- ordem de carga: depois de `render.svgRoot`, `render.camera`, `bus`; ultimo de `render.*` em `index.html` (`namespace.md` linha 130).

## 3. API publica

Tudo sob `window.LaserCAD.render.overlays`.

```js
/**
 * Inicializa: cria os <line>s do crosshair full-bleed no grupo #overlays,
 * cria o <text> de label de coordenadas (escondido por default), e anexa
 * o handler pointermove no <svg> raiz. Idempotente.
 */
window.LaserCAD.render.overlays.init();

/**
 * Atualiza posicao do crosshair e do label de coordenadas com base em
 * state.cursor. Chamado a cada cursor:moved e camera:changed.
 */
window.LaserCAD.render.overlays.refresh();

/**
 * Mostra ou esconde o crosshair (ex.: cursor saiu do viewport).
 * @param {boolean} visible
 */
window.LaserCAD.render.overlays.setCrosshairVisible(visible);

/**
 * Slot publico para marcadores de snap. Sprint 1 declara a API; Sprint Precision
 * implementa de fato. Adiciona um marcador no grupo #snaps; chamada subsequente
 * com o mesmo type substitui o marcador anterior. type=null limpa.
 * @param {('endpoint'|'midpoint'|'center'|'intersection'|null)} type
 * @param {{x:number,y:number}|null} worldPoint - em mm
 */
window.LaserCAD.render.overlays.showSnapMarker(type, worldPoint);

/**
 * Atualiza o conteudo textual do label de coordenadas flutuante a 16 px do cursor.
 * Chamado por tools.* durante toolState='preview' (ex.: '42.500 mm', '35.0 x 22.5 mm').
 * Passar null esconde o label.
 * @param {string|null} text
 */
window.LaserCAD.render.overlays.setCursorLabel(text);
```

### 3.1 Estrutura DOM gerada dentro do `<svg>`

```html
<g id="overlays">
  <line class="crosshair-h" x1="..." y1="..." x2="..." y2="..." />
  <line class="crosshair-v" x1="..." y1="..." x2="..." y2="..." />
  <text class="cursor-label" x="..." y="...">42.500 mm</text>
</g>
<g id="snaps">
  <!-- slot vazio na Sprint 1; populado dinamicamente em sprints seguintes -->
</g>
```

## 4. Invariantes e tolerancias

### 4.1 Crosshair

- **Full-bleed**: as duas linhas atravessam **todo** o viewport (de uma borda a outra), nao formam uma cruz curta sobre o cursor (`design.md` L9, L158-L159, L286-L298). Coordenadas das linhas sao recalculadas em cada `refresh` com base no `viewBox` corrente.
- Cor:
  - `--text-secondary` (#8E7CB8) com opacidade 60% quando `state.toolState === 'idle'` (sem ferramenta armada) — `design.md` L289.
  - `--laser-glow` (#9D4DFF) quando `state.toolState in ('armed','preview')` — `design.md` L290, L159.
- Largura: `1 px` em ambos modos (`design.md` L286-L290).
- Ponto central de 4 px: marcador opcional no centro exato do cursor (`design.md` L289). Implementacao livre (pequeno `<circle r="2">` ou nao desenhar — o overlay full-bleed ja deixa a posicao obvia).

### 4.2 Label de coordenadas flutuante

- Posicao: **16 px** a direita-abaixo do cursor (`design.md` L185). Usar `screenFromWorld` invertendo o offset, ou simplesmente colocar o `<text>` com coordenadas em world equivalentes a `(cursor.screenX + 16, cursor.screenY + 16)` convertidas.
- Tipografia: `--font-mono` 11 px, `--text-primary` sobre `--bg-elevated` a 85% (`design.md` L185). Implementar via classe CSS `cursor-label` definida em `assets/css/app.css` (ver `assets/css/app.md`).
- Conteudo: controlado por `setCursorLabel(text)` chamado pelas tools durante `preview`. Sprint 1 mantem o label oculto (`null`).

### 4.3 Eventos

- O handler `pointermove` esta neste modulo (`render.overlays`) porque ele e o "dono visual do cursor" e e quem precisa do `<svg>` raiz para `worldFromScreen`. Apos calcular `worldX/worldY`, chama `LaserCAD.app.state.setCursor({...})` — que e quem efetivamente emite `cursor:moved` (regra dura: estado e quem emite mutacoes; `state-contract.md` §2.3).
- Status bar consome `cursor:moved` (via `app.state.cursor`) para exibir coordenadas (`state-contract.md` §3 linha do evento).
- **Snaps:** `showSnapMarker` insere/atualiza um filho de `#snaps` posicionado em world space; o slot e declarado mas **nao** ha logica de deteccao na Sprint 1.
- Esse modulo nao ouve `pointerdown`/`pointerup` — esses sao tratados por `tools` / `ui`.

### 4.4 Outras invariantes

- O `<text>` do label de coordenadas vive em `#overlays`, nao em `#snaps`; o `#snaps` e exclusivo dos 4 tipos de marcador (`design.md` L174-L185).
- Eventos canonicos: apenas `cursor:moved` (emitido), `camera:changed` e `viewport:resized` (consumidos). Nenhum evento inventado.
- Este modulo nao desenha tooltip de nome de ferramenta no canto inferior direito do cursor (`design.md` L290) — isso e responsabilidade de `ui` (tooltip flutuante HTML, nao SVG).

## 5. Exemplos de uso

```js
// Em app/bootstrap.js, apos render.svgRoot.mount():
window.LaserCAD.render.overlays.init();

// (Sprint Drawing) Uma tool de linha em preview:
window.LaserCAD.render.overlays.setCursorLabel('42.500 mm');

// (Sprint Precision) Snap detectado em (40, 0):
window.LaserCAD.render.overlays.showSnapMarker('endpoint', { x: 40, y: 0 });

// Limpar marcador
window.LaserCAD.render.overlays.showSnapMarker(null, null);

// Inspecao do crosshair
document.querySelectorAll('#overlays > line.crosshair-h, #overlays > line.crosshair-v').length;
// → 2
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html`: duas linhas roxas-claras (1 px) atravessam todo o viewport, cruzando-se no cursor. Mover o cursor: o cruzamento acompanha em tempo real.
2. Sair com o cursor da area do viewport (entrar na toolbar ou statusbar): `setCrosshairVisible(false)` esconde as duas linhas; voltar com o cursor: reaparecem.
3. No DevTools, `document.querySelectorAll('#overlays > line').length` retorna `2`; `document.querySelector('#snaps').children.length` retorna `0` na Sprint 1.
4. Subscrever `window.LaserCAD.bus.on('cursor:moved', console.log)` e mexer o mouse: cada movimento loga um payload `{worldX, worldY, screenX, screenY}` com coordenadas em mm coerentes com a posicao na tela.
5. Chamar `window.LaserCAD.render.overlays.showSnapMarker('endpoint', {x:0,y:0})` cria um marcador amarelo (`--snap-endpoint`) no (0,0); chamar com `null, null` o remove.
6. Chamar `window.LaserCAD.render.overlays.setCursorLabel('42.500 mm')` faz aparecer o texto `42.500 mm` perto do cursor; `setCursorLabel(null)` esconde.

## 7. Notas de implementacao

- Crosshair full-bleed e marca registrada do R14 (`design.md` L9, L349); usar largura 1 px e cores conforme tabela em `design.md` L162-L172.
- O label de coordenadas e diferente da statusbar: o **da statusbar** (rodape, `--text-primary` mono 11 px) sempre mostra coordenadas absolutas; o **flutuante** so aparece em `preview` com dimensao relativa (`design.md` L185, L225-L230).
- A conversao screen↔world dentro do handler `pointermove` deve usar `render.camera.worldFromScreen(...)` — nao reimplementar matriz aqui.
- `showSnapMarker` na Sprint 1: contrato declarado, corpo pode ser stub que so cria/remove um filho com forma e cor corretas; a logica de quando chamar fica em Sprint Precision.
- Sem animacoes de transicao (`design.md` L11): mostrar e esconder e troca direta de `display` ou de filhos no DOM.
- Sem cursor customizado em CSS para o viewport — uso de cursores nativos do SO; o crosshair SVG e que faz o papel visual (`design.md` L297).
