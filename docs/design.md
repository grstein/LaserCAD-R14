# Design do LaserCAD R14

## Princípios de design

O LaserCAD R14 é um micro-CAD 2D no navegador. A interface segue três princípios não-negociáveis, herdados do AutoCAD R14 e adaptados ao contexto moderno de laser cutting:

1. **Área de desenho soberana.** O viewport ocupa praticamente toda a tela. Toolbar, command line e status bar somam menos de 12% da altura útil. Nenhum painel flutuante por padrão; nenhum overlay decorativo.
2. **Densidade alta, ruído baixo.** Ícones monocromáticos, sem sombras, sem gradientes em controles. A cor é reservada para informação operacional (snap ativo, ferramenta selecionada, seleção, preview).
3. **Teclado em primeiro lugar.** Cada ferramenta tem atalho de uma letra. A command line é o caminho canônico; o mouse acelera, mas não substitui. Cursor crosshair de borda a borda, como no R14.

Não há temas claros: o fundo escuro é parte da identidade e da legibilidade do laser. Não há onboarding tutorial cobrindo a tela. Não há animações de transição entre estados de ferramenta — apenas micro-feedback funcional (highlight de snap, dash de preview).

## Paleta laser 450nm

O 450nm é o comprimento de onda dos diodos laser azul-violeta usados em gravadoras a laser. A paleta deriva diretamente desse espectro, aplicada com hierarquia rígida: o roxo só aparece em elementos ativos e na geometria de corte.

| Token                 | Hex       | Função                                                                | Uso                            |
| --------------------- | --------- | --------------------------------------------------------------------- | ------------------------------ |
| `--bg-canvas`         | `#0A0612` | Fundo do viewport                                                     | Cor base do "papel" infinito   |
| `--bg-chrome`         | `#120A1F` | Fundo de toolbar, command line, status bar                            | Levemente acima do canvas      |
| `--bg-elevated`       | `#1A1030` | Menus, popovers, dialogs                                              | Único nível de elevação        |
| `--border-subtle`     | `#241638` | Divisórias entre regiões                                              | Linha 1px, sem sombras         |
| `--border-strong`     | `#3D2466` | Borda de input ativo, foco                                            | Linha 1px                      |
| `--grid-minor`        | `#1E1232` | Grid auxiliar (1mm)                                                   | Pontilhado fino                |
| `--grid-major`        | `#2E1A4D` | Grid principal (10mm)                                                 | Linha contínua                 |
| `--grid-axis`         | `#5B2DD1` | Eixos X/Y (0,0)                                                       | Linha de 1px                   |
| `--text-primary`      | `#E8DDFF` | Texto principal                                                       | Off-white com tinta roxa       |
| `--text-secondary`    | `#8E7CB8` | Labels, dicas, coordenadas                                            | Cinza-violeta                  |
| `--text-disabled`     | `#4A3E66` | Ferramentas indisponíveis                                             |                                |
| `--laser-450`         | `#6E00FF` | **Cor primária: traço de corte, ferramenta ativa, preview commitado** | Equivalente percebido do 450nm |
| `--laser-glow`        | `#9D4DFF` | Hover, highlight de seleção                                           | Versão mais clara              |
| `--laser-dim`         | `#3D0099` | Estado pressionado, traço atrás de outro                              |                                |
| `--snap-endpoint`     | `#FFD400` | Marcador de snap endpoint (□)                                         | Amarelo cromo, alto contraste  |
| `--snap-midpoint`     | `#FF8A00` | Marcador de snap midpoint (△)                                         | Laranja                        |
| `--snap-center`       | `#00E5FF` | Marcador de snap center (○)                                           | Ciano                          |
| `--snap-intersection` | `#FF2D7A` | Marcador de snap intersection (×)                                     | Magenta                        |
| `--status-ok`         | `#3DDC97` | Confirmações, autosave salvo                                          | Verde discreto                 |
| `--status-warn`       | `#FFB020` | Avisos não-bloqueantes                                                | Âmbar                          |
| `--status-error`      | `#FF4D6D` | Erros de comando, validação                                           | Rosa-vermelho                  |

