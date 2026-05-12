# Atalhos de teclado — LaserCAD R14

## Ferramentas

| Tecla | Comando |
|---|---|
| `L` | Line |
| `P` | Polyline |
| `R` | Rectangle |
| `C` | Circle |
| `A` | Arc |
| `S` | Select |
| `M` | Move |
| `T` | Trim |
| `E` | Extend |
| `Del` | Apagar entidades selecionadas |

## Câmera

| Tecla / gesto | Ação |
|---|---|
| Roda do mouse | Zoom com pivot no cursor |
| Botão do meio + arrastar | Pan |
| `Espaço` + arrastar (botão esquerdo) | Pan |
| Comando `zoom extents` | Enquadrar todo o documento |

## Modificadores durante desenho

| Tecla | Comportamento |
|---|---|
| `Shift` (segurar) | Trava ortho (0° / 90°) no segmento ativo |
| `F8` | Toggle ortho permanente |
| `F3` | Toggle snaps |
| `F7` | Toggle grid |

## Edição

| Tecla | Ação |
|---|---|
| `Esc` | Cancela ferramenta ativa, re-arma `select` |
| `Enter` ou `Espaço` (na command line) | Confirma input |
| `↑` / `↓` (na command line) | Histórico de comandos |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+S` | Salvar SVG (preset cut) |
| `Ctrl+N` | Documento novo (via menu File) |

## Entrada na command line

| Forma | Exemplo | Significado |
|---|---|---|
| Comando | `line`, `l`, `circle`, `c` | Aciona ferramenta |
| Toggle | `snap`, `grid`, `ortho` | Alterna toggle |
| Zoom | `zoom in`, `zoom out`, `zoom extents` (ou `z e`) | Zoom |
| Coord absoluta | `124.5, 87.3` | Ponto em mm |
| Coord relativa | `@50, 0` | Deslocamento do último ponto |
| Distância | `50` | Distância na direção do cursor (após primeiro ponto) |
