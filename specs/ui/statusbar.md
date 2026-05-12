# ui.statusbar

## 1. Responsabilidade

Renderizar a barra de status de 24px no rodape: slot de coordenadas (mm, 3 casas decimais, atualizado a cada `cursor:moved`), toggles SNAP/GRID/ORTHO (espelhando F3/F7/F8 e cliques), divisores `│`, e slot de autosave (estatico `● not yet` na Sprint 1).

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus`
  - `window.LaserCAD.app.state` (leitura de `cursor`, `toggles`; mutacao via `setToggle`)
- ordem de carga: posicao #24 em `specs/_conventions/namespace.md`.

## 3. API publica

```js
window.LaserCAD.ui.statusbar = {
  /**
   * Monta a status bar dentro de #statusbar-host.
   * @param {HTMLElement} host
   * @returns {void}
   */
  mount(host) { /* ... */ },

  /**
   * Re-renderiza coordenadas e toggles a partir do state. Normalmente
   * chamado pelos subscribers de cursor:moved / toggle:changed.
   * @returns {void}
   */
  refresh() { /* ... */ }
};
```

### 3.1 Estrutura horizontal (esquerda → direita)

| Slot              | Conteudo                                                  | Estado / cor                                       |
|-------------------|-----------------------------------------------------------|----------------------------------------------------|
| 1. coordenadas    | `XXX.XXX, YYY.YYY  mm` (3 casas decimais)                 | `--text-primary` (#E8DDFF)                         |
| 2. divisor        | `│`                                                       | `--border-subtle` (#241638)                        |
| 3. toggle SNAP    | `◉ SNAP` ou `○ SNAP` (icone + label)                      | `--laser-450` se on, `--text-secondary` se off     |
| 4. divisor        | `│`                                                       | `--border-subtle`                                  |
| 5. toggle GRID    | `◉ GRID` ou `○ GRID`                                      | idem                                               |
| 6. divisor        | `│`                                                       | `--border-subtle`                                  |
| 7. toggle ORTHO   | `◉ ORTHO` ou `○ ORTHO`                                    | idem                                               |
| 8. spacer flexivel|                                                           |                                                    |
| 9. autosave       | `● not yet` (Sprint 1, estatico)                          | `--text-secondary` (#8E7CB8)                       |

Cada slot eh um `<span>` (ou `<button>` para os toggles), tipografia `--font-mono` 11px (design.md L221, L67), altura total da barra 24px (design.md L112).

## 4. Invariantes e tolerancias

- **Coordenadas com 3 casas decimais.** Formatacao: `worldX.toFixed(3) + ', ' + worldY.toFixed(3) + '  mm'`. Mesmo quando `worldX < 0` ou inteiro exato, mantem 3 casas. Espacamento duplo entre `Y` e `mm` por convencao visual (design.md L102, L225).
- **Atualizacao por evento, nao por polling.** Subscrever `cursor:moved` no bus; ler `payload.worldX`/`worldY`. Frequencia natural de `pointermove` eh suficiente; nao usar `requestAnimationFrame` extra.
- **Toggles refletem `state.toggles`.** A barra **nunca** muta `state.toggles` por atribuicao direta — sempre `LaserCAD.app.state.setToggle(name, value)` (state-contract.md §2.3). O setter emite `toggle:changed`.
- **Click em toggle alterna.** `<button>` com `data-toggle="snap|grid|ortho"`. `click` → `state.setToggle(name, !state.toggles[name])`. Reagir a `toggle:changed` para re-render.
- **Hover em toggle muda cor.** Design.md L236: hover muda para `--laser-glow`. Implementar via CSS `:hover`.
- **F3/F7/F8 espelhados.** O modulo `app.shortcuts` (WS-A) emite os mesmos `toggle:changed` quando o usuario pressiona F3 (snap), F7 (grid), F8 (ortho). A statusbar consome esses eventos e re-renderiza. NAO duplicar o listener de teclado aqui.
- **Slot autosave Sprint 1.** Texto literal `● not yet` em `--text-secondary`. Sem subscribers de eventos de autosave (nao existem na lista canonica de Sprint 1). Reservado para Sprint 5+.
- **Divisores `│` em `--border-subtle`.** Caractere `│` (BOX DRAWINGS LIGHT VERTICAL). Cor distinta do label de toggle.
- **Acessibilidade.** Toggles tem `role="switch"`, `aria-checked` espelhando o estado, `aria-label="<Name> snap"`, etc. Tab-index na ordem coords (nao focavel) → SNAP → GRID → ORTHO → autosave (nao focavel).

## 5. Exemplos de uso

```js
// Bootstrap monta:
LaserCAD.ui.statusbar.mount(document.getElementById('statusbar-host'));

