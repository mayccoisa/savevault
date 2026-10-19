# Sistema visual do SaveVault

Um conjunto de tokens, em `src/gui/design.rs`. Cores e classes de widget ficam em `src/gui/style.rs`.
A fonte é a Noto Sans embutida (`src/gui/font.rs`), no Windows e no Linux. As variações v4 e v5
foram aposentadas: cada uma repetia a escala e um interruptor em `src/gui.rs` deixava a interface
oscilar entre duas fundações.

Se um número novo aparece num arquivo de tela, o número está no lugar errado. Ele entra aqui, com
nome, e a tela passa a ler o nome.

## Escala

**Tipo** — 12 legenda · 14 corpo · 16 subtítulo · 20 título. O corpo é 14, e é o `default_text_size`
da janela. Ênfase é peso (`TEXT_STRONG`) e cor, não um quinto tamanho.

**Espaço** — 4 · 8 · 12 · 16 · 24 · 32. Tudo múltiplo de 4. O espaço em volta de um grupo é pelo
menos o dobro do espaço dentro dele. Espaço igual dentro e entre grupos faz o rótulo parecer solto
do campo.

**Raio** — 8, um só. Distintivo e chip são pílula (`RADIUS_PILL`), que é uma forma, não um segundo
raio. Card, botão, campo e capa usam o mesmo 8.

**Controle** — altura 32 para botão, campo, lista de opções e item da barra. Uma variante muda a
largura e o tipo, não a altura. O chevron de expandir uma linha é a exceção documentada (`COMPACT`,
24): não é um controle com rótulo.

**Busca** — largura 280. Um campo que ocupa a linha inteira vira uma segunda barra de ferramentas.

**Sidebar** — 232, fixa. **Barra de título** — 56.

## Alinhamento do texto

O `Button` do iced posiciona o filho no topo da área interna. O destaque preenche a altura do
controle; o texto só tem a altura da linha. Sem um invólucro que centralize, o rótulo sobe e o
destaque fica abaixo da palavra. Foi medido na sidebar: o destaque ocupava y 90–125 e a tinta do
rótulo y 93–102.

Todo botão com altura declarada passa o conteúdo por `button::centered`. Rótulo de controle usa
entrelinha justa (`leading::TIGHT`, 1.0). A entrelinha padrão (1.3) reserva uma faixa embaixo da
tinta para uma segunda linha que o botão não tem, e é essa faixa que empurra a letra para cima
quando alguém centraliza a caixa em vez da tinta.

A Noto Sans é a mesma nos dois sistemas. Pedir Segoe UI no Windows, com outro arquivo de métricas,
fazia o mesmo botão parecer centrado num sistema e alto no outro.

Campo e lista de opções têm a mesma altura 32, mas a caixa da linha tem 20 (`control::LINE`). Os
6 px que sobram são padding igual em cima e embaixo. Uma linha da altura do controle desenha o
cursor encostado nas duas bordas.

O menu (⋮) abre para baixo, alinhado à esquerda do botão. Quando não cabe à direita da janela, ele
desliza para a esquerda até o rótulo inteiro aparecer. "Restauração em lote" é o rótulo que define
a largura (240).

## Densidade

Uma tela de ferramenta mostra caminho, data e contagem. Título de 25 px, barra de 96 px, capa de
192×288 e campo de busca na largura da janela competiam entre si e cabiam quatro capas por linha.

A galeria usa capa 128×192 (proporção 2:3). Na janela mínima (1036 px, com a sidebar) cabem cinco
capas; numa janela de 1280 px cabem sete. O título do card tem no máximo duas linhas e o que não
cabe vira reticências. O nome inteiro continua na dica. A página do jogo usa 160×240, a mesma
proporção, porque ali a capa é o assunto e não um item de uma grelha.

O espaço entre capas é 12. O espaço entre seções continua 24.

## Qual botão usar

Há uma ação primária por tela, e ela é a única preenchida com a cor de destaque.

| Papel | Classe | Quando |
|---|---|---|
| Primário | `Primary` | A ação que a tela existe para fazer. Uma. |
| Secundário | `Secondary` | Divide a barra com a primária, ou é a ação de uma linha. Contorno, sem preenchimento. |
| Perigo | `Danger` | Remover uma linha. Quieto em repouso; vermelho só sob o ponteiro. O vermelho sólido fica no diálogo de confirmação. |
| Negativo | `Negative` | Cancelar uma operação que já está correndo. |
| Menu | `PopupMenu` | Ações secundárias que não precisam estar sempre visíveis. |

A biblioteca não tem botão primário na barra: a ação principal é abrir o jogo, e isso é o card.
Atualizar, trocar o cofre e a restauração em lote ficam num único menu (⋮). O nome do cofre
permanece visível, como legenda, porque esconder o lugar dos saves seria economizar o controle
errado.

Galeria e Lista são um par segmentado (`button::segment`), não dois botões soltos. Os dois têm a
altura dos outros controles; só o modo ativo vem preenchido.

## Tema

Claro e escuro compartilham a escala. O que muda é a paleta em `style.rs`. A cor de destaque é a
escolha do usuário e vale nos dois temas. O item ativo da sidebar usa um véu dessa cor, para não
sumir num cinza em cima de outro cinza.
