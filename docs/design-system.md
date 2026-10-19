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

O menu e a dica têm regra própria, mais abaixo. O que vale aqui é só o alinhamento: o menu abre
para baixo do botão, e a dica fica afastada do controle que ela nomeia.

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

O nome da tela aparece uma vez, na barra de título. O conteúdo não repete "Biblioteca".

Na mesma linha do título: o badge com o total de jogos, e à direita a ordenação, o nome do cofre e
o menu (⋮). A ordenação fica nessa linha de propósito: o menu abre para baixo e não pode cobri-la.

Abaixo, a busca com ícone de lupa, os selects de plataforma e de status, e no fim Galeria e Lista.
A busca não divide a linha com a ordenação.

Plataforma lista o que o cofre realmente tem (PC e cada emulador). Status é "Qualquer", "Backup
recente" (o mais novo tem até 30 dias) ou "Backup antigo". Quando o filtro reduz a lista, uma
legenda diz quantos jogos estão visíveis. O badge continua sendo o total.

As outras telas seguem a mesma regra. Backup, Jogos personalizados, Emuladores, Registros e
Configurações não repetem o nome da tela no corpo. Emuladores mostra o resumo "N de M encontrados"
como título de seção. Configurações troca de seção por uma lista à esquerda (Geral, Idioma, Cofre
e backups, Atualizações, Avançado); o nome da seção dentro do cartão é o grupo, não uma segunda
cópia de "Configurações". O cofre vazio tem a frase "Seu progresso começa aqui", que é o estado
vazio, não outro "Biblioteca".

## Página do jogo

"Voltar à biblioteca" fica acima do hero. O hero é o mesmo quadro 16:9, escurecido, na largura da
página. Sobre ele: a miniatura do retrato, a plataforma pequena, o título grande, a data, os chips
e as ações. A ação primária é "Restaurar última versão" e ela abre a revisão do destino; não
escreve. "Fazer novo backup" e "Escolher capa" são secundárias.

Abaixo, três abas — Visão geral, Versões, Registros — e uma coluna à direita com o que o cofre
guarda. Versões lista cada cópia com "Revisar restauração". Registros mostra só as anotações.

## Dica

Toda dica passa por `widget::hint`. O texto é legenda (12), em uma frase curta, no idioma da
interface. Ela explica o controle quando o rótulo não cabe (ícone, título cortado, selo) e não
repete a palavra que já está no botão.

- Atraso de 400 ms. Dica imediata pisca enquanto o ponteiro só atravessa o controle.
- 8 px de distância do controle e 8 px de respiro interno. Distância zero faz a dica cobrir o
  elemento.
- Abaixo do controle na barra de título. Acima, quando o controle está no meio da lista ou do card,
  para não cair em cima da linha seguinte.
- A superfície é a cor de navegação, com fio de 1 px e sombra de sobreposição. O texto é o texto
  do tema, não um cinza em cima de um campo da mesma família.
- Se não couber na janela, ela entra na área visível, sem cobrir o controle quando ainda há espaço
  do lado combinado.

O menu (⋮) não leva dica. A dica e o menu aberto disputariam o mesmo lugar.

## Menu

O menu (⋮) é o lugar das ações secundárias que não precisam estar sempre visíveis. Largura 240.
Cada linha tem 36 px, ícone de 16 e rótulo, com o mesmo recuo. "Restauração em lote" é o rótulo
que define a largura.

Abre para baixo do botão, com 4 px de folga. Se não cabe à direita da janela, desliza para a
esquerda até o painel inteiro aparecer. Se não cabe embaixo, abre para cima.

O item sob o ponteiro usa um véu da cor de destaque (o mesmo tratamento do item ativo da sidebar),
com o texto normal por cima. Preencher a linha com a cor de destaque sólida faz o menu gritar mais
alto que a tela. O painel tem superfície de campo, fio e sombra de sobreposição.

Atualizar, trocar o cofre e a restauração em lote são as três linhas da biblioteca, cada uma com
ícone. O nome do cofre permanece visível, como legenda, ao lado do menu.

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

No escuro, a página não é preto puro. Três degraus:

| Papel | Cor | Onde |
|---|---|---|
| Página | `#12141c` | O fundo do conteúdo |
| Painel | `#1a1e28` | Sidebar, barra de título, card |
| Campo | `#222733` | Campo de texto, item sob o ponteiro |

Texto `#eef0f6`, texto secundário `#a4aaba`. Entre a sidebar e a página, e debaixo da barra de
título, vai um fio de 1 px a 10% do texto (`alpha::SUBTLE`). A borda do iced pinta os quatro
lados, então esse fio é uma faixa, não uma borda de container.

Um card tem superfície **e** um fio dessa mesma força. Um controle (botão secundário, campo,
lista) usa um fio mais forte (`alpha::HALF` ou mais). Os dois no mesmo peso fazem o botão parecer
outro card.

Pressionar muda o preenchimento: o primário fica mais transparente, o secundário ganha a cor de
campo, o item da sidebar ganha um véu mais forte. O iced 0.14 não tem estado de foco no botão, e
não anima a troca de tela. Tab já percorre os controles; um anel de foco e uma transição de página
ficam para uma decisão do dono.

## Configurações

A tela é uma lista de seções (`design::settings::NAV_WIDTH`, 200) e um formulário que para de
crescer em 720. Cada linha simples é o nome, uma frase curta, e o controle alinhado à direita.
Varredura, manifesto, nuvem, raízes, exclusões e redirecionamentos ficam em Avançado, cada um no
próprio cartão. O caminho do cofre aparece em Cofre e backups; trocar o caminho continua no menu
da biblioteca.

O idioma não é uma lista de texto. Cada linha é a bandeira em SVG (24×18, a proporção 4:3 dos
arquivos em `assets/flags`) e o nome nativo. Idioma incompleto mostra a porcentagem; idioma em
100% não. Emoji de bandeira não entra: o Windows não desenha. As artes são do flag-icons (MIT),
menos o Esperanto, desenhado no repositório porque aquele conjunto não traz a bandeira. O SVG não
é pintado pela cor do tema.

## Estado vazio

Tela sem conteúdo tem um título e uma frase, não uma área em branco e não uma legenda solta.
Biblioteca, Backup, Jogos personalizados e Registros seguem isso. A ação principal continua na
barra de título, para não haver dois botões primários.
