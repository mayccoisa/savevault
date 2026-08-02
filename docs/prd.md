# SaveVault

> Nome provisório. Backup contínuo e restauração inteligente de saves de jogos de PC e de emuladores.

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

Um app grátis e de código aberto que faz o que o Ludusavi faz (backup de save de jogo de PC:
Steam, GOG, Epic, Heroic, Lutris, jogo avulso) **e** trata emulador como cidadão de primeira classe,
com um motor de restauração que resolve o destino sozinho.

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
| Onde ele guarda dado do usuário? | Perfil por emulador com as variantes conhecidas: instalado, portátil, Flatpak, AppData, XDG. Quando o emulador expõe a raiz no próprio arquivo de config, ler de lá |
| Qual jogo é este arquivo? | Identidade do jogo pelo código de mídia (serial), não pelo nome do arquivo: `SLUS-xxxxx` no PS1 e PS2, Title ID no Switch e no Xbox 360, `ULUS/NPJH` no PSP |
| Este destino já tem save? | Comparação por conteúdo e data, e decisão explícita do usuário quando os dois lados divergem |

Nenhuma dessas quatro respostas pode ser inventada. Se o app não reconhecer a pasta com confiança,
ele **pergunta** em vez de adivinhar e escrever no lugar errado. Um restore errado destrói progresso,
que é exatamente o que o produto promete proteger.

### Emuladores do escopo inicial

Escolhidos pelo Maycon, mais os que compartilham o mesmo padrão de pasta e saem de graça:

| Emulador | Sistema | Identidade do jogo |
|---|---|---|
| PPSSPP | PSP | Código de disco (`ULUS`, `NPJH`) |
| DuckStation | PS1 | Serial `SLUS`, `SCES` |
| PCSX2 | PS2 | Serial `SLUS`, `SCES` |
| Sudachi | Switch | Title ID |
| Eden | Switch | Title ID |
| Xenia | Xbox 360 | Title ID |

Candidatos para a rodada seguinte, a confirmar com o Maycon: RetroArch (é caso especial, agrega
vários núcleos), Dolphin, Citra, Vita3K, melonDS, Ryujinx, RPCS3.

## 6. Escopo

### V1

| # | Entrega | Por que está na V1 |
|---|---|---|
| 1 | Descoberta automática de emuladores instalados na máquina | Sem isso o usuário configura na mão e o produto vira o que já existe |
| 2 | Vigilância contínua das pastas de save, com backup versionado por evento de mudança | É o JTBD 1, e é a diferença entre backup e "eu lembrei de fazer backup" |
| 3 | Restauração apontando só a pasta do emulador, com resolução automática do destino | É o diferencial |
| 4 | Backup de jogo de PC reaproveitando o catálogo de +19.000 jogos do Ludusavi | Paridade. É o que o Maycon pediu: ter o que o Ludusavi tem, mais emulador |
| 5 | Resolução de conflito quando origem e destino divergem, mostrando data e jogo | Sem isso o app pode destruir progresso, que anula a proposta |
| 6 | Destino de backup em pasta local e em pasta de nuvem já sincronizada (Drive, OneDrive, Dropbox) | Backup na mesma máquina não protege contra o cenário mais comum, que é formatar |
| 7 | Windows | É a máquina do Maycon e o maior público de emulação em desktop |

### Fora da V1

Registrado para não voltar como surpresa:

- Linux, SteamOS e macOS. Entram na V2, e a arquitetura tem que nascer preparada para isso, mas
  validar em três sistemas ao mesmo tempo mata a V1.
- Android, que é o segundo maior público de PPSSPP e DuckStation e merece rodada própria.
- Nuvem própria do produto. A V1 grava em pasta que o cliente de nuvem do usuário já sincroniza,
  sem servidor, sem conta e sem custo de operação.
- Sincronização bidirecional automática entre duas máquinas. A V1 faz backup e restauração
  explícita, porque sync automático é onde as ferramentas existentes destroem save.
- Download de ROM, gestão de BIOS, atualização de emulador.

### Não objetivos

- Não é launcher, não é frontend de emulação, não substitui Playnite nem EmuDeck.
- Não distribui conteúdo protegido: nem ROM, nem BIOS, nem firmware.

## 7. Arquitetura: as opções

O Ludusavi é MIT e escrito em Rust, então reusar é legítimo e é o caminho que o Maycon pediu.
Três formas, com a recomendação primeiro:

| Opção | Como | Ganho | Custo |
|---|---|---|---|
| **A. Orquestrar o Ludusavi (recomendada)** | App novo chama o binário do Ludusavi via linha de comando para a parte de jogo de PC, e implementa o motor de emulador e o restore inteligente por conta própria | Entrega paridade de PC no dia um, e o catálogo de 19.000 jogos continua sendo mantido por terceiros de graça | Depende de contrato de linha de comando externo, e são dois binários no pacote |
| B. Fork do Ludusavi | Adiciona o módulo de emulador dentro do código em Rust | Binário único, integração profunda | Assume a manutenção de todo o resto, e o Maycon herda um projeto grande em Rust |
| C. Contribuir com o upstream | Propor suporte nativo a emulador no próprio Ludusavi | Bem para a comunidade | Perde o controle de escopo e de prazo, e o diferencial de restauração vira feature de outro produto |

A recomendação é a A, com a nota de que o motor de emulador nasce como módulo separado, de forma que
migrar para B mais tarde continua possível.

Duas coisas a validar tecnicamente antes de fechar a arquitetura:

1. Se o Ludusavi expõe saída legível por máquina (JSON) na linha de comando, e se cobre backup e
   restauração nesse modo. É o que decide se a opção A se sustenta.
2. Se a licença MIT permite empacotar o binário junto na distribuição do SaveVault, incluindo o
   aviso de copyright exigido.

Stack do app em si ainda não decidida. As candidatas naturais são Rust com Tauri, que casa com o
Ludusavi e gera binário pequeno, ou Electron, que é onde o Maycon já tem repertório nos outros
projetos pessoais. Decisão pendente.

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
| RetroArch, que agrega dezenas de núcleos com um esquema próprio | Médio | Fora da V1 de propósito. Entra depois, com perfil próprio |
| Vigilância contínua pesar na máquina durante o jogo | Baixo | Vigiar por evento do sistema de arquivos, não por varredura, e agrupar eventos numa janela antes de gravar |

## 10. Perguntas em aberto para o refinamento

1. Nome. `SaveVault` é provisório.
2. Stack: Tauri com Rust ou Electron.
3. A lista de emuladores da V1 está fechada nesses seis, ou RetroArch é obrigatório desde o começo?
4. Windows na V1 e Linux na V2 está bom, ou o Steam Deck precisa entrar junto?
5. A restauração deve suportar "trazer save da versão antiga do emulador para a nova", que é migração
   entre emuladores do mesmo sistema (por exemplo Sudachi para Eden), ou isso é V2?
