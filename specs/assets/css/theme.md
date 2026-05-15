# assets/css/theme.css

## 1. Responsabilidade

Declarar **todos** os tokens visuais do LaserCAD R14 (paleta 450 nm, tipografia, escalas de tamanho) como CSS custom properties em `:root`, formando a unica fonte de cor/tipografia consumida por `app.css`, pelos componentes da chrome, pelos elementos SVG (via `var(--*)` em `stroke`/`fill`) e por overlays.

## 2. Dependencias

- carga: **segunda** das tres folhas CSS — depois de `reset.css`, antes de `app.css` (ver `index-html.md` §3.2).
- `app.css` consome todos os tokens daqui por `var(--*)`; `theme.css` nao consome ninguem.

## 3. API publica

Todos os tokens vivem em `:root`. Lista **exata** e completa — qualquer adicao precisa estar em `design.md` primeiro.

### 3.1 Paleta (`design.md` L17-L39)

```css
:root {
  /* Fundos */
  --bg-canvas:    #0A0612;   /* Fundo do viewport ("papel" infinito) */
  --bg-chrome:    #120A1F;   /* Toolbar, command line, status bar, menubar */
  --bg-elevated:  #1A1030;   /* Menus dropdown, popovers, dialogs */

  /* Bordas */
  --border-subtle: #241638;  /* Divisorias entre regioes, 1 px */
  --border-strong: #3D2466;  /* Borda de input ativo, foco em dialogs */

  /* Grid */
  --grid-minor:   #1E1232;   /* Pontilhado 1 mm */
  --grid-major:   #2E1A4D;   /* Linhas 10 mm */
  --grid-axis:    #5B2DD1;   /* Eixos X / Y (0,0), 1 px */

  /* Texto */
  --text-primary:   #E8DDFF; /* Off-white com tinta roxa */
  --text-secondary: #8E7CB8; /* Labels, dicas, coordenadas */
  --text-disabled:  #4A3E66; /* Ferramentas indisponiveis */

  /* Laser 450 nm — geometria, ativo, preview */
  --laser-450:    #6E00FF;   /* PRIMARIA: traco de corte, ferramenta ativa */
  --laser-glow:   #9D4DFF;   /* Hover, highlight de selecao */
  --laser-dim:    #3D0099;   /* Estado pressionado, traco atras */

  /* Snaps (formas tambem distintas — daltonismo, design.md L343) */
  --snap-endpoint:     #FFD400;  /* Quadrado vazado */
  --snap-midpoint:     #FF8A00;  /* Triangulo vazado */
  --snap-center:       #00E5FF;  /* Circulo vazado */
  --snap-intersection: #FF2D7A;  /* X */

  /* Status */
  --status-ok:    #3DDC97;   /* Autosave salvo, confirmacoes */
  --status-warn:  #FFB020;   /* Avisos nao-bloqueantes */
  --status-error: #FF4D6D;   /* Erros de comando, validacao */
```

### 3.2 Tipografia (`design.md` L58-L73)

```css
  /* Familias */
  --font-ui:      "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
  --font-mono:    "JetBrains Mono", "Menlo", "Consolas", monospace;
  --font-display: "Inter", system-ui, sans-serif;

  /* Escala (px absolutos — nao rem; design.md L65-L73) */
  --font-xs:   11px;   /* Status bar, coordenadas no cursor */
  --font-sm:   12px;   /* Command line, tooltips, labels */
  --font-base: 13px;   /* Inputs, dialog body */
  --font-md:   14px;   /* Menus dropdown */
  --font-lg:   16px;   /* Titulo de dialog */

  /* Pesos / line-heights fixos */
  --font-weight-regular: 400;
  --font-weight-bold:    600;
  --line-height-text:    1.4;
  --line-height-cmd:     1.0;
}
```

### 3.3 Lista exaustiva (resumo verificavel)

Tokens de cor (21): `--bg-canvas`, `--bg-chrome`, `--bg-elevated`, `--border-subtle`, `--border-strong`, `--grid-minor`, `--grid-major`, `--grid-axis`, `--text-primary`, `--text-secondary`, `--text-disabled`, `--laser-450`, `--laser-glow`, `--laser-dim`, `--snap-endpoint`, `--snap-midpoint`, `--snap-center`, `--snap-intersection`, `--status-ok`, `--status-warn`, `--status-error`.

Tokens de tipografia (12): `--font-ui`, `--font-mono`, `--font-display`, `--font-xs`, `--font-sm`, `--font-base`, `--font-md`, `--font-lg`, `--font-weight-regular`, `--font-weight-bold`, `--line-height-text`, `--line-height-cmd`.

Total: **33 tokens** em `:root`.

## 4. Invariantes e tolerancias

