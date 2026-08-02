# SaveVault

> Nome provisório. Backup contínuo e restauração inteligente de saves de jogos de PC e de emuladores.
> Fork do [Ludusavi](https://github.com/mtkennerly/ludusavi) (MIT), em Rust. Windows.

## 1. Problema

Quem joga em emulador perde save. Não é hipótese, é a rotina do hobby: o save vive numa pasta
interna do emulador (`memcards`, `savedata`, `nand`, `content/0000.../00000001`), o usuário atualiza
o emulador, troca de build, formata a máquina, migra do PC para o handheld, e o progresso vai junto.
Quando ele tenta restaurar um backup que fez, tropeça na segunda metade do problema: descobrir para
**qual** pasta cada arquivo tem que voltar, sabendo que o caminho muda por emulador, por versão, por
tipo de instalação (instalada, portátil, Flatpak) e por sistema operacional.

O mercado atual resolve bem o backup e mal a restauração:

| Ferramenta | Cobre PC | Cobre emulador | Vigia mudanças | Restauração sem configurar caminho | Grátis |
|---|---|---|---|---|---|
| Ludusavi | Sim, base de +19.000 jogos | Só via "custom games" configurado à mão | Por agendamento | Não, exige "redirect" manual quando o caminho muda | Sim, MIT |
| SaveState | Parcial | Sim, detecta vários emuladores | Não | Não documentado | Sim, GPLv3 |
| EmuSync | Sim | Sim | Sincronização automática | Não, sync espelha caminho | Sim |
| SaveSync (Steam Deck) | Sim | Sim, caminhos padrão e Flatpak | Sim | Não, presume caminho padrão | Sim |
| Syncthing e afins | Genérico | Genérico | Sim | Não, é espelho de pasta | Sim |

Ou seja: **existe** ferramenta grátis de backup de save, inclusive com emulador. O que não existe é
restauração que **descobre o destino sozinha**. Todas as opções acima presumem que o caminho de
destino é igual ao de origem, ou pedem que o usuário mapeie o caminho novo na mão.

## 2. Proposta

Um **fork do Ludusavi** que herda tudo o que ele já faz de backup de jogo de PC (Steam, GOG, Epic,
Heroic, Lutris, jogo avulso, saves no registro do Windows, versionamento, retenção, nuvem) e
acrescenta duas coisas: **emulador como cidadão de primeira classe** e um **motor de restauração que
resolve o destino sozinho**.

A decisão de forkar, e não construir do zero nem orquestrar por fora, é o que mantém o esforço
concentrado na única parte que ninguém resolveu. O Ludusavi é MIT, então isso é legítimo, e o
histórico dele já está neste repositório com o `upstream` configurado.

O contrato com o usuário na restauração é este:

> Aponte a pasta do emulador. O resto é problema meu.

E, no limite, nem isso: se o emulador estiver instalado num caminho conhecido, o app já acha.

## 3. Persona

| | |
|---|---|
| **Quem** | Jogador de emulador em PC ou handheld (Steam Deck, ROG Ally), joga em mais de um lugar, tem biblioteca grande e mistura jogo de PC com ROM |
| **Nível técnico** | Sabe instalar emulador e mexer em pasta. Não quer escrever YAML, script nem mapear caminho |
| **Gatilho de dor** | Perdeu save. Ou está prestes a fazer algo que já custou save antes: formatar, trocar de build do emulador, migrar de máquina |
| **O que ele faz hoje** | Copia pasta na mão para o Drive, ou usa Syncthing, ou não faz nada e reza |

Persona secundária a validar com o Maycon: o jogador de handheld que quer levar o save do PC para o
portátil e voltar, cenário em que o caminho **nunca** é o mesmo.

## 4. Jobs to be done

1. Quando eu paro de jogar, quero que o progresso esteja salvo em outro lugar sem eu lembrar de nada,
   para nunca perder mais de uma sessão.
2. Quando eu troco de máquina ou formato, quero recuperar o progresso apontando o mínimo possível,
   para voltar a jogar hoje e não no fim de semana.
3. Quando eu atualizo ou troco de emulador, quero que os saves continuem sendo achados,
   para a atualização não ser um risco.
4. Quando eu tenho dois lugares com progresso diferente, quero saber **qual é o mais novo** antes de
   sobrescrever, para não matar a versão certa.

O JTBD 2 é o diferencial. Os outros três são paridade competitiva.

## 5. O diferencial, em detalhe: restauração resolvida

A restauração hoje falha porque o backup guarda **caminho absoluto**. O SaveVault guarda, junto com
o arquivo, o **significado** dele:

```
save = (emulador, sistema, jogo, tipo de arquivo, caminho relativo à raiz de dados do emulador)
```

Com isso, restaurar deixa de ser copiar caminho e passa a ser resolver quatro perguntas:

| Pergunta | Como o app responde |
|---|---|
| Qual emulador é este? | Assinatura de pasta: nome e estrutura de subpastas, executável, arquivo de config característico (por exemplo `PPSSPP` com `PSP/SAVEDATA`, `PCSX2` com `memcards`, `DuckStation` com `memcards` e `settings.ini`) |
| Onde ele guarda dado do usuário? | Perfil por emulador com as variantes de instalação. Na V1, as de Windows: instalado em `AppData`, portátil ao lado do executável, e `Documentos`. Quando o emulador expõe a raiz no próprio arquivo de config, ler de lá. O modelo já nasce com espaço para XDG e Flatpak, que são a V2 |
| Qual jogo é este arquivo? | Identidade do jogo pelo código de mídia (serial), não pelo nome do arquivo: `SLUS-xxxxx` no PS1 e PS2, `BLUS/NPUB` no PS3, `CUSA-xxxxx` no PS4, Title ID no Switch e no Xbox 360, `ULUS/NPJH` no PSP |
| Este destino já tem save? | Comparação por conteúdo e data, e decisão explícita do usuário quando os dois lados divergem |

Nenhuma dessas quatro respostas pode ser inventada. Se o app não reconhecer a pasta com confiança,
ele **pergunta** em vez de adivinhar e escrever no lugar errado. Um restore errado destrói progresso,
que é exatamente o que o produto promete proteger.

### Emuladores do escopo, fechado em oito

| Emulador | Sistema | Identidade do jogo |
|---|---|---|
| PPSSPP | PSP | Código de disco (`ULUS`, `NPJH`) |
| DuckStation | PS1 | Serial `SLUS`, `SCES` |
| PCSX2 | PS2 | Serial `SLUS`, `SCES` |
| RPCS3 | PS3 | `BLUS`, `BLES`, `NPUB` |
| shadPS4 | PS4 | `CUSA-xxxxx` |
| Sudachi | Switch | Title ID |
| Eden | Switch | Title ID |
| Xenia | Xbox 360 | Title ID |

Os dois novos são bons casos de teste do motor, não só mais itens na lista. O **RPCS3** separa
`savedata` de `trophy` e de `home/<perfil>`, então força o modelo a distinguir tipo de arquivo dentro
do mesmo jogo. O **shadPS4** guarda em `user/savedata/<perfil>/<CUSA>/<pasta do jogo>` e decide entre
`AppData` e pasta portátil pela simples presença da pasta `user` ao lado do executável, que é
exatamente a variante de instalação que quebra restauração baseada em caminho absoluto.

**RetroArch está fora**, decidido, e não é "adiado por falta de tempo": ele agrega dezenas de núcleos
com esquema próprio de nome de arquivo, e resolver isso é um segundo produto dentro do primeiro.

Candidatos para uma rodada futura, sem compromisso: Dolphin, Citra, Vita3K, melonDS, Ryujinx.

## 6. Escopo

### V1

| # | Entrega | Por que está na V1 |
|---|---|---|
| 1 | Descoberta automática de emuladores instalados na máquina | Sem isso o usuário configura na mão e o produto vira o que já existe |
| 2 | Vigilância contínua das pastas de save, com backup versionado por evento de mudança | É o JTBD 1, e é a diferença entre backup e "eu lembrei de fazer backup" |
| 3 | Restauração apontando só a pasta do emulador, com resolução automática do destino | É o diferencial |
| 4 | Backup de jogo de PC, herdado do fork sem reescrever nada | Paridade de graça. O catálogo de +19.000 jogos, as lojas e os saves no registro do Windows já vêm prontos |
| 5 | Resolução de conflito quando origem e destino divergem, mostrando data e jogo | Sem isso o app pode destruir progresso, que anula a proposta |
| 6 | Destino de backup em pasta local e em pasta de nuvem já sincronizada (Drive, OneDrive, Dropbox) | Backup na mesma máquina não protege contra o cenário mais comum, que é formatar |
| 7 | Windows, só | É a máquina do Maycon e o maior público de emulação em desktop |

### V2

Já dimensionado, e fora da V1 de propósito:

| Entrega | Por quê |
|---|---|
| **Migração de save entre emuladores do mesmo sistema** (Sudachi para Eden, Yuzu para Sudachi, e o mesmo caso quando o Xenia ou o RPCS3 mudam de estrutura) | É a extensão natural do motor: se ele já sabe traduzir save em significado e significado em destino, migrar é resolver o destino num emulador diferente do de origem. Dói de verdade em quem acompanhou os forks do Switch. Fica na V2 porque exige o motor da V1 provado antes, e porque cada par de emuladores é uma regra de equivalência a validar caso a caso |
| Linux e SteamOS | O código herdado do Ludusavi já é multiplataforma, então isso não é reescrita: é escrever os perfis de emulador para os caminhos XDG e Flatpak, e validar. É onde o diferencial brilha mais, porque no Steam Deck o caminho nunca é o mesmo do PC |

### Fora do escopo por agora

Registrado para não voltar como surpresa:

- **RetroArch**, pelo motivo já explicado na seção 5.
- macOS.
- Android, que é o segundo maior público de PPSSPP e DuckStation e merece rodada própria.
- Nuvem própria do produto. Grava em pasta que o cliente de nuvem do usuário já sincroniza, ou usa o
  rclone que já vem do fork. Sem servidor, sem conta e sem custo de operação.
- Sincronização bidirecional automática entre duas máquinas. Backup e restauração são explícitos,
  porque sync automático é onde as ferramentas existentes destroem save.
- Download de ROM, gestão de BIOS, atualização de emulador.

Ser Windows-only **não** é arrancar o suporte a Linux e macOS que já existe no código herdado.
É a V1 não construir, não testar e não prometer esses sistemas. Quebrar o que já funciona lá dentro
atrapalharia trazer melhoria do upstream depois.

### Não objetivos

- Não é launcher, não é frontend de emulação, não substitui Playnite nem EmuDeck.
- Não distribui conteúdo protegido: nem ROM, nem BIOS, nem firmware.

## 7. Arquitetura: fork do Ludusavi

**Decidido: fork.** O repositório já tem o histórico completo do upstream
(`mtkennerly/ludusavi`, MIT) e o remote `upstream` configurado, então trazer melhoria de lá é
`git fetch upstream && git merge upstream/master`.

A stack sai decidida por consequência: **Rust**, interface em `iced`, linha de comando em `clap`.
Sem Tauri e sem Electron, porque o app já existe e o que falta é camada dentro dele.

### Onde a camada nova encaixa no código herdado

Levantado lendo o código, não presumido:

| Ponto de extensão | Arquivo | O que muda |
|---|---|---|
| **Raiz de biblioteca** | `src/resource/manifest.rs`, enum `Store` (hoje `Steam`, `Epic`, `Gog`, `Heroic`, `Lutris`, `OtherWindows` e afins) | Emulador entra como novo tipo de raiz. É o conceito que o Ludusavi já usa para "onde procurar jogos", e é exatamente o que um emulador é |
| **Catálogo de jogos** | `src/resource/manifest.rs` | Manifesto de emulador ao lado do manifesto do PCGamingWiki, com o perfil de cada um dos oito e as variantes de instalação. Arquivo de dados atualizável sem soltar versão nova |
| **Varredura** | `src/scan/` | Descoberta por assinatura de pasta, e identidade do jogo por serial ou Title ID em vez de nome de arquivo |
| **Metadado do backup** | `src/resource/config.rs` e a estrutura de backup documentada em `docs/help/backup-structure.md` | Guardar o **significado** do save junto do arquivo, que é o que a seção 5 descreve. É a mudança de fundo, e o resto do motor de restauração depende dela |
| **Restauração** | `src/scan/` e a tela de restauração em `src/gui/` | O motor que resolve o destino, substituindo o `redirect` manual que existe hoje em `docs/help/redirects.md` |
| **Vigilância contínua** | novo | O Ludusavi hoje depende de agendamento externo ou de `wrap` acionado por launcher. Vigiar por evento do sistema de arquivos é código novo |

Duas decisões técnicas que ficam para o refinamento, porque mudam o desenho e não só a
implementação: se o metadado de significado nasce como extensão do formato de backup atual ou como
arquivo paralelo (o primeiro é mais limpo, o segundo facilita o merge com o upstream), e se a
vigilância roda no processo da interface ou num serviço separado.

### Obrigações da licença

O MIT exige manter o aviso de copyright e o texto da licença. O `LICENSE` do upstream veio no fork e
fica. O README já credita o autor original e aponta o repositório de origem.

## 8. Métricas de sucesso

Não definidas. É produto pessoal e de código aberto, sem funil de receita, então o candidato natural
é medir a promessa e não o uso: taxa de restauração que termina sem o usuário precisar apontar
caminho na mão. A instrumentação disso, e se faz sentido ter telemetria em app de código aberto,
fica para a rodada de refinamento com o Maycon.

## 9. Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| Restaurar no lugar errado e destruir progresso | Alto. Mata a proposta do produto | Backup automático do estado atual do destino antes de qualquer escrita, e restauração que pergunta quando não tem certeza |
| O emulador muda a estrutura de pasta numa atualização | Médio. O perfil quebra em silêncio | Perfil de emulador em arquivo de dados versionado e atualizável sem soltar versão nova do app, com validação da assinatura antes de restaurar |
| Emulador que não expõe identidade do jogo no nome do arquivo | Médio. Sobra save órfão | Aceitar o órfão explicitamente: restaurar por caminho relativo e avisar que a identificação não foi possível |
| Vigilância contínua pesar na máquina durante o jogo | Baixo | Vigiar por evento do sistema de arquivos, não por varredura, e agrupar eventos numa janela antes de gravar |
| **O fork divergir tanto do upstream que trazer melhoria vire trabalho manual** | Médio. Perde o benefício que justificou forkar | Camada nova em módulo próprio, tocando o código herdado no mínimo e sempre por ponto de extensão que já existe (o enum `Store`, o manifesto). Mesclar o upstream com frequência, e não em lote de um ano |
| Herdar um projeto grande em Rust que o Maycon não escreveu | Médio | Aceito com consciência: é o preço de não reimplementar 19.000 jogos. Mitiga tocando pouco no que já funciona, e a documentação de ajuda do upstream veio junto |

## 10. Decisões fechadas

| Decisão | Escolha |
|---|---|
| Base do código | Fork do Ludusavi, MIT, Rust com `iced` |
| Emuladores | Oito: PPSSPP, DuckStation, PCSX2, RPCS3, shadPS4, Sudachi, Eden, Xenia |
| RetroArch | Fora |
| Sistema operacional | Windows, só |
| Migração entre emuladores | Dentro do produto, na V2 |
| Nome | `SaveVault`, provisório, mantido por ora |

## 11. Perguntas em aberto para o refinamento

1. **Onde entra a interface da camada nova.** A tela de restauração do Ludusavi hoje é lista de jogos
   com caixa de seleção. O fluxo "aponte a pasta do emulador" é outro fluxo, e a decisão é se ele vira
   uma aba nova ou se reformula a tela que existe. Isso é o próximo passo natural, e é trabalho de
   design.
2. **Métricas.** Ainda não definidas (seção 8).
3. **Ordem de implementação dos oito emuladores.** O motor precisa de um primeiro caso para nascer
   contra algo real. O candidato é o PPSSPP, por ser o mais simples e o de estrutura mais estável, com
   o shadPS4 logo depois por ser o que exercita a variante portátil.
