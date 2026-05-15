# assets/css/reset.css

## 1. Responsabilidade

Zerar defaults inconsistentes do navegador para que o resto do CSS comece em terreno previsivel: box-sizing universal, margens/paddings 0, scroll desabilitado no `<body>`, e fonte sans-serif do sistema como fallback ate `theme.css` aplicar `--font-ui`.

## 2. Dependencias

- carga: **primeira** das tres folhas CSS — antes de `theme.css` e antes de `app.css` (ver `index-html.md` §3.2).
- nenhuma dependencia de tokens (`reset.css` nao usa `var(--*)` — esses sao definidos em `theme.css`).

## 3. API publica

`reset.css` nao expoe "API" no sentido funcional. Lista exata de regras (seletores → propriedades):

```css
*,
*::before,
*::after {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
}

body {
  overflow: hidden; /* viewport ocupa a tela toda; sem scroll */
  font-family:
    system-ui,
    -apple-system,
    'Segoe UI',
    sans-serif; /* fallback ate theme.css */
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

button,
input,
select,
textarea {
  margin: 0;
  padding: 0;
  font: inherit;
  color: inherit;
  background: transparent;
  border: 0;
}

button {
  cursor: pointer;
}

button:focus,
input:focus,
select:focus,
textarea:focus {
  outline: none; /* foco visivel sera definido em app.css */
}

ul,
ol {
  list-style: none;
  margin: 0;
  padding: 0;
}

a {
  color: inherit;
  text-decoration: none;
}

svg {
  display: block; /* elimina baseline gap embaixo de inline SVG */
}

img {
  max-width: 100%;
  display: block;
}
```

## 4. Invariantes e tolerancias

- **Nenhum token `--*` aqui.** Reset e estrutural; cores e fontes definitivas vivem em `theme.css`.
- **`box-sizing: border-box` universal** — incluindo pseudo-elementos `::before`/`::after`.
- **`body { overflow: hidden }`** e nao-negociavel: o viewport ocupa todo o espaco entre as 4 regioes de chrome, e nao pode haver barra de rolagem na pagina (`design.md` L116-L118 reforca regioes fixas).
- **`html, body { height: 100% }`**: necessario para o CSS Grid de `app.css` referenciar `100vh` indiretamente — o body, sendo o grid container, precisa ter altura definida em cascata.
- **Reset de `button`/`input`/`select`/`textarea` para `background: transparent` e `border: 0`**: a chrome do LaserCAD redesenha todos os controles do zero; defaults do browser (gradients, sombras, borda 3D) violariam "densidade alta, ruido baixo" (`design.md` L7).
- **`outline: none` no foco e proposital**, com obrigacao de redefinir em `app.css` (acessibilidade: `--laser-glow` 2 px conforme `design.md` L338).
- **Sem reset de tipografia (h1..h6, p)** — o app nao usa esses elementos em fluxo de documento.
- **Sem reset de `:root` ou `html` para `font-size`** — manter o default do browser (16 px), as escalas xs/sm/base/md/lg em `theme.css` sao em **px absolutos**, nao em `rem` (`design.md` L65-L73).

## 5. Exemplos de uso

```html
<!-- Em index.html, primeira folha carregada -->
<link rel="stylesheet" href="assets/css/reset.css" />
<link rel="stylesheet" href="assets/css/theme.css" />
<link rel="stylesheet" href="assets/css/app.css" />
```

Inspecao no DevTools:

```js
getComputedStyle(document.body).overflow;
// → "hidden"

getComputedStyle(document.body).fontFamily;
// → fonte definida em theme.css (--font-ui resolve para "Inter, system-ui, ..."),
//   ou, se theme.css nao carregar, o fallback "system-ui, ..." de reset.css.

getComputedStyle(document.documentElement).boxSizing;
// → "border-box"
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html` por duplo-clique; no DevTools, `getComputedStyle(document.body).margin` retorna `"0px"` (sem margem default do browser).
2. `getComputedStyle(document.body).overflow` retorna `"hidden"`; tentar rolar a pagina (rodando wheel fora do viewport) nao cria scrollbar.
3. `getComputedStyle(document.documentElement).boxSizing` retorna `"border-box"`; idem para qualquer elemento amostrado (`document.querySelector('#menubar')`).
4. Renomear temporariamente `theme.css` para verificar que o fallback funciona: a UI vira sans-serif do sistema (sem fontes do tema), mas o layout das 4 regioes nao quebra.
5. Inspecionar um `<button>` dentro de qualquer regiao: `background-color` resolvido e `rgba(0, 0, 0, 0)` (transparente — nao o cinza nativo).

## 7. Notas de implementacao

- Reset minimo deliberadamente — nao e Normalize.css/Eric Meyer; cobre apenas o que a aplicacao usa.
- **Stack de fallback de fonte**: `system-ui, -apple-system, "Segoe UI", sans-serif` cobre Linux/macOS/Windows sem dependencia de download. Quando `theme.css` define `--font-ui: Inter, ...`, a regra de `body` em `app.css` (`font-family: var(--font-ui)`) sobrescreve essa cascata.
- **`-webkit-font-smoothing: antialiased`** alinha com o `antialiasing` esperado do crosshair SVG (`design.md` L349-L351).
- **Foco zerado em controles**: substituido por outline 2 px `--laser-glow` em `app.css` para todos os elementos focaveis fora do viewport (`design.md` L338).
- **Sem `*` reset agressivo** (`* { all: unset }` ou similar) — esse padrao quebra herancia de cor e dificulta debug.
- Aceitabilidade em `file://`: este arquivo nao usa `@import`, `@font-face` com URL externa, ou qualquer recurso que exija HTTP — tudo funciona com `file://` (ADR 0001 §3).
