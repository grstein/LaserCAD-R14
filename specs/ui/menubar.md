# ui.menubar

## 1. Responsabilidade

Renderizar a menubar superior (28px de altura) com os 6 menus textuais `File / Edit / View / Draw / Modify / Help`. Abrir dropdown ao click, fechar em `Esc` ou click fora. Sprint 1: apenas 5 itens habilitados; demais permanecem visiveis mas desabilitados.

Este modulo eh um **adendo** ao escopo: nao consta em `plan.md` L196–200 (que lista `toolbar`/`commandLine`/`statusbar`/`dialogs`), mas eh exigido por `design.md` L82, L108, L238–251. O adendo eh permitido pelo briefing de WS-C.

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus`
  - `window.LaserCAD.app.shortcuts` (para reverse-lookup de atalhos exibidos a direita do label)
- ordem de carga: posicao #25 em `specs/_conventions/namespace.md`.

## 3. API publica

```js
window.LaserCAD.ui.menubar = {
  /**
   * Monta a menubar dentro de #menubar-host.
   * @param {HTMLElement} host
   * @returns {void}
   */
  mount(host) {
    /* ... */
  },

  /**
   * Abre o dropdown do menu indicado (`'file'|'edit'|'view'|'draw'|'modify'|'help'`).
   * Fecha qualquer outro aberto. Sem efeito se ja aberto.
   * @param {string} menuId
   * @returns {void}
   */
  open(menuId) {
    /* ... */
  },

  /**
   * Fecha qualquer dropdown aberto. Idempotente.
   * @returns {void}
   */
  close() {
    /* ... */
  },
};
```

### 3.1 Estrutura dos menus (design.md L242–250)

| Menu       | Item                   | Atalho   | Sprint 1    |
| ---------- | ---------------------- | -------- | ----------- |
| **File**   | New                    | `Ctrl+N` | disabled    |
|            | Open…                  | `Ctrl+O` | disabled    |
|            | Save SVG…              | `Ctrl+S` | disabled    |
|            | Recent files ›         | —        | disabled    |
|            | _separador_            |          |             |
|            | Exit                   | —        | disabled    |
| **Edit**   | Undo                   | `Ctrl+Z` | disabled    |
|            | Redo                   | `Ctrl+Y` | disabled    |
|            | _separador_            |          |             |
|            | Cut                    | `Ctrl+X` | disabled    |
|            | Copy                   | `Ctrl+C` | disabled    |
|            | Paste                  | `Ctrl+V` | disabled    |
|            | _separador_            |          |             |
|            | Delete                 | `Del`    | disabled    |
|            | Select all             | `Ctrl+A` | disabled    |
| **View**   | Zoom in                | `+`      | disabled    |
|            | Zoom out               | `−`      | disabled    |
|            | **Zoom extents**       | `Z E`    | **enabled** |
|            | Zoom window            | `Z W`    | disabled    |
|            | _separador_            |          |             |
|            | **Toggle grid**        | `F7`     | **enabled** |
|            | **Toggle snap**        | `F3`     | **enabled** |
|            | **Toggle ortho**       | `F8`     | **enabled** |
| **Draw**   | Line                   | `L`      | disabled    |
|            | Polyline               | `P`      | disabled    |
|            | Rectangle              | `R`      | disabled    |
|            | Circle                 | `C`      | disabled    |
|            | Arc                    | `A`      | disabled    |
| **Modify** | Select                 | `S`      | disabled    |
|            | Move                   | `M`      | disabled    |
|            | Trim                   | `T`      | disabled    |
|            | Extend                 | `E`      | disabled    |
|            | Delete                 | `Del`    | disabled    |
| **Help**   | Keyboard shortcuts     | `F1`     | disabled    |
|            | **About LaserCAD R14** | —        | **enabled** |

Itens habilitados na Sprint 1: 5 (Zoom extents, Toggle grid, Toggle snap, Toggle ortho, About).

### 3.2 Acoes dos itens habilitados

| Item         | Acao no click                                                                                                                                                                                                                  |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Zoom extents | `LaserCAD.bus.emit('command:submit', { raw:'zoom extents', parsed:{kind:'view', action:'zoomExtents'} })`                                                                                                                      |
| Toggle grid  | `LaserCAD.app.state.setToggle('grid', !state.toggles.grid)` (setter emite `toggle:changed`)                                                                                                                                    |
| Toggle snap  | `LaserCAD.app.state.setToggle('snap', !state.toggles.snap)`                                                                                                                                                                    |
| Toggle ortho | `LaserCAD.app.state.setToggle('ortho', !state.toggles.ortho)`                                                                                                                                                                  |
| About        | `LaserCAD.ui.dialogs.open({ id:'about', title:'About LaserCAD R14', body:'…', actions:[{label:'Close', primary:true}] })` (Sprint 1 entrega o contrato — o dialog real pode ser stub que ainda assim demonstra `dialogs.open`) |

Itens `disabled` **nao** emitem nada quando clicados — apenas mostram o cursor `default` e nao reagem.

## 4. Invariantes e tolerancias

- **Altura 28px.** Fixa em CSS. Padding horizontal 14px por item de menubar (design.md L240).
- **Sem icones nos menus.** Design.md L240: texto puro, sem icones decorativos.
- **Click abre, hover NAO abre.** Design.md L240: "Dropdown abre ao click (nao hover)". Apos um menu aberto, mover o mouse para outro item de menubar pode trocar o dropdown aberto (comportamento OS padrao) — sprint 1 simplifica: para trocar de menu, requer click no novo item.
- **Esc fecha dropdown.** Listener global de `keydown`: se `Escape` e ha dropdown aberto, fecha. NAO emite `tool:cancel` quando o objetivo eh fechar menu.
- **Click fora fecha.** Listener em `document.click` em fase capture: se o target nao esta dentro do menubar nem do dropdown aberto, fecha.
- **Item disabled.** Renderizado em `--text-disabled` (#4A3E66). Atributo `aria-disabled="true"`. `cursor: default`. Sem hover-state.
- **Item habilitado.** Cor `--text-primary` (#E8DDFF). Hover: fundo `--laser-dim` (#3D0099), texto `--text-primary` (design.md L251).
- **Atalhos alinhados a direita** no dropdown, em `--text-secondary` mono 11px (design.md L251). Sprint 1: o texto do atalho eh informativo; nao emite eventos. O atalho **funcional** vem de `app.shortcuts`.
- **Largura minima do dropdown:** 220px (design.md L251). Fundo `--bg-elevated` (#1A1030). Borda 1px `--border-subtle`. Sem sombra (design.md L251).
- **Apenas 1 dropdown aberto por vez.** Abrir o segundo fecha o primeiro.
- **Acessibilidade.** Menubar tem `role="menubar"`, cada item tem `role="menuitem"`, dropdown tem `role="menu"`. `aria-haspopup="menu"` nos titulos. `Tab` percorre apos statusbar (design.md L339).
- **Eventos do bus.**
  - Emite: `command:submit` (apenas para Zoom extents), nenhum outro evento canonico diretamente.
  - Consome: `toggle:changed` (para refletir check visual `◉/○` ao lado dos 3 itens View → Toggle \*), `app:ready` (montar pode esperar isso ou ser sincrono — bootstrap chama `mount` antes de `emit('app:ready')`, entao consumir nao eh estritamente necessario).
  - Nada de eventos fora da lista de state-contract.md §3.

## 5. Exemplos de uso

```js
// Bootstrap monta:
LaserCAD.ui.menubar.mount(document.getElementById('menubar-host'));

