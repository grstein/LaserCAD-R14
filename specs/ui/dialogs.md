# ui.dialogs

## 1. Responsabilidade

Fornecer a API `open(spec) / close()` para dialogs do LaserCAD R14. Sprint 1 entrega **apenas o contrato e o container**; nenhum dialog concreto eh implementado/renderizado (o About em `menubar.md` pode usar um stub minimo). A intencao eh ter a interface congelada antes da Sprint 5 (Export SVG dialog, design.md L257–274).

## 2. Dependencias

- runtime:
  - `window.LaserCAD.bus`
  - `window.LaserCAD.app.state` (somente leitura — para futuros dialogs)
- ordem de carga: posicao #26 em `specs/_conventions/namespace.md` (depois da menubar, antes de bootstrap).

## 3. API publica

```js
/**
 * @typedef {Object} DialogAction
 * @property {string} label          - Texto do botao.
 * @property {boolean} [primary]     - Se true, estilo "primario" (laser-450). Senao, secundario (transparente).
 * @property {() => boolean|void} [onClick]
 *           - Callback. Se retorna `false`, o dialog NAO fecha (permite validacao).
 *             Default eh fechar.
 */

/**
 * @typedef {Object} DialogField
 * @property {string} id             - Identificador do campo (usado no objeto retornado).
 * @property {'text'|'number'|'radio'|'checkbox'} kind
 * @property {string} label
 * @property {any}    [value]        - Valor inicial.
 * @property {Array<{label:string, value:any}>} [options] - Para 'radio'.
 */

/**
 * @typedef {Object} DialogSpec
 * @property {string} id             - Id unico do dialog (ex.: 'about', 'export-svg').
 * @property {string} title          - Titulo do header (16px Inter 600).
 * @property {string|HTMLElement} [body]
 *           - String (texto curto) ou elemento DOM (corpo customizado). Mutually exclusive com `fields`.
 * @property {Array<DialogField>} [fields] - Campos estruturados (Sprint 5+).
 * @property {Array<DialogAction>} actions - Botoes do rodape. Pelo menos 1.
 * @property {number} [width]        - Largura em px. Default 400. Faixa permitida 360–480 (design.md L255).
 */

window.LaserCAD.ui.dialogs = {
  /**
   * Registra/cria o container `.dialog-layer` em `document.body`.
   * Idempotente. Chamado pelo bootstrap.
   * @param {HTMLElement} hostBody - document.body
   * @returns {void}
   */
  mount(hostBody) { /* ... */ },

  /**
   * Abre um dialog conforme a spec. Apenas um dialog visivel por vez —
   * abrir outro fecha o atual (sem callback de cancelamento implicito).
   * @param {DialogSpec} spec
   * @returns {{ close: () => void, values: () => Object }}
   *   - `close()`: fecha este dialog (no-op se ja fechado).
   *   - `values()`: snapshot dos valores correntes dos campos (`{ <field.id>: any }`).
   */
  open(spec) { /* ... */ },

  /**
   * Fecha qualquer dialog aberto. Idempotente.
   * @returns {void}
   */
  close() { /* ... */ },

  /**
   * Retorna referencia ao dialog atualmente aberto, ou null.
   * Util para testes.
   * @returns {{ id: string, spec: DialogSpec }|null}
   */
  current() { /* ... */ }
};
```

### 3.1 Comportamento de `open`

1. Se ha um dialog aberto, fecha-o silenciosamente (sem chamar `onClick` de nenhuma `action`).
2. Cria a estrutura DOM dentro do `.dialog-layer`:
   ```text
   <div class="dialog" style="width:<width>px">
     <header>title + close button (✕)</header>
     <main>body or rendered fields</main>
     <footer>actions buttons (right-aligned)</footer>
   </div>
   ```
