# assets/css/app.css

## 1. Responsabilidade
Definir o **layout CSS Grid** das 4 regioes fixas do LaserCAD R14 sobre o `<body>` (menubar / toolbar / viewport / command line / status bar), as regras visuais nao-cobertas por `theme.css` (fundos das regioes, divisoes, padding), e o estilo base de elementos compostos (botoes de ferramenta, items de menu, prompts da command line).

## 2. Dependencias
- carga: **terceira** das tres folhas CSS — depois de `reset.css` (estrutura) e depois de `theme.css` (tokens). Ver `index-html.md` §3.2.
- consome tokens de `theme.css`: todas as cores e tipografia entram por `var(--*)` — `app.css` nunca declara hex direto.

## 3. API publica

`app.css` nao expoe funcoes; o "API" e o conjunto de classes/IDs estilizados e a malha de grid. Lista exata:

### 3.1 Grid do body

```css
body {
  display: grid;
  grid-template-rows: 28px auto 66px 24px;     /* menubar, viewport, command, statusbar */
  grid-template-columns: 40px 1fr;             /* toolbar | restante */
  grid-template-areas:
    "menu    menu"
    "tools   view"
    "tools   cmd"
    "status  status";
  background: var(--bg-canvas);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: var(--font-base);
}

#menubar       { grid-area: menu;   background: var(--bg-chrome);  border-bottom: 1px solid var(--border-subtle); }
#toolbar       { grid-area: tools;  background: var(--bg-chrome);  border-right:  1px solid var(--border-subtle); }
#viewport-host { grid-area: view;   background: var(--bg-canvas);  position: relative; overflow: hidden; }
#command-line  { grid-area: cmd;    background: var(--bg-chrome);  border-top:    1px solid var(--border-subtle); border-left: 1px solid var(--border-subtle); }
#statusbar     { grid-area: status; background: var(--bg-chrome);  border-top:    1px solid var(--border-subtle); }
```

### 3.2 IDs e classes estilizadas

| Seletor | Origem visual |
|---|---|
| `#menubar` (28 px) — itens texto 13 px Inter, padding horizontal 14 px | `design.md` L80-L82, L108, L240 |
| `#toolbar` (40 px largura, coluna vertical) — botoes 40×32 px com icone 24×24 px | `design.md` L83-L95, L109, L122-L155 |
| `#viewport-host` (recebe `<svg>` filho unico via `render.svgRoot.mount`) | `design.md` L85-L95, L110 |
| `#command-line` (66 px = 3 linhas mono, padding 4 px) | `design.md` L96-L99, L111, L188-L218 |
| `#statusbar` (24 px, mono 11 px, divisores `\|` em `--border-subtle`) | `design.md` L100-L102, L112, L220-L236 |
| `.tool-button`, `.tool-button.is-active`, `.tool-button:hover`, `.tool-button:disabled` | `design.md` L124-L130 |
| `.menu-item`, `.menu-item:hover`, `.menu-dropdown` | `design.md` L240-L251 |
| `.cmd-history`, `.cmd-prompt`, `.cmd-input` | `design.md` L195-L213 |
| `.status-coords`, `.status-toggle`, `.status-toggle.is-on`, `.status-divider`, `.status-autosave` | `design.md` L225-L236 |
| `.cursor-label` (texto SVG dentro de `#overlays`) | `design.md` L185 |
| `:focus-visible` em qualquer controle fora do viewport: `outline: 2px solid var(--laser-glow)` | `design.md` L338 |

### 3.3 Regras nao-negociaveis (transcritas em CSS)

```css
/* Viewport >= 88% da altura: garantido implicitamente pelas linhas 28/66/24 = 118 px de chrome. */
/* Toolbar nunca > 40 px de largura — fixado em grid-template-columns. */
/* SVG raiz preenche 100% do host */
#viewport-host > svg { width: 100%; height: 100%; display: block; }

/* Stroke minimo do laser nao colapsa em zoom — usa vector-effect quando suportado */
#entities * { vector-effect: non-scaling-stroke; }

/* prefers-reduced-motion: caret da command line estatico */
@media (prefers-reduced-motion: reduce) {
  .cmd-input::after { animation: none; }
}
```

## 4. Invariantes e tolerancias