// Click no titulo "View" abre o dropdown View:
document.querySelector('[data-menu="view"]').click();
// dropdown visivel, com itens listados.

// Click em "Toggle grid" no dropdown:
// 1) handler interno
// 2) LaserCAD.app.state.setToggle('grid', !state.toggles.grid)
// 3) state emite 'toggle:changed'
// 4) menubar fecha o dropdown (efeito padrao apos selecao)

// Click em "About":
// 1) LaserCAD.ui.dialogs.open({ id:'about', ... })

// Abrir/fechar programaticamente:
LaserCAD.ui.menubar.open('help');
LaserCAD.ui.menubar.close();
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Render correto.** `document.querySelectorAll('#menubar-host [role="menuitem"][data-menu]').length === 6` (os 6 titulos).
2. **Altura 28px.** `getBoundingClientRect()` do menubar-host devolve altura 28.
3. **Dropdown abre ao click, nao no hover.** Mover mouse sobre `View`: nada acontece. Click em `View`: dropdown visivel.
4. **Itens habilitados.** Dentro do dropdown View, `Zoom extents`, `Toggle grid`, `Toggle snap`, `Toggle ortho` tem cor `--text-primary`. Outros itens em cinza `--text-disabled` com `aria-disabled="true"`.
5. **Click em `Toggle grid` muda state e fecha dropdown.** Antes: `state.toggles.grid === true`. Click. Depois: `state.toggles.grid === false`, dropdown fechado, bus emitiu `toggle:changed { name:'grid', value:false }`.
6. **Click em `Zoom extents` emite `command:submit`.** Subscriber `LaserCAD.bus.on('command:submit', console.log)` ouve `{ raw:'zoom extents', parsed:{kind:'view', action:'zoomExtents'} }`.
7. **Click em `About` abre dialog.** `LaserCAD.ui.dialogs.open` chamado. Sprint 1: aceitavel que o dialog seja visualmente vazio/stub, mas a chamada deve ser observavel (instrumentar wrapper ou inspecionar log).
8. **`Esc` fecha dropdown.** Abrir View, pressionar `Esc`: dropdown some.
9. **Click fora fecha.** Abrir View, clicar na area do viewport: dropdown some.
10. **Apenas 1 aberto.** Abrir View, clicar em Help: View fecha, Help abre.
11. **Item disabled nao reage.** Click em `New` (File → New): nada emitido no bus, `state` inalterado.
12. **Atalho alinhado a direita.** Inspecao visual: `Z E` aparece a direita de `Zoom extents`; cor `--text-secondary`; tipografia mono.

