# index.html

## 1. Responsabilidade
Definir a casca HTML do LaserCAD R14: declaracao de documento, metadados, regioes DOM fixas com IDs canonicos, ordem de carga das tres folhas CSS e dos **28 scripts** classicos. Abre por **duplo-clique** (protocolo `file://`) sem build, sem servidor, sem `type="module"`.

## 2. Dependencias
- runtime: nao tem (e o ponto de entrada que carrega tudo).
- carga de CSS: `assets/css/reset.css` → `assets/css/theme.css` → `assets/css/app.css` (nessa ordem; ver dependencias entre folhas em `assets/css/app.md`).
- carga de JS: a sequencia exata dos 28 `<script src="...">` segue **integralmente** a Tabela §3 de `specs/_conventions/namespace.md`. Esta spec **referencia** essa tabela como autoridade; nao duplica nem reordena.

## 3. API publica

Esta spec descreve markup, nao funcoes. As "APIs publicas" sao:

- **Estrutura de regioes DOM com IDs estaveis** (vide §3.1).
- **Atributos de documento obrigatorios** (vide §3.2).
- **Ordem dos `<script>`** (referencia §3.3).

### 3.1 Regioes DOM e IDs canonicos

Cinco regioes fixas, mapeadas a `design.md` L80-L112:

| ID | Tag | Conteudo | Origem |
|---|---|---|---|
| `#menubar`        | `<nav>`     | File / Edit / View / Draw / Modify / Help (texto puro, sem icones) | `design.md` L80-L82, L108, L238-L251 |
| `#toolbar`        | `<aside>`   | Coluna vertical de 10 botoes-ferramenta SVG inline | `design.md` L83-L95, L109, L120-L155 |
| `#viewport-host`  | `<main>`    | Container do `<svg>` raiz montado por `render.svgRoot.mount()` | `design.md` L85-L95, L110, L157-L185 |
| `#command-line`   | `<section>` | 3 linhas mono: historico + prompt + input | `design.md` L96-L99, L111, L187-L218 |
| `#statusbar`      | `<footer>`  | Coordenadas mm, toggles SNAP/GRID/ORTHO, indicador autosave | `design.md` L100-L102, L112, L220-L236 |

Cada regiao tem `grid-area` correspondente em `assets/css/app.css` (ver `assets/css/app.md` §3).

### 3.2 Atributos de documento obrigatorios

```html
<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="color-scheme" content="dark" />
  <title>LaserCAD R14</title>

  <link rel="stylesheet" href="assets/css/reset.css" />
  <link rel="stylesheet" href="assets/css/theme.css" />
  <link rel="stylesheet" href="assets/css/app.css" />
</head>
<body>
  <nav     id="menubar"></nav>
  <aside   id="toolbar"></aside>
  <main    id="viewport-host"></main>
  <section id="command-line"></section>
  <footer  id="statusbar"></footer>

  <!-- 28 scripts classicos: ver §3.3 -->
</body>
</html>
```

- `lang="pt-BR"`: a UI e em portugues do Brasil; prompts da command line seguem o vocabulario R14 ("Specify first point:", "Specify next point or [Undo]:") em ingles por fidelidade cultural (`design.md` L74, L194-L197) — esse e um overlay de string, nao o idioma da pagina.
- `<meta name="color-scheme" content="dark">`: dica para o SO de que a aplicacao e dark-only (`design.md` L11). Nao tem fallback de tema claro.
- Sem `<meta name="theme-color">` (irrelevante em `file://`, redundante com CSS).
- Sem `<script type="module">` em nenhum lugar — proibido pelo ADR 0001 §3.

### 3.3 Ordem exata dos `<script src="...">`

A sequencia obrigatoria dos 28 arquivos esta normativamente em **`specs/_conventions/namespace.md` §3**. Este `index.html` **deve** declarar exatamente esses 28 `<script src="...">` na exata ordem da tabela ali — qualquer alteracao se faz primeiro em `namespace.md` e depois aqui. Esta spec nao duplica a lista; consulte a fonte unica de verdade.

Forma esperada de cada `<script>`:

```html
<script src="<caminho>" onerror="console.error('[LaserCAD] script falhou:', '<caminho>')"></script>
```

- Sem `type="module"`.
- Sem `defer`/`async` — a ordem manual exige carga sincrona conforme aparece no DOM (`plan.md` ADR 0001 §3.4: "Ordem manual obrigatoria").
- Sem `crossorigin`, sem `integrity` — `file://` nao aplica CORS aqui e nao ha CDN.
- O `onerror` inline e a **mitigacao do risco de erro silencioso em `file://`** (alguns browsers escondem erros de rede neste protocolo); a string passada precisa ser o caminho do arquivo, para diagnostico em campo.

### 3.4 Conteudo inicial das regioes

Sprint 1 **nao** popula nenhuma das 5 regioes em HTML estatico — todas comecam vazias e sao preenchidas pelo respectivo modulo:

- `#menubar`: populado por `ui.menubar` (Sprint 2/3, ja existe spec WS-C).
- `#toolbar`: populado por `ui.toolbar` (idem).
- `#viewport-host`: populado pelo `<svg>` que `render.svgRoot.mount(host)` injeta como filho unico.
- `#command-line`: populado por `ui.commandLine`.
- `#statusbar`: populado por `ui.statusbar`.

A unica obrigacao do `index.html` e existir como contineres com `id` corretos e tamanhos governados pelo grid CSS.

## 4. Invariantes e tolerancias

- **`file://` first.** Abrir `index.html` por duplo-clique em Chrome/Edge/Firefox modernos deve carregar 28 scripts e 3 CSS sem erro. Qualquer construcao incompativel com `file://` e proibida (ADR 0001 §3).
- **Sem `<script type="module">`, sem `import`, sem `export`** em nenhuma parte do projeto referenciada por este HTML.
- **Sem `defer`/`async`**: a ordem de carga e estritamente sincrona, na ordem dos `<script>` no DOM. Inverter dois requer atualizar a tabela em `namespace.md` primeiro.
- **Os 5 IDs canonicos sao fixos**; renomear quebra os modulos de UI/render.
- **Nada de inline script de aplicacao.** O unico inline aceito e o atributo `onerror` de cada `<script src>` para logar falha de carga em `file://`.
- **CSS na ordem reset → theme → app**: app.css depende dos tokens `--*` definidos em theme.css; reset.css zera defaults do browser e tem que vir antes.
- **Sem favicon, sem manifest, sem service worker** no MVP (irrelevantes em `file://` e fora do escopo Sprint 1).
- **Sem `<noscript>`** com mensagem de erro — supondo browser moderno; um `<noscript>` invisivel seria ruido.
- **Sem `<base href="">`**: caminhos relativos resolvem a partir do diretorio do HTML, que e o que `file://` espera.

## 5. Exemplos de uso

```html
<!-- Exemplo de bloco final de body (apenas 4 dos 28 scripts mostrados; lista completa em namespace.md §3) -->
  <script src="src/core/geometry/epsilon.js"
          onerror="console.error('[LaserCAD] script falhou:', 'src/core/geometry/epsilon.js')"></script>
  <script src="src/core/geometry/vec2.js"
          onerror="console.error('[LaserCAD] script falhou:', 'src/core/geometry/vec2.js')"></script>
  <!-- ... (24 scripts intermediarios; ver namespace.md §3) ... -->
  <script src="src/app/bootstrap.js"
          onerror="console.error('[LaserCAD] script falhou:', 'src/app/bootstrap.js')"></script>
  <script src="src/main.js"
          onerror="console.error('[LaserCAD] script falhou:', 'src/main.js')"></script>
</body>
</html>
```

## 6. Criterios de aceitacao testaveis manualmente

1. Salvar o repositorio em qualquer pasta local, abrir `index.html` por **duplo-clique** (URL comeca com `file://`): a pagina carrega sem mensagem de erro no console; `window.LaserCAD` existe no console e tem as chaves `core, render, tools, ui, io, app, bus`.
2. No DevTools (aba Network), confirmar que **28** requisicoes JavaScript e **3** requisicoes CSS sao feitas em sequencia ordenada — nao paralelas indevidamente (sem `defer`/`async`).
3. Verificar que `document.querySelectorAll('#menubar, #toolbar, #viewport-host, #command-line, #statusbar').length === 5`.
4. Buscar por `type="module"` no fonte de `index.html`: zero ocorrencias.
5. Renomear um dos arquivos JS (ex.: `src/core/geometry/vec2.js` → `vec2_x.js`) e recarregar: o console mostra `[LaserCAD] script falhou: src/core/geometry/vec2.js` (mensagem do `onerror`).
6. `document.documentElement.lang === 'pt-BR'` e `document.querySelector('meta[name="color-scheme"]').content === 'dark'`.

## 7. Notas de implementacao

- Justificativa do `file://` first e proibicao de `type="module"`: ADR 0001 §3 (substitui as orientacoes de `plan.md` L23, L142, L220, L228-L235).
- `xmlns` do `<svg>`: nao vai no `index.html` — o `<svg>` raiz e criado dinamicamente por `render.svgRoot.mount()` com `createElementNS` (`plan.md` L9, L19).
- Regioes e seus tamanhos: `design.md` L80-L118; layout governado por CSS Grid em `assets/css/app.md`.
- `onerror` inline e a unica linha de codigo neste HTML; aceitavel porque (a) e mensagem operacional, (b) e identico em todos os 28 scripts, (c) substitui telemetria que nao existe.
- A tabela definitiva de 28 scripts esta em `specs/_conventions/namespace.md` §3 — atualizar **la** primeiro quando adicionar arquivo novo (passo §3.2 daquela spec).
- Sem branding/logo na chrome (`design.md` L116) — o `<title>` "LaserCAD R14" aparece apenas na aba do navegador.
