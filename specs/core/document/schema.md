# schema (core/document)

## 1. Responsabilidade

Publicar — via JSDoc `@typedef` — a forma normalizada do documento JSON (`entities`, `selection`, `camera`, etc.) e oferecer construtores puros que produzem um documento vazio válido. **Sem mutação de estado**, sem persistência.

## 2. Dependências

- runtime: nenhum sub-namespace anterior. O módulo publica apenas `@typedef`s e helpers de construção.
- ordem de carga: posição 7 em `specs/_conventions/namespace.md` §3 (depois de toda a geometria, antes de `validators`).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.document.schema`.

### 3.1 Typedefs publicados (via JSDoc)

A forma exata do estado já está congelada em `specs/_conventions/state-contract.md` §1.1. Este módulo **referencia** esse documento e republica os typedefs como JSDoc para uso programático/IDE:

```text
@typedef {'line'|'polyline'|'rect'|'circle'|'arc'} EntityType

@typedef {{ id: string, type: 'line',     p1: Vec2, p2: Vec2 }} LineEntity
@typedef {{ id: string, type: 'circle',   center: Vec2, r: number }} CircleEntity
@typedef {{ id: string, type: 'arc',
            center: Vec2, r: number,
            startAngle: number, endAngle: number, ccw: boolean }} ArcEntity
@typedef {LineEntity | CircleEntity | ArcEntity} Entity
   // Sprint 1 cobre apenas line/circle/arc; polyline/rect entram em sprints posteriores

@typedef {{ w: number, h: number }} DocumentBounds
@typedef {{ cx: number, cy: number, zoom: number, viewportW: number, viewportH: number }} Camera

@typedef {{
  schemaVersion: 1,
  units: 'mm',
  documentBounds: DocumentBounds,
  entities: Entity[],
  selection: string[],
  camera: Camera
}} DocumentJSON
```

> O singleton `window.LaserCAD.app.state` é um **superset** de `DocumentJSON` (acresce `activeTool`, `toolState`, `cursor`, `toggles`, `commandHistory`, `commandInput`). Ver `state-contract.md` §1.1 para a forma completa. **Não duplicar a tabela aqui.**

### 3.2 Funções

```text
emptyDocument()                  : DocumentJSON
   // Retorna { schemaVersion: 1, units: 'mm', documentBounds: {w:128, h:128},
   //          entities: [], selection: [],
   //          camera: { cx:0, cy:0, zoom:1, viewportW:0, viewportH:0 } }

isEntityType(t: string)          : boolean
   // true se t ∈ {'line','polyline','rect','circle','arc'}
```

Contratos:

- `emptyDocument()` retorna **objeto novo** a cada chamada (sem singleton compartilhado).
- Nenhuma função muta argumentos.
- Pureza absoluta — **sem DOM, sem `app.state`, sem `bus`** (plan.md L222).

## 4. Invariantes e tolerâncias

- `schemaVersion: 1` literal — qualquer mudança de forma exige incremento e migrador (plan.md L317).
- `units: 'mm'` literal no MVP (ADR 0001 §2; plan.md L217).
- `documentBounds.w`/`h` sempre `> 0`. Default 128×128 mm (plan.md L346; state-contract.md §1.1).
- `entities[*].id` segue padrão `e_<n>` (n inteiro crescente); o counter vive privado em `core.document.commands`, **não** no schema.
- `selection` é subconjunto de `entities[*].id`. Validador conferirá em `core.document.validators`.
- Ângulos em `ArcEntity` em **radianos** (ADR 0001 §2.4; state-contract.md §1.2).
- Schema é **dado**, não comportamento — nenhuma função muta estado externo.

## 5. Exemplos de uso

```js
const S = window.LaserCAD.core.document.schema;

const doc = S.emptyDocument();
// {
//   schemaVersion: 1,
//   units: 'mm',
//   documentBounds: { w: 128, h: 128 },
//   entities: [],
//   selection: [],
//   camera: { cx: 0, cy: 0, zoom: 1, viewportW: 0, viewportH: 0 }
// }

S.isEntityType('line'); // true
S.isEntityType('text'); // false (texto fora do MVP — plan.md L299)
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.document.schema.emptyDocument().schemaVersion === 1`.
2. `window.LaserCAD.core.document.schema.emptyDocument().units === 'mm'`.
3. Duas chamadas a `emptyDocument()` retornam objetos **distintos** (`emptyDocument() !== emptyDocument()`) — sem singleton compartilhado.
4. `window.LaserCAD.core.document.schema.isEntityType('arc')` retorna `true`; `isEntityType('text')` retorna `false`.
5. `emptyDocument().documentBounds` retorna `{w:128, h:128}` — coerente com plan.md L346.

## 6.1. Critério adicional (segurança de referência)

6. Após `const d = S.emptyDocument(); d.entities.push({id:'x'});` chamar `S.emptyDocument()` retorna um documento com `entities: []` (vazio) — confirmando que cada chamada produz arrays/objetos novos.

## 7. Notas de implementação

- O typedef `DocumentJSON` é o **subset persistível** do estado global (`window.LaserCAD.app.state`). Campos transitórios (cursor, toolState, commandInput, etc.) não fazem parte e **não** devem ir para export/autosave (plan.md L219: "JSON normalizado, sem estado derivado persistido").
- O contador de `id` mora em `commands.js` para garantir que toda criação de entidade passe pelo único caminho autorizado (plan.md L226; state-contract.md §1.2).
- Plan.md L346: "área configurável, com presets simples; 128×128 mm aparece apenas como referência recorrente". Sprint 1 trava o default em 128×128; sprints futuras adicionam dialog/preset.
- Este módulo **não** valida; validação fica em `core.document.validators`.
- Polyline/rect aparecem no typedef como reserva sintática mas não têm forma fixada na Sprint 1 — definidas na Sprint Drawing (plan.md L95).