3. Coloca foco no primeiro elemento focavel (input ou primeiro botao da footer).
4. Registra listener de `Escape` no `document` (somente enquanto aberto) que dispara `close()`.
5. Click no botao `✕` do header tambem fecha sem disparar `onClick` de actions.
6. Click em `action`: chama `action.onClick(values())` se existir; se retorno !== `false`, fecha.

### 3.2 Eventos do bus

Sprint 1: **nenhum**. A abertura/fechamento de dialogs eh estritamente local. Nao ha evento canonico em state-contract.md §3 para `dialog:opened` — e nao se inventa um. Modulos que precisam saber se um dialog esta aberto chamam `LaserCAD.ui.dialogs.current()`.

## 4. Invariantes e tolerancias

- **Sem backdrop escurecido.** Design.md L254: "Sem modal escurecendo o fundo". Apenas a janela do dialog flutua.
- **Largura 360–480px.** Default 400. Se `spec.width < 360` ou `> 480`, clamp (sem erro).
- **Header 36px.** Design.md L255. Titulo 16px Inter 600. Botao `✕` a direita.
- **Botoes do rodape.**
  - Primario (`primary: true`): fundo `--laser-450` (#6E00FF), texto `#FFFFFF`, padding `8px 16px`, border-radius 4px.
  - Secundario: transparente, borda 1px `--border-strong` (#3D2466), texto `--text-primary`.
  - Hover: primario vira `--laser-glow`; secundario ganha borda `--laser-450`.
  - Alinhados a direita; primario por ultimo.
- **Foco em laser-450.** Inputs e botoes recebem outline `--laser-glow` 2px ao receber foco (design.md L338).
- **Apenas 1 dialog visivel.** A intencao eh evitar empilhamento. Abrir outro substitui.
- **Sem animacoes.** Design.md L11: sem transicoes decorativas.
- **Acessibilidade.** Container `role="dialog"`, `aria-modal="true"`, `aria-labelledby` apontando para o `<header>`. `Tab` ciclico dentro do dialog (focus trap).
- **Sprint 1: nenhum dialog concreto eh enviado por padrao.** O unico consumidor previsto eh o `About` da `ui.menubar`, que pode invocar `open` com uma spec minima:

  ```js
  LaserCAD.ui.dialogs.open({
    id: 'about',
    title: 'About LaserCAD R14',
    body: 'LaserCAD R14 — micro-CAD 2D for laser cutting. Sprint 1 alpha.',
    actions: [{ label: 'Close', primary: true }]
  });
  ```

  Aceitavel que o conteudo do body seja mínimo (texto curto). O importante eh o contrato funcionar.
- **Container persistente.** O `.dialog-layer` em `document.body` permanece montado mesmo sem dialog visivel; eh apenas `display: none` enquanto nao ha conteudo. Z-index alto (ex.: 1000) garante sobreposicao a viewport e chrome.

## 5. Exemplos de uso

```js
// Bootstrap chama:
LaserCAD.ui.dialogs.mount(document.body);

// Abrir o About:
const h = LaserCAD.ui.dialogs.open({
  id: 'about',
  title: 'About LaserCAD R14',
  body: 'LaserCAD R14 alpha. Sprint 1.',
  actions: [{ label: 'Close', primary: true }]
});

// Fechar por API:
h.close();
// ou:
LaserCAD.ui.dialogs.close();

// Spec mais elaborada (preview da Sprint 5, valida o contrato hoje):
LaserCAD.ui.dialogs.open({
  id: 'export-svg',
  title: 'Export SVG',
  width: 420,
  fields: [
    { id: 'filename', kind: 'text',   label: 'Filename',     value: 'drawing-001.svg' },
    { id: 'preset',   kind: 'radio',  label: 'Preset',       value: 'cut',
      options: [{label:'Cut', value:'cut'}, {label:'Mark', value:'mark'}, {label:'Engrave', value:'engrave'}] },
    { id: 'stroke',   kind: 'number', label: 'Stroke width', value: 0.1 },
    { id: 'flatten',  kind: 'checkbox', label: 'Flatten transforms', value: true },
  ],
  actions: [
    { label: 'Cancel', onClick: () => true /* fecha */ },
    { label: 'Export', primary: true, onClick: (values) => {
        if (!values.filename) return false;  // bloqueia fechamento
        console.log('would export', values);
      }
    },
  ]
});

// Inspecao:
LaserCAD.ui.dialogs.current();
// → { id: 'export-svg', spec: {...} }
```

## 6. Criterios de aceitacao testaveis manualmente

1. **`mount` cria container.** Apos `app:ready`, `document.querySelector('.dialog-layer')` existe e tem `display:none` (ou semelhante).
2. **`open` exibe dialog.** Chamar `LaserCAD.ui.dialogs.open({ id:'t', title:'Test', body:'hi', actions:[{label:'OK', primary:true}] })`. Visivel: titulo "Test", body "hi", botao "OK". `LaserCAD.ui.dialogs.current().id === 't'`.
3. **Largura clampada.** `open({...width:200, ...})` → largura efetiva 360px (`getBoundingClientRect().width === 360`). `width: 600` → 480.
4. **`close()` fecha.** Chamar `LaserCAD.ui.dialogs.close()`: dialog desaparece, `current() === null`.
5. **Esc fecha.** Abrir dialog, pressionar `Escape`: fecha, `current() === null`.
6. **`✕` no header fecha.** Click no `✕`: fecha.
7. **Click em action chama `onClick` e fecha.** `let v = null; open({ id:'t2', title:'X', body:'', actions:[{label:'Yes', onClick:()=>{v='y'}}] })`. Click em "Yes": `v === 'y'` e dialog fecha.
8. **`onClick` retornando `false` impede fecho.** `open({ id:'t3', ..., actions:[{label:'Block', onClick:()=>false}] })`. Click: dialog permanece aberto.
9. **Abrir outro fecha o atual.** Abrir dialog A, abrir dialog B: A desaparece, B visivel, `current().id === 'B'`.
10. **Sem backdrop.** Inspecao visual: o resto da UI permanece visivel atras do dialog (sem overlay escurecido). Confirmar via `getComputedStyle(document.querySelector('.dialog-layer')).background === 'rgba(0,0,0,0)' || === 'transparent'`.
11. **Focus inicial.** Abrir dialog com input: `document.activeElement` eh esse input. Sem input: eh o primeiro botao da footer.
12. **About via menubar.** Click em Help → About: o dialog About aparece. `LaserCAD.ui.dialogs.current().id === 'about'`.

## 7. Notas de implementacao

- Design.md L253–283 eh a fonte autoritativa do visual; tudo aqui deve aderir.
- A Sprint 5 (plan.md L121) consumira essa API para o Export SVG dialog. Por isso o `fields` ja esta no contrato, ainda que so renderize a partir da Sprint 5. **Sprint 1 nao precisa implementar render de `fields`** — apenas o `body` (string ou HTMLElement). Spec descreve o contrato completo para evitar churn.
- `aria-modal="true"` instrui leitores de tela apesar da ausencia de backdrop visual.
- Focus trap (Tab ciclico) eh feito interceptando `keydown` `Tab` quando o foco esta no ultimo/primeiro elemento focavel do dialog. Implementacao classica; livre de bibliotecas.
- O parametro `body: HTMLElement` permite que o About no Sprint 1 use uma `<div>` simples com texto, e que dialogs futuros injetem layouts customizados.
- Plan.md nao menciona dialogs explicitamente alem do export — design.md L253–283 eh a unica fonte. A Sprint 1 entrega o contrato em vez do dialog concreto.
- Nenhum evento do bus eh emitido ou consumido por dialogs no Sprint 1. Isso eh deliberado para nao inventar `dialog:opened`/`dialog:closed`.
- O click em "Cancel" pode usar `onClick` retornando `true` (default fecha). Para acoes apenas decorativas, omitir `onClick`.
