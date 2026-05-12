# event-bus

## 1. Responsabilidade

Pub/sub minimo em `window.LaserCAD.bus`: registrar subscribers (`on`), remove-los (`off`) e despachar eventos sincronos para todos os subscribers de um tipo (`emit`). Nao decide regras de negocio.

## 2. Dependencias

- runtime: nenhuma. O bus eh folha absoluta — nada depende dele para ser instanciado, e ele nao depende de mais nada.
- ordem de carga: posicao #11 em `specs/_conventions/namespace.md`. Carrega depois de `core/document/history.js` (#10) e antes de `app/state.js` (#12), porque o `state` emite eventos no construtor/setters.

## 3. API publica

```js
window.LaserCAD.bus = {
  /**
   * Registra um subscriber para um tipo de evento.
   * @param {string} event - Nome canonico do evento (ver state-contract.md §3).
   * @param {(payload: any) => void} fn - Callback. Sera chamado sincronamente em cada `emit`.
   * @returns {() => void} unsubscribe - Funcao que, quando chamada, remove esse subscriber (mesmo efeito que `off(event, fn)`).
   */
  on(event, fn) { /* ... */ },

  /**
   * Remove um subscriber previamente registrado.
   * Idempotente: chamar com fn nao registrada nao faz nada nem lanca.
   * @param {string} event
   * @param {(payload: any) => void} fn
   * @returns {void}
   */
  off(event, fn) { /* ... */ },

  /**
   * Despacha o evento sincronamente para todos os subscribers ativos.
   * Subscribers sao invocados na ordem de registro.
   * Excecao em um subscriber nao impede os seguintes (capturada e logada via console.error).
   * @param {string} event
   * @param {any} payload - Forma do payload definida em specs/_conventions/state-contract.md §3.
   * @returns {void}
   */
  emit(event, payload) { /* ... */ }
};
```

Contrato pre/pos:

- **Pre `on`**: `event` deve ser uma string da lista canonica (ver §4). Strings fora da lista sao aceitas tecnicamente, mas violam o contrato e devem ser tratadas como bug de chamador.
- **Pos `on`**: o subscriber esta registrado e recebera todos os `emit` subsequentes ate ser removido.
- **Pre `emit`**: payload deve respeitar a forma da tabela em `state-contract.md` §3. O bus nao valida — confianca contratual.
- **Pos `emit`**: cada subscriber foi invocado exatamente uma vez (modulo excecao individual capturada). Subscribers registrados durante o proprio `emit` **nao** sao chamados nesse ciclo (snapshot da lista no inicio).
- **Pos `off`**: o subscriber removido nao recebera futuros `emit`. Se `off` for chamado durante um `emit`, o subscriber removido ainda pode receber a chamada corrente (snapshot ja capturado).

## 4. Invariantes e tolerancias

- **Lista canonica de eventos eh fechada.** Definida em `specs/_conventions/state-contract.md` §3: `app:ready`, `viewport:resized`, `camera:changed`, `cursor:moved`, `tool:request`, `tool:armed`, `tool:cancel`, `command:submit`, `command:error`, `toggle:changed`. **Inventar evento novo exige ADR** (ver state-contract.md §3.3).
- **Despacho sincrono.** Nao usa `setTimeout`, `queueMicrotask` ou `Promise`. `emit` retorna apenas depois que todos os subscribers terminaram.
- **Subscribers podem se desregistrar dentro do callback.** A iteracao usa snapshot, entao isso eh seguro.
- **`emit` nunca lanca para o chamador.** Excecoes de subscribers sao capturadas individualmente e logadas com `console.error('[bus] subscriber error for', event, err)`.
- **Sem prioridades, sem wildcards, sem namespaces hierarquicos.** Match eh igualdade exata de string.
- **Sem retencao de "ultimo valor".** Subscriber registrado depois do `emit` nao recebe nada retroativo. Quem precisa de estado le `app.state.*`.

## 5. Exemplos de uso

```js
// Registrar interesse em troca de ferramenta
const unsub = LaserCAD.bus.on('tool:armed', (p) => {
  console.log('armed', p.toolId);
});

// Pedir troca de ferramenta (caminho oficial)
LaserCAD.bus.emit('tool:request', { toolId: 'line' });

// Receber coordenadas do cursor na statusbar
LaserCAD.bus.on('cursor:moved', (p) => {
  // p = { worldX, worldY, screenX, screenY }
});

// Sinalizar erro de comando
LaserCAD.bus.emit('command:error', {
  raw: 'arc 30',
  message: '! Not available in Sprint 1'
});

// Desregistrar via retorno de on()
unsub();

// Desregistrar via off() (equivalente)
function handler(p) { /* ... */ }
LaserCAD.bus.on('toggle:changed', handler);
LaserCAD.bus.off('toggle:changed', handler);
```

## 6. Criterios de aceitacao testaveis manualmente

1. **Pub/sub basico.** No DevTools depois do load: `let n = 0; LaserCAD.bus.on('cursor:moved', () => n++); LaserCAD.bus.emit('cursor:moved', { worldX:0, worldY:0, screenX:0, screenY:0 });` — `n === 1`.
2. **Multiplos subscribers, ordem preservada.** `const log = []; LaserCAD.bus.on('app:ready', () => log.push('a')); LaserCAD.bus.on('app:ready', () => log.push('b')); LaserCAD.bus.emit('app:ready', {});` — `log` eh `['a','b']`.
3. **Excecao isolada.** `LaserCAD.bus.on('command:error', () => { throw new Error('x'); }); let ok = false; LaserCAD.bus.on('command:error', () => ok = true); LaserCAD.bus.emit('command:error', { raw:'', message:'! test' });` — console mostra `[bus] subscriber error for command:error` e `ok === true`.
4. **`off` remove e `on` retorna unsub.** `function h(){ throw new Error('should not run'); } const u = LaserCAD.bus.on('toggle:changed', h); u(); LaserCAD.bus.emit('toggle:changed', { name:'grid', value:false });` — nada lanca.
5. **Subscriber registrado durante emit nao recebe no ciclo atual.** `let count = 0; LaserCAD.bus.on('app:ready', () => { LaserCAD.bus.on('app:ready', () => count++); }); LaserCAD.bus.emit('app:ready', {});` — `count === 0` no fim do emit (sera 1 no proximo).
6. **Inspecao manual de subscribers** (modo desenvolvimento): se a implementacao expuser uma propriedade interna para inspecao (ex.: `LaserCAD.bus._subscribers`), ela deve ser tratada como debug-only e nao parte do contrato publico.

## 7. Notas de implementacao

- Decisao deliberada de **nao incluir** `state:changed` generico (state-contract.md §3.2). Cada mutacao relevante tem evento especifico.
- O bus eh o substrato que viabiliza ADR 0001 §3: sem `import`, modulos se comunicam por strings, e o bus eh o unico ponto de roteamento.
- Plan.md L223 documenta a maquina `idle/armed/preview/commit/cancel` — eventos `tool:armed`/`tool:cancel` espelham transicoes desta maquina; `tool:request` eh o pedido inicial.
- Design.md L207–209 fundamenta `command:submit` (Enter/Espaco) e o ciclo de prompts.
- A regra "lista fechada" eh dura: se um modulo precisar comunicar algo novo, primeiro vem um ADR de extensao da tabela §3 do state-contract; nao se introduz evento ad-hoc.
- Em testes Node/JSDOM (CI), o mesmo `bus` carrega normalmente como IIFE; o shim apenas injeta `window.LaserCAD = {}` antes.