- **Viewport >= 88% da altura util.** Soma fixa de chrome: `28 + 66 + 24 = 118 px`. Em janelas de altura `H >= ~983 px`, o viewport cobre >= 88%. Em telas menores que ~983 px, a porcentagem cai, mas as alturas absolutas de chrome **nao** encolhem — esse e o trade-off declarado (`design.md` L115).
- **Toolbar nunca passa de 40 px.** Travado em `grid-template-columns: 40px 1fr` (`design.md` L116).
- **Menubar 28 px, statusbar 24 px, command line 66 px** — alturas fixadas em pixel; nao usar `rem` aqui para evitar surpresas com `font-size` do usuario.
- **Stroke nao escalavel da geometria comitada**: `#entities * { vector-effect: non-scaling-stroke }` mantem o traco com largura visual estavel em qualquer zoom, conforme regra "0.1 mm convertido a px conforme zoom, minimo 1 px" (`design.md` L167).
- **Cor de fundo de cada regiao**: `--bg-canvas` apenas no `#viewport-host`; todas as outras regioes (menubar, toolbar, command line, statusbar) usam `--bg-chrome`. Menus dropdown e dialogs usam `--bg-elevated` (`design.md` L19-L22).
- **Divisorias** entre regioes: linha 1 px `--border-subtle` (sem sombras — `design.md` L22).
- **Foco visivel** em controles HTML fora do viewport: `outline: 2px solid var(--laser-glow)` (acessibilidade — `design.md` L338).
- **Sem animacoes decorativas.** A unica animacao tolerada e o blink do caret da command line, e ela deve respeitar `prefers-reduced-motion: reduce` (`design.md` L340).
- **`overflow: hidden` em `#viewport-host`**: impede que crosshair/labels SVG "vazem" para a chrome quando o cursor sai pela borda.
- **Nada de `position: absolute`** para as 4 regioes — o grid governa tudo. `position: relative` no `#viewport-host` so existe para servir de ancora para tooltips/labels HTML que sobreponham o `<svg>` (Sprint Drawing).

## 5. Exemplos de uso

```html
<!-- index.html, com app.css ja carregado, as regioes ja se organizam -->
<body>
  <nav     id="menubar"></nav>
  <aside   id="toolbar"></aside>
  <main    id="viewport-host"></main>
  <section id="command-line"></section>
  <footer  id="statusbar"></footer>
</body>
```

```js
// Inspecao no DevTools
getComputedStyle(document.body).gridTemplateRows;
// → "28px <auto-resolvido> 66px 24px"

getComputedStyle(document.body).gridTemplateColumns;
// → "40px <restante>px"

getComputedStyle(document.getElementById('toolbar')).width;
// → "40px"

getComputedStyle(document.getElementById('viewport-host')).backgroundColor;
// → resolvido para o hex de --bg-canvas: "rgb(10, 6, 18)"
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html` por duplo-clique; com a janela em 1920×1080, as 5 regioes aparecem nas posicoes do diagrama em `design.md` L80-L102 (menubar fina no topo, toolbar fina na esquerda, viewport ocupando o miolo, command line acima do statusbar).
2. Redimensionar a janela para ~600×400: as alturas das regioes de chrome (28 / 66 / 24) **nao mudam**; a toolbar continua com **exatamente 40 px** de largura (verificavel com `getComputedStyle(document.getElementById('toolbar')).width` retornando `"40px"`).
3. No DevTools, `getComputedStyle(document.getElementById('viewport-host')).backgroundColor` resolve para a cor `--bg-canvas` (RGB `10, 6, 18`); `#menubar` resolve para `--bg-chrome` (RGB `18, 10, 31`).
4. Inspecionar uma divisoria entre regioes: borda 1 px com cor resolvida para `--border-subtle` (`rgb(36, 22, 56)`).
5. `getComputedStyle(document.body).gridTemplateRows` retorna `"28px <valor>px 66px 24px"` (a coluna `auto` do viewport varia com a janela; as outras 3 sao fixas).
6. Dar Tab por elementos da chrome: o controle focado mostra outline 2 px `--laser-glow` (`rgb(157, 77, 255)`).

## 7. Notas de implementacao

- `grid-template-rows: 28px auto 66px 24px` distribui assim: linha 1 = menubar (28 px), linha 2 = viewport / command-line "verticais lado a lado da toolbar" (auto = ocupar o restante = viewport), linha 3 = command-line (66 px), linha 4 = statusbar (24 px). O `grid-template-areas` mostra que o `command-line` ocupa a linha 3 na coluna direita; a coluna esquerda (toolbar) atravessa linhas 2-3 — isso e proposital, mantendo a toolbar continua atras da command line (`design.md` L80-L102).
- Wait, conferindo o ASCII em `design.md` L80-L102: a toolbar atinge ate o nivel da command line, e a statusbar atravessa todas as colunas. O `grid-template-areas` acima reflete isso: `"tools view" / "tools cmd" / "status status"`. A toolbar abrange linhas 2 e 3 da grade.
- `#viewport-host { overflow: hidden }` impede vazamento de elementos do SVG ao passar do `viewBox` visivel; o `<svg>` em si ocupa 100% via `display: block`.
- `vector-effect: non-scaling-stroke` em `#entities *` mantem largura percebida do traco estavel em todos os zooms (regra de minimo 1 px em `design.md` L167) sem precisar recalcular `stroke-width` em JS. Tem boa cobertura em Chromium/Gecko/WebKit; em fallback, o renderizador pode recalcular `stroke-width` por zoom em sprints seguintes.
- Tokens de cor sao **lidos** de `theme.css` por `var(--*)`. Esta folha **nunca** redefine `--*` (escopo limpo).
- Tipografia base do app: `font-family: var(--font-ui); font-size: var(--font-base)` — escala xs/sm/md/lg ficam por classe (`design.md` L65-L73).
- `prefers-reduced-motion: reduce` desliga o blink do caret (acessibilidade, `design.md` L340).
