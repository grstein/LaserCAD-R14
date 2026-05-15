# ADR 0001 — Arquitetura base do LaserCAD R14

Este ADR consolida três decisões estruturais que travam a base técnica do projeto antes de qualquer linha de implementação. As decisões aqui registradas são pré-requisito para os subagentes paralelos da Sprint 1 (WS-A/B/C) e quaisquer mudanças exigem novo ADR.

- **Status global:** Aceito
- **Data:** 2026-05-12
- **Decisores:** Equipe LaserCAD

---

## 1. SVG-first como renderização e exportação

### 1.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 1.2 Contexto

O LaserCAD R14 é um micro-CAD 2D que roda no navegador, cujo objetivo operacional final é gerar arquivos SVG consumíveis pelo LaserGRBL (plan.md L7, L13). A `plan.md` L9 estabelece explicitamente a arquitetura "KISS e SVG-first": modelo geométrico próprio em JavaScript como fonte da verdade, renderização em SVG nativo no DOM e exportação de SVG plain para download local. A `plan.md` L25 reforça SVG como renderização **e** saída, citando SVG 2 (formas básicas equivalentes a paths). A tabela de tecnologias descartadas (plan.md L67–75) elimina explicitamente Canvas-first, Fabric.js/Konva, Paper.js/Two.js como base obrigatória e WebGL — todas pelo mesmo motivo: complicam a exportação SVG limpa exigida pelo LaserGRBL ou adicionam camadas de abstração desnecessárias entre o modelo e o SVG final.

### 1.3 Decisão

Adotar **SVG como única tecnologia de renderização e exportação**. O viewport (design.md L110, L159) é um `<svg>` nativo no DOM, com `viewBox` em milímetros (plan.md L26, L274). O exportador serializa essencialmente o mesmo SVG já renderizado, com flatten de transformações e normalização para "plain SVG" conforme checklist (plan.md L291–305). Não há Canvas, não há WebGL, não há bibliotecas de renderização intermediárias.

### 1.4 Consequências

- O viewport é um único elemento `<svg>` controlado via `document.createElementNS()` (plan.md L19), com filhos criados, atualizados e removidos diretamente no DOM SVG.
- O renderer (`src/render/`) mapeia entidades do modelo para nós SVG: `line` → `<line>`, `circle` → `<circle>`, `arc` → `<path d="M ... A ...">` (plan.md L246, L297).
- O exportador (Sprint Edit and Export, plan.md L121) reaproveita a árvore SVG já existente, flatten de `transform` e força `fill="none"` (plan.md L298).
- Não é possível usar técnicas baseadas em raster (Canvas 2D, WebGL, OffscreenCanvas) para acelerar render ou efeitos — qualquer efeito visual precisa caber no que SVG 2 oferece nativamente.
- A performance fica limitada ao que o motor SVG do navegador entrega. Mitigação: limitar o `<g>` ativo ao visível por câmera (plan.md L265) só se necessário; index espacial fica fora do MVP.
- A consistência entre o que se vê e o que se exporta é estrutural — não acidental — porque é a mesma árvore.

---

## 2. Milímetros como unidade canônica do documento e exportação

### 2.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 2.2 Contexto

LaserCAD é desenho técnico para corte a laser. A `plan.md` L26 estabelece "Precisão em mm: documento, comandos e exportação em mm; `viewBox` define user space". A `plan.md` L103 fixa como critério de aceitação do MVP que "o usuário abre um documento novo, enxerga grid/cursor e mede tudo em milímetros". A `plan.md` L217 normatiza: "mm em todo o documento e exportação; pixel só na câmera/render". A `plan.md` L218 separa ângulos: UI em graus, kernel em radianos. A área de referência recorrente é 128×128 mm (plan.md L346, design.md L102 indiretamente via exemplos).

### 2.3 Decisão

**Milímetros são a unidade canônica** do modelo do documento, da entrada do usuário, das ferramentas, do kernel geométrico e da exportação SVG. Pixels só aparecem na camada de câmera/viewport e em conversões `screen ↔ world` via `getScreenCTM()` (plan.md L19). Ângulos seguem a regra split: **graus na UI** (entrada/exibição) e **radianos no kernel** (cálculos trigonométricos).

### 2.4 Consequências