// Cursor move (emitido por render.overlays):
LaserCAD.bus.emit('cursor:moved', {
  worldX: 124.5, worldY: 87.3, screenX: 800, screenY: 400
});
// Statusbar atualiza o slot de coordenadas para "124.500, 87.300  mm".

// Click no toggle SNAP (programatico):
document.querySelector('[data-toggle="snap"]').click();
// → state.setToggle('snap', !state.toggles.snap)
// → emit 'toggle:changed' { name:'snap', value:<novo> }
// → refresh() reflete visualmente.

// F7 pressionado (via shortcuts):
LaserCAD.bus.emit('toggle:changed', { name:'grid', value:false });
// Statusbar mostra "○ GRID" em --text-secondary.

// Forcar refresh:
LaserCAD.ui.statusbar.refresh();
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Render inicial.** Apos `app:ready`, a statusbar mostra `0.000, 0.000  mm  │ ◉ SNAP │ ◉ GRID │ ○ ORTHO       ● not yet` (snap=true, grid=true, ortho=false, pre state-contract §1.1).
2. **Atualizacao por `cursor:moved`.** Emitir `LaserCAD.bus.emit('cursor:moved', {worldX:1, worldY:2, screenX:0, screenY:0})` no console. O texto vira `1.000, 2.000  mm`.
3. **3 casas decimais sempre.** `cursor:moved` com `worldX: 10` -> exibe `10.000`. `worldX: 10.12345` -> `10.123`. `worldX: -5.5` -> `-5.500`.
4. **Toggle visual.** `LaserCAD.app.state.toggles.snap` eh `true` -> elemento de SNAP tem cor computada `--laser-450` (#6E00FF). Apos click ou `LaserCAD.app.state.setToggle('snap', false)`, cor vira `--text-secondary` (#8E7CB8) e icone `○`.
5. **Click reflete bus.** Click no botao SNAP. Subscriber `LaserCAD.bus.on('toggle:changed', console.log)` recebe `{name:'snap', value:false}` (ou true).
6. **F3 sincroniza.** Apertar `F3` com foco no body. Statusbar mostra novo estado de SNAP. (Pressuposto: `app.shortcuts` esta ativo — testado quando WS-A entrega.)
7. **Hover muda cor.** Mover mouse sobre o toggle GRID: cor de stroke/cor do span vira `--laser-glow` (#9D4DFF).
8. **Autosave estatico.** Slot mostra `● not yet` em cinza, e isso **nao muda** em qualquer evento de Sprint 1.
9. **Divisores presentes.** `#statusbar-host` contem ao menos 3 elementos com caractere `│` (ou `│`).
10. **Acessibilidade.** Cada toggle tem `role="switch"` e `aria-checked` espelhando o state. `Tab` entra na statusbar e percorre os 3 toggles em ordem.

## 7. Notas de implementacao

- Design.md L220–236 eh a fonte autoritativa do visual.
- Design.md L102 mostra `124.500, 87.300 mm` — espaco duplo antes de `mm`. Manter.
- Para evitar reflows custosos a 60+ fps, a refresh por `cursor:moved` pode atualizar somente o `textContent` do `<span>` de coordenadas, nao re-renderizar a barra inteira.
- Plan.md L102, L268 fundamentam a integridade do autosave (Sprint 5+). Sprint 1 nao implementa.
- A regra "Tab percorre toolbar → command line → status bar toggles → menubar" (design.md L339) sugere `tabindex` ordenado. Em Sprint 1, basta deixar a ordem natural do DOM coincidir com essa expectativa.
- Eventos consumidos: `cursor:moved`, `toggle:changed`, `app:ready` (para primeira renderizacao). Eventos emitidos: nenhum diretamente; a mutacao de toggles passa por `state.setToggle` que emite `toggle:changed`.
- Nenhum evento fora da lista canonica de state-contract.md §3.
- Resilience: se `state.cursor` ainda esta `{0,0,0,0}` no inicio (antes do primeiro `cursor:moved`), o texto inicial eh `0.000, 0.000  mm` — comportamento aceitavel.