**Decisão deliberada:** o vermelho clássico do R14 é substituído por `--laser-450` (roxo 450nm) como cor da geometria. Mantém-se a referência cultural de "linha colorida em fundo preto", mas alinhada ao domínio do produto. As cores de snap (amarelo/laranja/ciano/magenta) mantêm o vocabulário de cores distintas do AutoCAD para evitar confusão entre tipos de snap.

```text
┌─────────────────────────────────────────────────────────────────┐
│  #0A0612   #120A1F   #1A1030   ── chrome levels                 │
│                                                                 │
│      #6E00FF  ──  laser 450nm (geometria, ativo)                │
│      #9D4DFF  ──  hover/seleção                                 │
│      #5B2DD1  ──  eixos X/Y                                     │
│                                                                 │
│   ▣ #FFD400   △ #FF8A00   ○ #00E5FF   × #FF2D7A                 │
│   endpoint    midpoint    center      intersection              │
└─────────────────────────────────────────────────────────────────┘
```

## Tipografia

| Token            | Família                              | Uso                                              |
| ---------------- | ------------------------------------ | ------------------------------------------------ |
| `--font-ui`      | `Inter`, `system-ui`, sans-serif     | Toolbar tooltips, menus, dialogs                 |
| `--font-mono`    | `JetBrains Mono`, `Menlo`, monospace | Command line, coordenadas, dimensões, status bar |
| `--font-display` | `Inter`, sans-serif, peso 600        | Apenas título de dialogs                         |

Tamanhos fixos, sem escala fluida:

| Escala |  px | Uso                               |
| ------ | --: | --------------------------------- |
| `xs`   |  11 | Status bar, coordenadas no cursor |
| `sm`   |  12 | Command line, tooltips, labels    |
| `base` |  13 | Inputs, dialog body               |
| `md`   |  14 | Menus dropdown                    |
| `lg`   |  16 | Título de dialog                  |

Line-height fixo em 1.4 para texto e 1.0 para command line / coordenadas. Sem itálico em UI. Bold apenas em prompts ativos da command line ("`Specify next point:`").

## Layout geral

A tela é dividida em quatro regiões fixas. Sem reordenação por drag, sem painéis dockáveis no MVP.

```text
┌─────────────────────────────────────────────────────────────────┐
│ FILE  EDIT  VIEW  DRAW  MODIFY  HELP             ▢ LaserCAD R14 │ ← 28px menubar
├──┬──────────────────────────────────────────────────────────────┤
│  │                                                              │
│ L│                                                              │
│ I│                                                              │
│ N│                                                              │
│ E│                  ╋  (crosshair infinito)                     │
│  │                                                              │ ← Viewport
│ R│                                                              │
│ E│                                                              │
│ C│                                                              │
│  │                                                              │
│ C│                                                              │
│  │                                                              │
├──┴──────────────────────────────────────────────────────────────┤
│ Command: line                                                   │ ← 22px × 3 linhas
│ LINE  Specify first point: _                                    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ 124.500, 87.300  mm  │ SNAP │ GRID │ ORTHO │       ● autosaved  │ ← 24px statusbar
└─────────────────────────────────────────────────────────────────┘
   40px
```

| Região           | Altura/Largura  | Conteúdo                                                                                                    |
| ---------------- | --------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------- |
| Menubar          | 28px            | File / Edit / View / Draw / Modify / Help. Texto, sem ícones.                                               |
| Toolbar vertical | 40px            | Ícones das ferramentas (line, polyline, rect, circle, arc, select, trim, extend, move, delete). Uma coluna. |
| Viewport         | restante        | SVG nativo, cobre todo o espaço entre toolbar e command line.                                               |
| Command line     | 66px (3 linhas) | Histórico de 2 linhas + linha ativa de input.                                                               |
| Status bar       | 24px            | Coordenadas mm                                                                                              | toggles (SNAP/GRID/ORTHO) | indicador de autosave. |

**Regras de layout não-negociáveis:**

- Viewport sempre ocupa ≥ 88% da altura. Toolbar nunca passa de 40px de largura.
- Menubar e toolbar não têm ícones decorativos do app — sem logo, sem branding na chrome.
- Status bar fixa no rodapé, sempre visível. Coordenadas atualizam a cada movimento do cursor.
- Sem rulers laterais no MVP. O grid + status bar cobrem a função.

