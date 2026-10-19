# Biblioteca do SaveVault · primeira entrega

## Decisões confirmadas

A revisão de UX abrange o app inteiro, com primeira entrega concentrada na biblioteca.
Somente jogos com backup entram no acervo. Galeria é a visualização inicial, com lista compacta opcional.
Cada card mostra capa, título, último backup e quantidade de versões restauráveis.
O padrão é mais recentes primeiro; nome e mais antigos são alternativas.
Busca encontra título, identidade interna e emulador.
O clique abre detalhes, histórico e ações. A restauração passa pela revisão e confirmação existentes.

## Densidade

A capa da grelha é 128×192. Título com mais de duas linhas é cortado com reticências; o nome
inteiro fica na dica. Busca, ordenação e o par Galeria/Lista ficam na faixa da grelha. Atualizar,
trocar o cofre e a restauração em lote ficam no menu da barra de título. As regras de tamanho e de
qual botão usar estão em [design-system.md](design-system.md).

## Capas

Busca automática pelo ID Steam exato do manifesto, sem correspondência aproximada por título.
Fonte: https://partner.steamgames.com/doc/store/assets/libraryassets
O download usa a capa de biblioteca do CDN Steam e no máximo quatro requisições simultâneas.
Imagens ficam em `library/covers` dentro do diretório de configuração, sem alterar o cofre.
Uma capa escolhida manualmente tem precedência sobre downloads e é copiada para o cache.
PNG e JPEG são aceitos, com limite de 12 MB e dimensões de até 8192 pixels.
Sem rede, usa o cache. Sem correspondência, mantém título legível e permite escolher uma imagem.

## Limites desta entrega

A integração automática cobre os jogos com ID Steam no manifesto. Catálogos de capas de emuladores
e de outras lojas ainda não foram integrados. A escolha manual funciona para qualquer jogo.
As demais telas permanecem para a entrega seguinte, após avaliar a biblioteca.

## Segurança e identidade

Título exibido vem do próprio backup. Ações usam a identidade interna original.
A revisão de uma versão limita a confirmação àquele jogo, nunca ao cofre inteiro.
Nenhuma mudança foi feita no algoritmo de restauração, na resolução de destinos ou no ponto de desfazer.
Preferências de galeria, lista e ordenação ficam em `library/preferences.json`.
