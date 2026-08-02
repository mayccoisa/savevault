# SaveVault

Backup contínuo e **restauração inteligente** de saves de jogos: PC (Steam, GOG, Epic, Heroic,
Lutris, avulsos) **e emuladores** como cidadão de primeira classe (PPSSPP, DuckStation, PCSX2,
RPCS3, shadPS4, Sudachi, Eden, Xenia).

Nome provisório. O diferencial não é o backup, é a **restauração**: você aponta a pasta do emulador
e o app descobre sozinho onde cada save tem que voltar.

## Base: fork do Ludusavi

Este repositório é um fork do [Ludusavi](https://github.com/mtkennerly/ludusavi) de
Michael Kennerly, sob licença MIT (ver [LICENSE](./LICENSE)). Toda a camada de jogo de PC vem de lá
pronta: o catálogo de mais de 19.000 jogos do [Ludusavi Manifest](https://github.com/mtkennerly/ludusavi-manifest)
alimentado pelo PCGamingWiki, detecção por loja, saves no registro do Windows, backup versionado,
retenção, nuvem via rclone, interface gráfica e linha de comando.

O que o SaveVault acrescenta é a camada de emulador e o motor de restauração que resolve o destino
sozinho. Nada disso substitui o que já existe: entra ao lado.

O `upstream` está configurado, então trazer melhoria de lá é rotina:

```bash
git fetch upstream && git merge upstream/master
```

## Estado

Rascunho de produto. Nenhuma linha da camada nova escrita ainda. O PRD vive no Product Hub
(produto **SaveVault**) e a cópia local está em [`docs/prd.md`](docs/prd.md).

## Stack

Rust, herdada do Ludusavi: interface em [iced](https://iced.rs), CLI em clap. Windows na V1.

## Documentação herdada

A ajuda do Ludusavi continua válida e vive em [`docs/help/`](docs/help). Os pontos mais úteis para
entender onde a camada nova encaixa:

* [Roots](docs/help/roots.md), que é o conceito onde o emulador vai virar um tipo de raiz
* [Custom games](docs/help/custom-games.md), o jeito manual que o SaveVault quer tornar desnecessário
* [Redirects](docs/help/redirects.md), o mapeamento na mão que o motor de restauração substitui
* [Backup structure](docs/help/backup-structure.md), onde entra o metadado de significado do save
* [Command line](docs/help/command-line.md)

## Desenvolvimento

Ver [CONTRIBUTING.md](./CONTRIBUTING.md), herdado do upstream.