## Toolbar vertical

Inspirada diretamente no R14: coluna estreita de ícones monocromáticos, sem rótulos visíveis. Cada ícone é 24×24px dentro de um botão 40×32px. Tooltip aparece após 400ms de hover com o nome da ferramenta e atalho (`Line (L)`).

| Estado                         | Visual                                                                                                                 |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Inativo                        | Ícone `--text-secondary` (#8E7CB8) sobre `--bg-chrome`                                                                 |
| Hover                          | Ícone `--laser-glow` (#9D4DFF), fundo `--bg-elevated`                                                                  |
| Ativo (ferramenta selecionada) | Ícone `--text-primary` (#E8DDFF), fundo `--laser-dim` (#3D0099), barra vertical de 2px `--laser-450` na borda esquerda |
| Desabilitado                   | Ícone `--text-disabled`, sem hover                                                                                     |

```text
┌──────┐
│ ─    │ Line       (L)
├──────┤
│ ◢    │ Polyline   (P)
├──────┤
│ ▢    │ Rectangle  (R)
├──────┤
│ ○    │ Circle     (C)
├──────┤
│ ⌒    │ Arc        (A)
├──────┤
│ ▭    │ Select     (S)   ← separador antes de edit
├──────┤
│ ⊣    │ Trim       (T)
├──────┤
│ ⊢    │ Extend     (E)
├──────┤
│ ✥    │ Move       (M)
├──────┤
│ ✕    │ Delete   (Del)
└──────┘
```

Ícones desenhados como SVG inline, traço 1.5px, sem fill, cantos retos. Estilo único em todo o app — sem mistura de ícones outline e filled.

## Viewport

O viewport é um `<svg>` que cobre todo o espaço disponível, com `viewBox` em mm. O cursor padrão dentro do viewport é um **crosshair de borda a borda** (linha horizontal + vertical de 1px atravessando toda a área), em `--text-secondary` com 60% de opacidade. Quando uma ferramenta está armada, o crosshair vira `--laser-glow`.

| Elemento do viewport                    | Estilo                                                                              |
| --------------------------------------- | ----------------------------------------------------------------------------------- |
| Fundo                                   | `--bg-canvas` (#0A0612), sólido                                                     |
| Grid minor (1mm)                        | Pontos `--grid-minor` (#1E1232), 1px                                                |
| Grid major (10mm)                       | Linhas `--grid-major` (#2E1A4D), 0.5px                                              |
| Eixos X/Y                               | Linha `--grid-axis` (#5B2DD1), 1px contínua                                         |
| Geometria comitada                      | Traço `--laser-450` (#6E00FF), 0.1mm convertido a px conforme zoom, mínimo 1px      |
| Geometria em preview                    | Traço `--laser-glow` (#9D4DFF), `stroke-dasharray="4 2"`                            |
| Geometria selecionada                   | Traço `--laser-glow` + pontos de controle quadrados 6px `--laser-450` nos endpoints |
| Geometria sob hover (pré-seleção)       | Traço espesso `--laser-glow`, sem dash                                              |
| Box de seleção (drag → direita)         | Borda `--laser-450` contínua, fill `--laser-450` a 8%                               |
| Box de seleção (drag → esquerda, cross) | Borda `--laser-450` tracejada `4 2`, fill `--laser-450` a 8%                        |

**Marcadores de snap** aparecem flutuando sobre o cursor quando próximo a um candidato. Cada tipo tem forma e cor distintas, sempre 10px:

| Tipo         | Forma            | Cor                             |
| ------------ | ---------------- | ------------------------------- |
| Endpoint     | Quadrado vazado  | `--snap-endpoint` (#FFD400)     |
| Midpoint     | Triângulo vazado | `--snap-midpoint` (#FF8A00)     |
| Center       | Círculo vazado   | `--snap-center` (#00E5FF)       |
| Intersection | ×                | `--snap-intersection` (#FF2D7A) |

Ao lado do marcador, tooltip mono 11px com o nome (`endpoint`, `midpoint`, `center`, `intersection`), em `--text-primary` com fundo `--bg-elevated` translúcido a 90%.

**Coordenadas dinâmicas:** quando uma ferramenta está em estado `preview`, exibir um label flutuante a 16px do cursor mostrando dimensão atual em mono 11px: `42.500 mm` para linha, `35.0 × 22.5 mm` para retângulo, `R 18.000 mm` para círculo. Cor `--text-primary` sobre fundo `--bg-elevated` a 85%, sem borda.

## Command line

O coração da experiência R14. Sempre visível, sempre focável. Três linhas de altura, tipografia mono.

```text
─────────────────────────────────────────────────────────────────
 Command: line
 LINE  Specify first point: 10,20
 LINE  Specify next point or [Undo]: _
─────────────────────────────────────────────────────────────────
```

| Linha    | Conteúdo                             |
| -------- | ------------------------------------ |
| 1 (topo) | Comando anterior ou último resultado |
| 2 (meio) | Prompt da ferramenta ativa           |
| 3 (base) | Input ativo com cursor piscante `_`  |

**Comportamento:**

- Qualquer tecla alfanumérica fora de um input dá foco automaticamente à command line (replicando o R14).
- `Enter` confirma o comando ou repete o último.
- `Espaço` também confirma (compatibilidade R14).
- `Esc` cancela ferramenta ativa, esvazia input, retorna a `idle`.
- Histórico navegável com `↑`/`↓` quando o input está vazio.
- Prompts ativos em peso 600, `--text-primary`. Histórico em peso 400, `--text-secondary`.
- Erros em `--status-error` (#FF4D6D) com prefixo `! `: `! Invalid point: '10x20'`.

A entrada aceita:

- Coordenadas absolutas: `124.5,87.3`
- Coordenadas relativas: `@50,0`
- Distância direta (após primeiro ponto + Shift para ortho): `50` + Enter
- Comando: `line`, `l`, `rect`, `r`, etc.

## Status bar

Rodapé fino, somente leitura exceto pelos toggles. Tipografia mono 11px.

```text
 124.500, 87.300  mm  │ ◉ SNAP │ ◉ GRID │ ○ ORTHO │       ● autosaved 2s ago
```

| Slot        | Conteúdo                                           | Estado                                                            |
| ----------- | -------------------------------------------------- | ----------------------------------------------------------------- |
| Coordenadas | `X.XXX, Y.YYY  mm`, atualiza a cada movimento      | `--text-primary`                                                  |
| SNAP        | Toggle (F3)                                        | `◉` ligado em `--laser-450` / `○` desligado em `--text-secondary` |
| GRID        | Toggle (F7)                                        | idem                                                              |
| ORTHO       | Toggle (F8)                                        | idem                                                              |
| Autosave    | `● saved Xs ago` ou `● saving…` ou `! save failed` | `--status-ok` / `--text-secondary` / `--status-error`             |

Divisores `│` em `--border-subtle`. Hover nos toggles muda a cor para `--laser-glow`. Click alterna o estado e reflete no atalho de teclado correspondente.

## Menubar

Texto puro, sem ícones. Itens 13px Inter, padding horizontal 14px, altura 28px. Dropdown abre ao click (não hover), fecha no `Esc` ou click fora.

| Menu       | Itens                                                                                                                                           |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **File**   | New `(Ctrl+N)` · Open… `(Ctrl+O)` · Save SVG… `(Ctrl+S)` · Recent files › · — · Exit                                                            |
| **Edit**   | Undo `(Ctrl+Z)` · Redo `(Ctrl+Y)` · — · Cut `(Ctrl+X)` · Copy `(Ctrl+C)` · Paste `(Ctrl+V)` · — · Delete `(Del)` · Select all `(Ctrl+A)`        |
| **View**   | Zoom in `(+)` · Zoom out `(−)` · Zoom extents `(Z E)` · Zoom window `(Z W)` · — · Toggle grid `(F7)` · Toggle snap `(F3)` · Toggle ortho `(F8)` |
| **Draw**   | Line `(L)` · Polyline `(P)` · Rectangle `(R)` · Circle `(C)` · Arc `(A)`                                                                        |
| **Modify** | Select `(S)` · Move `(M)` · Trim `(T)` · Extend `(E)` · Delete `(Del)`                                                                          |
| **Help**   | Keyboard shortcuts `(F1)` · About LaserCAD R14                                                                                                  |

Dropdowns têm largura mínima 220px, fundo `--bg-elevated`, borda 1px `--border-subtle`, sem sombra. Item ativo highlight `--laser-dim` com texto `--text-primary`. Atalhos alinhados à direita em `--text-secondary` mono 11px.

## Dialogs

Dialogs são raros e centrais. Sem modal escurecendo o fundo. Janela 360–480px de largura, fundo `--bg-elevated`, borda 1px `--border-strong`, header 36px com título 16px Inter 600.

```text
┌─────────────────────────────────────────────────────┐
│ Export SVG                                       ✕  │
├─────────────────────────────────────────────────────┤
│                                                     │
│   Filename       drawing-001.svg                    │
│                                                     │
│   Preset         ◉ Cut    ○ Mark    ○ Engrave       │
│                                                     │
│   Stroke width   0.1 mm                             │
│                                                     │
│   ☑ Flatten transforms                              │
│   ☑ Force fill="none"                               │
│   ☐ Include grid as comment                         │
│                                                     │
├─────────────────────────────────────────────────────┤
│                              [ Cancel ]  [ Export ] │
└─────────────────────────────────────────────────────┘
```

| Controle         | Estilo                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- |
| Input texto      | Altura 28px, padding horizontal 10px, borda 1px `--border-subtle`, fundo `--bg-chrome`, foco com borda `--laser-450` |
| Radio / checkbox | 14×14px, sem animação, marcador `--laser-450`                                                                        |
| Botão primário   | `--laser-450` (fundo) + texto `#FFFFFF`, padding 8×16px, sem border-radius extremo (4px)                             |
| Botão secundário | Transparente, borda 1px `--border-strong`, texto `--text-primary`                                                    |
| Botão hover      | Primário vira `--laser-glow`; secundário ganha borda `--laser-450`                                                   |

## Cursor e feedback

| Estado                                   | Cursor                                                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Sobre viewport, sem ferramenta           | Crosshair full-bleed `--text-secondary` 60% + ponto central 4px                                                         |
| Sobre viewport, ferramenta armada        | Crosshair full-bleed `--laser-glow` + tooltip flutuante 11px com nome da ferramenta no canto inferior direito do cursor |
| Sobre entidade hover (modo select)       | Crosshair + entidade highlight em `--laser-glow`                                                                        |
| Snap candidato detectado                 | Crosshair "trava" visualmente no ponto: marcador colorido aparece e o crosshair se desloca para o snap exato            |
| Pan ativo (botão do meio ou Espaço+drag) | Cursor "grab" do sistema                                                                                                |
| Zoom box                                 | Cursor lupa do sistema                                                                                                  |
| Sobre chrome (toolbar/menu)              | Cursor default do sistema                                                                                               |

Sem cursor customizado fora do viewport — uso de cursores nativos do SO para reduzir uncanny valley.

## Iconografia

Conjunto único, desenhado como SVG inline. Especificações:

- **Tamanho:** 24×24px viewBox, traço visível em 18×18 efetivos.
- **Stroke:** 1.5px, `stroke-linecap="round"`, `stroke-linejoin="round"`.
- **Fill:** `none` sempre. Sem ícones preenchidos.
- **Estilo:** geométrico, sem perspectiva, sem detalhes decorativos. Cada ícone deve sugerir o resultado, não a ação (ex: ícone de Line é uma linha diagonal, não uma mão desenhando).

| Ferramenta | Glifo conceitual                                                   |
| ---------- | ------------------------------------------------------------------ |
| Line       | Linha diagonal `\` de canto a canto, com endpoints em quadrado 3px |
| Polyline   | Três linhas conectadas em zigzag                                   |
| Rectangle  | Quadrado vazado                                                    |
| Circle     | Círculo vazado + ponto central                                     |
| Arc        | Arco de 180° com endpoints marcados                                |
| Select     | Cursor seta diagonal                                               |
| Trim       | Tesoura estilizada (duas linhas cruzando) com indicador de corte   |
| Extend     | Linha curta + seta indicando prolongamento                         |
| Move       | Cruz com setas nas 4 pontas                                        |
| Delete     | × diagonal                                                         |

## Estados de ferramenta e feedback visual

Cada ferramenta tem 5 estados (espelhando a máquina de estados do plano: `idle`, `armed`, `preview`, `commit`, `cancel`). O visual de cada estado:

| Estado    | Toolbar                       | Command line                                                               | Cursor                             | Viewport                                                    |
| --------- | ----------------------------- | -------------------------------------------------------------------------- | ---------------------------------- | ----------------------------------------------------------- |
| `idle`    | nenhum botão ativo            | `Command: _`                                                               | crosshair cinza                    | grid normal                                                 |
| `armed`   | botão da ferramenta highlight | `LINE  Specify first point:`                                               | crosshair roxo + tooltip           | grid normal                                                 |
| `preview` | idem                          | `LINE  Specify next point or [Undo]:`                                      | crosshair roxo + label de dimensão | linha tracejada `--laser-glow` do último ponto até o cursor |
| `commit`  | idem                          | `LINE  Specify next point or [Undo]:` (volta a `armed` para próximo ponto) | idem                               | nova entidade aparece em `--laser-450` sólido               |
| `cancel`  | botão desativa                | `*Cancel*` por 1 frame, retorna a `Command: _`                             | crosshair cinza                    | preview desaparece                                          |

## Acessibilidade

| Requisito          | Aplicação                                                                                                                                         |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contraste          | Texto principal sobre fundo: `#E8DDFF` em `#0A0612` = 13.8:1 (AAA). Texto secundário 5.9:1 (AA).                                                  |
| Foco visível       | Outline 2px `--laser-glow` em qualquer elemento focável fora do viewport. Dentro do viewport o foco é implícito via cursor crosshair.             |
| Teclado            | Toda função alcançável por teclado. `Tab` percorre toolbar → command line → status bar toggles → menubar.                                         |
| Movimento reduzido | Sem animações decorativas; o único movimento é o piscar do cursor da command line, respeitando `prefers-reduced-motion: reduce` (caret estático). |
| Tamanho mínimo     | Toolbar e toggles têm hit area mínima de 32×32px.                                                                                                 |

Daltonismo: as 4 cores de snap (amarelo/laranja/ciano/magenta) são também distinguíveis por forma (□/△/○/×). Nunca depender só de cor para diferenciar tipo de snap.

## Inspirações e referências culturais

| Elemento                       | Origem      | O que mantemos                  | O que modernizamos                                              |
| ------------------------------ | ----------- | ------------------------------- | --------------------------------------------------------------- |
| Crosshair full-bleed           | AutoCAD R14 | Sim, característica central     | Antialiasing nativo do SVG, opacidade ajustada                  |
| Command line obrigatória       | AutoCAD R14 | Sim, prompts e fluxo idênticos  | Tipografia JetBrains Mono no lugar de bitmap font               |
| Toolbar vertical estreita      | AutoCAD R14 | Sim, uma coluna                 | Ícones SVG redesenhados, hit areas maiores                      |
| Status bar com toggles         | AutoCAD R14 | Sim, SNAP/GRID/ORTHO + F3/F7/F8 | Indicador de autosave moderno                                   |
| Fundo preto + linhas coloridas | AutoCAD R14 | Fundo escuro mantido            | Roxo 450nm no lugar de vermelho — coerência com o domínio laser |
| Menus drop-down de texto       | AutoCAD R14 | Sim, sem ícones nos menus       | Tipografia limpa, atalhos alinhados à direita                   |
| Snap markers coloridos         | AutoCAD R14 | Sim, paleta similar             | Antialiasing, tooltip com nome                                  |

O que **não** trazemos do R14: ribbon, paleta de propriedades sempre aberta, model/paper space, gradientes 3D nos botões, sons do sistema, splash screen, model viewport tabs.

## Resumo visual em uma frase

Fundo `#0A0612`, geometria em `#6E00FF`, crosshair atravessando a tela, toolbar de uma coluna, command line viva no rodapé, e nada mais — a leveza do R14 com a luz de um diodo laser de 450nm.
