# validators (core/document)

## 1. Responsabilidade
Publicar funções puras de validação do documento e de suas peças — `isValidDoc`, `isFinitePoint`, `assertMm` — usadas por comandos, autosave (sprint futura) e shim de testes. Sprint 1 apenas **publica as assinaturas**; a implementação pode ser mínima (no MVP basta o esqueleto correto + 1 ou 2 checagens críticas usadas por `commands`).

## 2. Dependências
- runtime: `window.LaserCAD.core.document.schema` (typedefs), `window.LaserCAD.core.geometry.epsilon`, `window.LaserCAD.core.geometry.vec2`.
- ordem de carga: depois de `schema` (posição 8 em `specs/_conventions/namespace.md` §3).

## 3. API pública

Tudo exposto sob `window.LaserCAD.core.document.validators`.

```text
isFinitePoint(p)                      : boolean
   // true se p tem campos x, y numéricos e finitos (Number.isFinite).
   // Não verifica tipo do objeto além disso.

isValidDoc(doc)                       : boolean
   // true se doc respeita schema (DocumentJSON):
   // - schemaVersion === 1
   // - units === 'mm'
   // - documentBounds.w > 0 && .h > 0
   // - Array.isArray(entities), Array.isArray(selection)
   // - cada entity tem id (string não vazia) + type ∈ tipos válidos
   // - cada selection[i] referencia um entities[*].id existente
   // - camera tem cx/cy/zoom finitos, zoom > 0; viewportW/H >= 0

assertMm(x)                           : number
   // Se Number.isFinite(x): retorna x.
   // Caso contrário: lança Error('assertMm: expected finite mm value, got <x>').
   // Helper para construtores de comandos que recebem valores numéricos em mm.

isValidEntity(e)                      : boolean
   // dispatch por e.type: chama check específico (linha, círculo, arco).
   // Sprint 1 cobre line/circle/arc; polyline/rect retornam false (não suportados ainda).

reasonsInvalidDoc(doc)                : string[]
   // OPCIONAL na Sprint 1: lista textual de motivos quando isValidDoc é false.
   // Útil para mensagem de erro do autosave futuro.
```

Contratos:
- Todas as funções são **puras**: não modificam argumentos, não tocam estado externo.
- Em caso de violação grave (NaN em ponto, doc malformado), funções **booleanas** retornam `false` em vez de lançar; `assertMm` é a exceção que lança.
- `isValidDoc` é tolerante a campos extras (não conhecidos) — só requer que os obrigatórios estejam corretos. Razão: forward-compat com migrações futuras.

## 4. Invariantes e tolerâncias
- Unidades em mm presumidas. `assertMm` valida finitude, não magnitude (`-1e6` mm é "válido" — só geometria absurda).
- Tolerância `EPS` é usada em comparações de magnitude (ex.: `r > EPS` para círculo/arco).
- Pureza absoluta: **sem DOM, sem `app.state`, sem `bus`** (plan.md L222).
- Validators **não** corrigem dados — só dizem se são válidos. Coerções/normalizações ficam em `commands`.
- Sprint 1: as funções podem ter implementação **minimal** (suficiente para `commands.setCamera` validar entrada); a cobertura completa do schema vem na Sprint Geometry Core ou Edit/Export.

## 5. Exemplos de uso

```js
const V = window.LaserCAD.core.document.validators;

V.isFinitePoint({x: 10, y: 20});       // true
V.isFinitePoint({x: NaN, y: 0});       // false
V.isFinitePoint({x: 1});               // false (y faltando)

V.assertMm(64);                        // 64
V.assertMm(Infinity);                  // lança Error
V.assertMm('64');                      // lança Error (string)

V.isValidDoc(window.LaserCAD.core.document.schema.emptyDocument());
// → true

V.isValidEntity({id:'e_1', type:'line',
                 p1:{x:0,y:0}, p2:{x:10,y:0}});
// → true
```

## 6. Critérios de aceitação testáveis manualmente

1. `window.LaserCAD.core.document.validators.isFinitePoint({x:1,y:2})` retorna `true`; `isFinitePoint({x:1})` retorna `false`; `isFinitePoint({x:NaN, y:2})` retorna `false`.
2. `window.LaserCAD.core.document.validators.assertMm(123.456)` retorna `123.456`; `assertMm(NaN)` lança um `Error`.
3. `window.LaserCAD.core.document.validators.isValidDoc(window.LaserCAD.core.document.schema.emptyDocument())` retorna `true`.
4. `validators.isValidDoc({schemaVersion: 2, units: 'mm'})` retorna `false` (schemaVersion errado).
5. `validators.isValidEntity({id:'e_1', type:'polyline', points:[]})` retorna `false` na Sprint 1 (polyline ainda não suportada).
6. Toda função pública é uma função (`typeof === 'function'`), mesmo que a implementação interna seja stub na Sprint 1.

## 7. Notas de implementação
- A Sprint 1 só **publica as assinaturas e contratos**. A intenção é que `commands.setCamera` já consuma `isFinitePoint` e `assertMm` no construtor para evitar lixo entrando no histórico. As checagens completas de `isValidDoc`/`isValidEntity` ganham implementação real quando o autosave entrar (Sprint Edit/Export, plan.md L121).
- Plan.md L270 lista risco "autosave corromper documento" — `isValidDoc` antes de gravar e antes de carregar é a mitigação direta.
- `reasonsInvalidDoc` é opcional na Sprint 1 porque ainda não há UI para exibir a mensagem; quando entrar, evita "documento inválido" sem contexto.
- A separação `assertMm` (lança) vs `isFinitePoint` (booleano) reflete: assertions são para erros de programação (caller corrupto); booleanos são para dados externos (autosave, import futuro).
- Polyline e rect retornam `false` deliberadamente até a Sprint Drawing fixar suas formas (plan.md L95).