- **Os 21 valores hex sao exatamente os de `design.md` L17-L39.** Nao arredondar, nao re-tonificar, nao substituir por nomes (`rebeccapurple` etc.). Cada token tem 6 digitos hex maiusculos ou minusculos consistentes (formato livre).
- **Nenhum token e calculado** (`color-mix`, `hsl(from ...)`, `light-dark()`). Conserva compatibilidade com `file://` em qualquer browser moderno e com inspecao trivial.
- **Sem temas claros**: o app e dark-only (`design.md` L11). Nao ha `@media (prefers-color-scheme: light)`.
- **Sem `@font-face`** apontando para arquivos externos no MVP — confia na presenca de Inter / JetBrains Mono no sistema do usuario; se ausentes, cai para `system-ui` / `Menlo` / `Consolas`. (Embed de fontes via `@font-face` com `data: URI` e possibilidade futura; nao no MVP.)
- **Escala em px absolutos** (`design.md` L65 "tamanhos fixos, sem escala fluida"): nao usar `rem`/`em`/`clamp()`.
- **Line-height 1.4** para texto narrativo, **1.0** para command line e coordenadas (texto mono compacto) — `design.md` L74.
- **Sem itálico em UI** — convencao escrita em `design.md` L74; nao ha `--font-style` token porque o default `normal` cobre todos os usos.
- **`--laser-450` e a unica cor primaria** da geometria comitada e da chrome ativa; nao criar derivados extras sem ADR.
- **Daltonismo**: as 4 cores de snap (`--snap-*`) sao distintas em hue **e** em forma (□/△/○/×). A forma e responsabilidade do renderer; aqui so o token de cor.

## 5. Exemplos de uso

```css
/* Em app.css */
#viewport-host {
  background: var(--bg-canvas);
}
.tool-button.is-active {
  background: var(--laser-dim);
  color: var(--text-primary);
}
.cmd-prompt {
  font-family: var(--font-mono);
  font-size: var(--font-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}
.status-toggle.is-on {
  color: var(--laser-450);
}
```

```html
<!-- Em geometria SVG (render/entity-renderers gera assim) -->
<line stroke="var(--laser-450)" stroke-width="0.1" fill="none" ... />
<circle stroke="var(--grid-minor)" ... />
```

```js
// Inspecao no DevTools
getComputedStyle(document.documentElement).getPropertyValue('--laser-450').trim();
// → "#6E00FF"

getComputedStyle(document.documentElement).getPropertyValue('--bg-canvas').trim();
// → "#0A0612"

getComputedStyle(document.documentElement).getPropertyValue('--font-mono').trim();
// → "\"JetBrains Mono\", \"Menlo\", \"Consolas\", monospace"
```

## 6. Criterios de aceitacao testaveis manualmente

1. No DevTools, `getComputedStyle(document.documentElement).getPropertyValue('--laser-450').trim()` retorna **exatamente** `"#6E00FF"`.
2. Mesma verificacao para todos os 21 tokens de cor: hex coincide com a tabela em `design.md` L17-L39 — sem desvio em nenhum digito.
3. `getComputedStyle(document.documentElement).getPropertyValue('--font-xs').trim()` retorna `"11px"`; `--font-base` retorna `"13px"`; `--font-lg` retorna `"16px"`.
4. Em uma area do app que usa `--bg-canvas` (o `#viewport-host`), o computed `background-color` resolve para `rgb(10, 6, 18)` (= `#0A0612`).
5. Em um elemento que usa `--font-mono` (comando ativo da command line apos `ui.commandLine` montar), o computed `font-family` lista "JetBrains Mono" em primeiro.
6. Buscar no arquivo `theme.css` por hex que **nao** estejam na lista de `design.md`: zero ocorrencias (todos os hex aqui sao tokens listados).

## 7. Notas de implementacao

- Lista derivada **exatamente** de `design.md` L17-L39 (paleta) e L58-L73 (tipografia). Qualquer cor nova exige atualizar `design.md` antes — esse e o vinculo de governanca.
- A escolha de `--laser-450 = #6E00FF` substitui o vermelho classico do R14; alinhamento de identidade explicado em `design.md` L41 e L353.
- Tokens de snap mantem vocabulario AutoCAD (`design.md` L41, L177-L185) — nao renomear.
- Tipografia: Inter e JetBrains Mono sao fontes de uso livre; mesmo sem instalacao, o fallback (`system-ui`, `Menlo`/`Consolas`) preserva a legibilidade. Em `file://`, evitar `@import url(...)` para Google Fonts (gera CORS em alguns browsers e quebra offline).
- Token `--laser-glow` e usado em foco visivel (`design.md` L338); mantem cor distinta de hover/seleao sem precisar criar `--focus-ring`.
- `--bg-elevated` e o **unico** nivel de elevacao (`design.md` L21) — nao adicionar `--bg-elevated-2`. Profundidade percebida vem de 1 px de `--border-strong`, nao de sombras.
- O arquivo nao define seletores diferentes de `:root`. Estilos visuais por componente vivem em `app.css`.
