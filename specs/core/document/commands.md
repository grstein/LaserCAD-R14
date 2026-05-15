# commands (core/document)

## 1. Responsabilidade

Definir o contrato de comando reversível (`{type, do, undo, meta}`) e expor a única fábrica autorizada para criar comandos que mutam `entities`/`selection`/`camera` do estado. **Comandos são a única forma legítima de mutar o estado do documento** (state-contract.md §2; plan.md L226).

## 2. Dependências

- runtime: `window.LaserCAD.core.document.schema`, `window.LaserCAD.core.document.validators`.
- ordem de carga: depois de `schema` e `validators` (posição 9 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.document.commands`.

### 3.1 Forma do Command

```text
@typedef {{
  type: string,                    // identificador único: 'noop', 'setCamera', ...
  do: (state) => void,             // aplica a mutação no estado
  undo: (state) => void,           // reverte a mutação no estado
  meta: object                     // payload original + timestamp + qualquer dado para debug
}} Command
```

Contratos do contrato `Command`:

- `do(state)` aplica a mutação no `window.LaserCAD.app.state` (ou um subset, em testes).
- `undo(state)` precisa restaurar **exatamente** o estado pré-`do`. Tolerância: igual byte-a-byte para `entities`/`selection`; igual dentro de `EPS` para `camera`.
- `meta` nunca contém referências mutáveis ao estado externo — é snapshot frio para debug.
- Comando **não emite eventos**; quem chama `state.applyCommand(cmd)` cuida disso.

### 3.2 Fábricas expostas na Sprint 1

```text
noop()                                       : Command
   // type: 'noop', do: () => {}, undo: () => {}. Útil para testes e como
   // placeholder do contrato.

setCamera({ cx, cy, zoom })                  : Command
   // type: 'setCamera', do: salva camera anterior em meta, escreve novos cx/cy/zoom.
   // undo: restaura cx/cy/zoom anteriores. viewportW/H NUNCA são tocados aqui
   // (esses só mudam via state.setViewportSize).
```

Argumentos:

- `setCamera`: `cx`, `cy` finitos em mm; `zoom > 0`. Validado por `validators.isFinitePoint({x:cx, y:cy})` + checagem de `zoom`. Lança `Error` na **fábrica** (não no `do`).

### 3.3 Contador privado de id

O módulo mantém **internamente** (closure-local, não em `state`) um contador inteiro crescente para `id` de entidade:

```text
nextId()                                     : string   // retorna 'e_1', 'e_2', ...
resetIdCounter()                             : void     // só usado em testes/reload de documento
```

O contador é privado da IIFE — nenhum outro módulo o lê ou escreve. `nextId` é consumido por futuras fábricas (`addLine`, `addCircle`, `addArc`) que entram nas sprints Geometry Core/Drawing.

## 4. Invariantes e tolerâncias

- **Único caminho de mutação**: módulos externos chamam `LaserCAD.app.state.applyCommand(cmd)`, **nunca** `cmd.do(state)` direto. Isso garante que `history.push(cmd)` é invocado e o undo/redo funciona.
- O `id` counter **não** vive no `state` (state-contract.md §1.2: "O contador vive privado em `core.document.commands`"). Razão: o counter é metadado de criação, não dado do documento — não persiste em export.
- `do` e `undo` precisam ser idempotentes-em-par: `do; undo; do; undo` deixa o estado igual ao inicial.
- Pureza: o módulo **não** toca DOM nem `bus` (plan.md L222). Toca `state` somente quando `state.applyCommand` chama `cmd.do(state)`.
- Comandos com efeitos colaterais externos (rede, localStorage) são **proibidos** — esses ficam em `io/*` e não passam pelo histórico.

## 5. Exemplos de uso

```js
const C = window.LaserCAD.core.document.commands;
const state = window.LaserCAD.app.state;

// Comando vazio (útil em testes do history)
const nop = C.noop();
state.applyCommand(nop); // history.past cresce em 1, estado igual

// Pan / zoom programático
const cmd = C.setCamera({ cx: 64, cy: 64, zoom: 2 });
state.applyCommand(cmd);
state.camera; // { cx: 64, cy: 64, zoom: 2, viewportW: ..., viewportH: ... }

// Undo via history
window.LaserCAD.core.document.history.undo(state);
state.camera; // { cx: 0, cy: 0, zoom: 1, ... }
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.document.commands.noop()` retorna objeto com `type === 'noop'` e funções `do`/`undo`.
2. **Counter privado:** `window.LaserCAD.core.document.commands.nextId()` retorna `'e_1'` na primeira chamada, `'e_2'` na segunda, etc.; chamadas independentes são monotônicas.
3. **Round-trip do setCamera:** `const cmd = commands.setCamera({cx:10, cy:20, zoom:2}); state.applyCommand(cmd); window.LaserCAD.core.document.history.undo(state)` deixa `state.camera.cx === 0` e `state.camera.zoom === 1` (assumindo valores iniciais).
4. `commands.setCamera({cx: NaN, cy: 0, zoom: 1})` lança um `Error` na fábrica (não silenciosamente).
5. `commands.setCamera({cx:0, cy:0, zoom: 0})` lança um `Error` na fábrica (`zoom > 0` obrigatório).
6. Não há propriedade `commands.idCounter` ou similar publicada — o contador é estritamente privado.

## 7. Notas de implementação

- O par `setCamera` + `noop` na Sprint 1 demonstra o contrato sem antecipar funcionalidade. Sprints futuras adicionam `addEntity(entity)`, `removeEntity(id)`, `setSelection(ids)`, `updateEntity(id, patch)` etc.
- Plan.md L226: "histórico por comandos, nunca 'undo pelo DOM'". O kernel força isso por construção: o DOM não tem método de undo, só o histórico.
- A separação `do(state)` / `undo(state)` em vez de armazenar diffs JSON evita custo de serialização e ambiguidade — cada comando sabe como reverter a si mesmo.
- `meta` deve incluir, no mínimo: `{ at: <Date.now()>, payload: <args originais> }`. Útil para o painel de debug e para o smoke da Sprint Edit/Export.
- A validação no construtor (em vez de no `do`) garante que comandos inválidos **nunca entram** no histórico — não há undo de erro.
- State-contract.md §2.3 lista o esboço `LaserCAD.app.state.applyCommand(cmd)` — é responsabilidade de `app.state` chamar `cmd.do(state)` e empurrar no histórico.