## 7. Notas de implementacao

- Design.md L238–251 eh a fonte autoritativa.
- O briefing autoriza este adendo: design.md exige menubar, mas plan.md L196–200 lista so 4 modulos de UI. O ADR 0001 nao impede a adicao de `ui.menubar` ao namespace; ela cabe naturalmente em `ui/` por ser chrome.
- Atualizar a tabela §3.3 de `specs/_conventions/namespace.md` ja foi feita (posicao #25 reservada). Esta spec confirma o uso.
- Os 6 itens enabled foram escolhidos para que **toda funcionalidade exposta na menubar seja exercitavel na Sprint 1**: zoom extents emite comando que a Sprint 1 ja reconhece, os 3 toggles passam por caminho oficial, e About valida o contrato de `dialogs`.
- Apos `Toggle snap/grid/ortho`, alguns CADs marcam o item com `◉` a esquerda do label refletindo estado on/off. Sprint 1 entrega este marcador (consumindo `toggle:changed` para re-render). Posicao: `◉ Toggle grid` (on) / `○ Toggle grid` (off). Cor: `--laser-450` (on) / `--text-secondary` (off), mesma logica da statusbar (design.md L231–L234).
- Tudo passa por eventos canonicos. Nenhuma extensao do bus.
- Plan.md L223 / design.md L321–331 nao se aplicam diretamente: menubar nao tem maquina de estado propria alem de "aberto / fechado".