- O `<svg>` raiz declara `viewBox` em coordenadas-mundo (mm) — ex.: `viewBox="0 0 128 128"` para um documento 128×128 mm (plan.md L280, L346).
- O exportador SVG escreve `width="128mm" height="128mm"` no elemento raiz (plan.md L278–280, L294).
- O schema do documento (`core/document/schema.js`) descreve todas as coordenadas, distâncias, raios e comprimentos como números em mm, sem sufixo, sem unidade embutida.
- O kernel (`core/geometry/*`) opera exclusivamente em mm. Tolerância `EPS` (plan.md L225) é fixada em mm (ex.: `1e-6` mm).
- Conversão `mm ↔ pixel` é responsabilidade exclusiva de `render/camera.js`. Nenhum módulo de `core/` ou `tools/` deve conhecer pixel.
- A command line aceita coordenadas em mm sem sufixo: `124.5,87.3` significa `(124.5 mm, 87.3 mm)` (design.md L194, L215).
- Ângulos: o usuário digita "30" e a UI converte para `π/6 rad` antes de entregar ao kernel. Status bar e dimensions overlay exibem graus.
- A status bar exibe coordenadas em mm com 3 casas decimais: `124.500, 87.300 mm` (design.md L102, L225).
- Stroke width visual escala com zoom; valor lógico de exportação é fixo `0.1 mm` (design.md L167, L266; plan.md L281).

---

## 3. Abandono de ES modules em favor de scripts clássicos com namespace global

### 3.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 3.2 Contexto

A `plan.md` foi escrita assumindo `<script type="module" src="./src/main.js">` e operação via servidor estático local. A linha 228 alerta explicitamente: "para módulos nativos, o app deve rodar por servidor local. A MDN alerta que carregar HTML por `file://` produz erros de CORS em módulos". A `plan.md` L232–234 cristalizava `python -m http.server 8080` como pré-requisito de desenvolvimento.

O requisito operacional do LaserCAD R14, porém, é que `index.html` abra por **duplo-clique** em qualquer navegador moderno (protocolo `file://`), sem servidor, sem dependência externa, sem build step. Esse requisito colide diretamente com `<script type="module">`, que falha em `file://` por restrições de CORS aplicadas ao loader de módulos.

A escolha é binária:

1. Manter ES modules e exigir servidor local (a `plan.md` original).
2. Trocar para `<script src="...">` clássicos e namespace global (esta decisão).

### 3.3 Decisão

**Trocar ES modules por scripts clássicos com namespace global `window.LaserCAD`.** Cada arquivo é uma IIFE que atribui sub-objetos a `window.LaserCAD.<ns>.<nome>`. O `index.html` carrega os arquivos via uma sequência ordenada de `<script src="...">` clássicos, sem `type="module"`, sem `import`, sem `export`. Nenhum bundler. Nenhum build step. A convenção exata de namespace e a ordem dos scripts ficam em `specs/_conventions/namespace.md`.

Esta decisão **substitui** as orientações da `plan.md` L23, L142, L220, L228–235 que assumiam módulos nativos. A pasta `src/` permanece como em `plan.md` L144–212 (organização modular por domínio mantida — só o mecanismo de linkage muda).

### 3.4 Consequências

- **Ordem manual obrigatória.** O `index.html` declara `<script src="...">` em ordem das folhas para as raízes (quem ninguém depende vai primeiro). A sequência canônica é fixada em `specs/_conventions/namespace.md` e cada novo arquivo precisa ser inserido na posição correta.
- **Sem `import`/`export`.** Cada arquivo é uma IIFE no formato `(function(LaserCAD){ ... })(window.LaserCAD = window.LaserCAD || { ... });`. Nada de variáveis globais soltas.
- **Sem bundler, sem build step.** O repositório é servido diretamente como está, e `index.html` abre por duplo-clique. CI valida apenas sintaxe, testes e smoke; não há etapa de empacotamento.
- **Dependências explícitas e frágeis.** O autor de `line.js` precisa garantir que `vec2.js` carregou antes — o sistema não detecta sozinho. Mitigação: ordem documentada em `namespace.md` e cada IIFE checa pré-condições básicas (`if (!LaserCAD.core.geometry.vec2) throw ...`) durante desenvolvimento.
- **Sem tree-shaking.** Todo arquivo listado em `index.html` carrega sempre. Aceitável enquanto o bundle estiver < 200 KB minificado (não há minificação no MVP, então < 200 KB cru); revisitar esta decisão quando ultrapassar esse limite.
- **IDEs/linters menos amigáveis sem `import/export`** — autocompletar e go-to-definition ficam parciais. Mitigação: **JSDoc com `@typedef`** (plan.md L224) descrevendo entidades, payloads de eventos e formas do estado; cada arquivo declara um cabeçalho com `@typedef`s relevantes e usa `@param`/`@returns` JSDoc em todas as funções públicas. ESLint configurado para reconhecer `window.LaserCAD` como global lido.
- **Convenção operacional de teste:** todo módulo é inspecionável em `window.LaserCAD.*` no DevTools — ver `specs/_conventions/namespace.md` para exemplos.
- **CI ajustado:** os jobs `static-check` e `unit-geometry` (plan.md L256–257) usam Node + JSDOM ou um shim que injeta `window.LaserCAD` antes de avaliar cada arquivo na ordem canônica. Não há resolução de módulos a fazer.
- **A `plan.md` L23, L142, L220, L228–235 ficam superadas por este ADR.** Documentação derivada deve referenciar este ADR ao tocar nesses pontos.
