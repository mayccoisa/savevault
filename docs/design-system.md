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

Uma tela de ferramenta mostra caminho, data e contagem. Título de 25 px, barra de 96 px e capa
vertical de 192×288 competiam entre si.

O card da biblioteca é largo (228 px). Em 1280 px cabem quatro; na janela mínima cabem três. O
título do card tem uma linha e o que não cabe vira reticências. O nome inteiro continua na dica.
O espaço entre cards é 12. O espaço entre seções continua 24.

O nome do jogo na página dele é 28 px (`text::DISPLAY`). É o único tipo acima do título de tela,
porque nessa página o título de tela continua sendo "Biblioteca" e o assunto é o jogo.

## Card da biblioteca

O card abre a página do jogo. Não abre painel nem diálogo.

De cima para baixo:

1. Quadro 16:9. O selo no canto direito é a plataforma (PC ou o nome do emulador).
2. A mesma plataforma em caixa-alta, pequena. Repete o selo de propósito: a arte compete com o
   texto, e a linha continua legível quando a capa é escura.
3. Título, uma linha.
4. Chip com a quantidade de versões.
5. Rodapé: "Último backup" à esquerda, a data à direita.

Não existe arte horizontal separada da capa. A capa Steam é retrato 2:3. O quadro é montado quando
a imagem carrega: o fundo é a mesma capa ampliada, desfocada e escurecida, e a capa nítida fica
centralizada, sem corte. A miniatura da página e a lista continuam com o retrato inteiro.

## Barra da biblioteca

Duas linhas, e a busca não divide a linha com a ordenação.

1. "Biblioteca", um badge com o total de jogos no cofre, e a ordenação à direita.
2. Campo de busca com ícone de lupa, depois os selects de plataforma e de status. Galeria e Lista
   ficam no fim dessa linha.

Plataforma lista o que o cofre realmente tem (PC e cada emulador). Status é "Qualquer", "Backup
recente" (o mais novo tem até 30 dias) ou "Backup antigo". Quando o filtro reduz a lista, uma
legenda diz quantos jogos estão visíveis. O badge continua sendo o total.

## Página do jogo

"Voltar à biblioteca" fica acima do hero. O hero é o mesmo quadro 16:9, escurecido, na largura da
página. Sobre ele: a miniatura do retrato, a plataforma pequena, o título grande, a data, os chips
e as ações. A ação primária é "Restaurar última versão" e ela abre a revisão do destino; não
escreve. "Fazer novo backup" e "Escolher capa" são secundárias.

Abaixo, três abas — Visão geral, Versões, Registros — e uma coluna à direita com o que o cofre
guarda. Versões lista cada cópia com "Revisar restauração". Registros mostra só as anotações.

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
