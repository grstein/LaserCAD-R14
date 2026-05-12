# history (core/document)

## 1. Responsabilidade
Manter as pilhas `past` e `future` de comandos aplicados ao estado, expondo `push`, `undo` e `redo`. Implementa o contrato "undo/redo por comandos, nunca pelo DOM" (plan.md L226).

## 2. Dependências
- runtime: `window.LaserCAD.core.document.commands` (contrato `Command`).
- ordem de carga: depois de `commands` (posição 10 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.document.history`.

```text
@typedef {{ past: Command[], future: Command[], limit: number }} HistoryState

push(state, cmd: Command)         : void
   // Chamado por state.applyCommand DEPOIS de cmd.do(state).
   // Empurra cmd no past; limpa future; aplica limite (FIFO no past).

undo(state)                       : boolean
   // Se past vazio: retorna false (no-op).
   // Caso contrário: pop do past, chama cmd.undo(state), empurra em future, retorna true.

redo(state)                       : boolean
   // Se future vazio: retorna false.
   // Caso contrário: pop do future, chama cmd.do(state), empurra em past, retorna true.

clear(state)                      : void
   // Esvazia past e future. Usado em "novo documento" / load de autosave.

canUndo(state)                    : boolean   // past.length > 0
canRedo(state)                    : boolean   // future.length > 0
```

Contratos:
- `push` **não** chama `cmd.do(state)` — assume que foi feito por `state.applyCommand`.
- `undo`/`redo` chamam `cmd.undo`/`cmd.do` respectivamente.
- Lançam `Error` se `cmd` não respeitar o contrato `Command` (faltam `do` ou `undo`).
- Pureza relativa: o módulo lê/grava `state.history` (ou onde quer que a `HistoryState` viva — ver §7), nunca toca DOM nem `bus` diretamente.

## 4. Invariantes e tolerâncias
- **Limite obrigatório**: `limit = 200` comandos no `past` (plan.md L226 menciona "histórico por comandos"; a especificação técnica fixa 200 — buffer para sessões longas sem inflar memória). Acima disso, o comando mais antigo é **descartado** (FIFO).
- `future` é **sempre limpo** após `push` — qualquer comando novo invalida a linha de redo (comportamento padrão de editores).
- `past` e `future` armazenam comandos por **referência**, não por clone. Comandos precisam ser imutáveis após criação (`meta` em particular, plan.md L226).
- "Undo pelo DOM" é proibido por construção: o DOM não tem método de undo aqui; só o `history`.
- Pureza: **sem DOM, sem `bus`** (plan.md L222).

## 5. Exemplos de uso

```js
const H = window.LaserCAD.core.document.history;
const C = window.LaserCAD.core.document.commands;
const state = window.LaserCAD.app.state;

// applyCommand internamente faz: cmd.do(state); H.push(state, cmd);
state.applyCommand(C.setCamera({cx:10, cy:20, zoom:2}));
H.canUndo(state);                   // true
H.canRedo(state);                   // false

H.undo(state);
state.camera.zoom;                  // 1 (valor anterior)
H.canRedo(state);                   // true

H.redo(state);
state.camera.zoom;                  // 2

H.clear(state);
H.canUndo(state);                   // false
```

## 6. Critérios de aceitação testáveis manualmente

1. **Estado inicial:** após carregar a página, `window.LaserCAD.core.document.history.canUndo(state)` retorna `false`.
2. **Round-trip:** após `state.applyCommand(commands.setCamera({cx:50, cy:50, zoom:3}))`, chamar `history.undo(state)` e depois `history.redo(state)` deixa `state.camera.zoom === 3`.
3. **Invalidação do future:** após `applyCommand → undo → applyCommand(novoCmd)`, `history.canRedo(state)` retorna `false` (a linha de redo foi descartada).
4. **Limite de 200:** após aplicar 250 comandos consecutivos, `history.canUndo(state)` continua `true` mas só 200 undos retrocedem (o 201º retorna `false` e o estado fica no que era depois do comando mais antigo retido).
5. `history.undo(state)` em histórico vazio retorna `false` e **não** modifica `state`.
6. `history.clear(state)` deixa `canUndo === false` e `canRedo === false`.

## 7. Notas de implementação

### Onde mora a `HistoryState`?
Duas opções aceitáveis:

- **A — Privada à IIFE:** `past`, `future`, `limit` são closures internas; `state` é passado só para que `do/undo` possam mutá-lo. O `state` global não expõe `history`.
- **B — Embutida no `state`:** uma propriedade `state.history = {past:[], future:[], limit:200}` que o módulo manipula.

A **Opção A é preferida** porque o histórico é estado de processo, não de documento — não vai para export/autosave nem aparece em `DocumentJSON` (schema.md §3.1). Mantê-lo fora do `state` evita poluir snapshots de debug e o singleton documentado em `state-contract.md` §1.1.

Sprint 1 adota Opção A. Se uma sprint futura precisar inspecionar o histórico (ex.: painel de debug), expor um getter readonly em `history.peek(state)` em vez de espalhar a estrutura.

### Outras notas
- Plan.md L226: "histórico por comandos, nunca 'undo pelo DOM'". A consequência prática é que `delete` em DOM SVG nunca é a fonte da verdade — sempre o commando.
- Plan.md L266 lista risco "Trim/extend quebrar histórico" — mitigado pelo fato de que trim/extend serão comandos compostos (sprint Edit and Export, plan.md L121), não manipulação direta do DOM.
- `limit = 200` é constante interna ao módulo; configurabilidade futura entra via `app.config.historyLimit` se necessário.
- O state-contract.md §1.1 **não** menciona `history` na lista de 12 campos — confirma a Opção A.
