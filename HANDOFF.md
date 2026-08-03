# SaveVault — estado, planejamento e próximos passos

Documento de passagem de bastão. Escrito em 2026-08-03, ao fim da fatia do DuckStation.

**Antes de escrever código, carregue `.claude/skills/desenvolvimento-savevault/SKILL.md`.** Ele tem
as armadilhas do fork e os invariantes de segurança, todos tirados de erros reais desta base.

---

## 1. O produto, e por que ele existe

Quem joga em emulador perde save. O save vive numa pasta interna do emulador, o usuário atualiza o
emulador, troca de build, formata a máquina, migra do PC para o handheld, e o progresso vai junto.
Quando tenta restaurar um backup, tropeça na segunda metade do problema: descobrir para **qual**
pasta cada arquivo tem que voltar, sabendo que o caminho muda por emulador, por versão e por tipo
de instalação.

Pesquisa de mercado feita (está no PRD, seção 1): **existe** ferramenta grátis de backup de save,
inclusive com emulador (Ludusavi, SaveState, EmuSync, SaveSync). O que **não existe** é restauração
que descobre o destino sozinha. Todas presumem que o caminho de destino é igual ao de origem, ou
pedem que o usuário mapeie o caminho novo na mão.

O contrato com o usuário é este:

> Aponte a pasta do emulador. O resto é problema meu.

E, no limite, nem isso: se o emulador estiver num caminho conhecido, o app já acha.

**Toda decisão técnica se subordina a isso.** Se uma mudança facilita o backup e piora a resolução
do destino, ela está errada.

## 2. Decisões já fechadas, não reabrir

| Decisão | Escolha | Por quê |
|---|---|---|
| Base do código | Fork do Ludusavi (MIT, Rust, interface `iced`) | Herda 19.000 jogos de PC, lojas, registro do Windows e nuvem de graça. O esforço fica na única parte que ninguém resolveu |
| Variante de `Store` | **Uma só**, `Store::Emulator`, com o emulador dentro da struct da raiz | Uma variante nova quebra 6 `match` exaustivos. Oito seriam 48 arms e um campo minado em cada merge do upstream |
| Emuladores do escopo | Oito: PPSSPP, DuckStation, PCSX2, RPCS3, shadPS4, Sudachi, Eden, Xenia | Escolha do Maycon |
| RetroArch | **Fora**, decidido | Agrega dezenas de núcleos com esquema próprio de nome de arquivo. É um segundo produto dentro do primeiro |
| Sistema operacional | Windows apenas | O código herdado é multiplataforma e **não deve ser quebrado**, mas a V1 não constrói, não testa e não promete Linux nem macOS |
| Escopo de arquivos | Memory cards **e** savestates. Configuração fica fora | Restaurar configuração cegamente em outra máquina quebra caminho de BIOS, resolução e controle |
| Nome exibido | Título lido de dentro do save, com o serial como apoio | Sem base externa. O `gamedb.yaml` do DuckStation mora na pasta de instalação, que não é alcançável a partir da pasta de dados |
| Chave do jogo | O **serial**, nunca o título | A chave é o nome da pasta de backup. Se dependesse de o título ter sido lido, uma falha criaria uma segunda pasta e orfanaria a primeira |
| Migração entre emuladores | Dentro do produto, mas **V2** | Exige o motor da V1 provado, e cada par de emuladores é uma regra de equivalência a validar |
| Nome do produto | `SaveVault`, provisório | Mantido por ora |
| Nome do binário | Segue `ludusavi.exe` | Renomear move a pasta de config e a de backup de quem já usa: é migração, não renomeação |

## 3. A arquitetura, como está construída

Um save de emulador entra no pipeline como **jogo sintético injetado em memória no manifesto**, pela
mesma porta que jogos customizados já usam (`Manifest::incorporate_extensions`). A partir daí,
varredura, hash, `mapping.yaml`, retenção, backup diferencial, zip, nuvem, lista da interface, CLI e
filtros funcionam **sem uma linha de diff**.

No backup, a pasta de cada área do emulador é gravada no `mapping.yaml` como semântica de diretório.
Na restauração, `game_file_target` (o ponto único de decisão de destino) reancora o caminho na pasta
de dados **desta** máquina.

| Peça | Arquivo | O que faz |
|---|---|---|
| Perfil do emulador, como **dado** | `src/scan/emulator.rs` | `App`, `Area`, `Profile`, `Signature`, `Identity`, `AreaSpec`. Acrescentar emulador é acrescentar um literal |
| Descoberta e atribuição | `src/scan/emulator.rs`, `discover_saves` | Lista as áreas e atribui cada arquivo a um jogo |
| Pastas de dados desta máquina | `src/scan/emulator.rs`, `Roots` | `data_root(app)` devolve `None` quando há zero **ou mais de uma**: ambiguidade não vira palpite |
| Diagnóstico | `src/scan/emulator.rs`, `diagnose` + subcomando `emulators` | Imprime candidatas, veredito da assinatura, escolha e jogos |
| Formato do memory card do PS1 | `src/scan/emulator/psx_card.rs` | Lê serial e título de dentro do `.mcd` |
| Metadado no backup | `src/scan/layout.rs`, `SemanticDirKind::Emulator { app, area }` | Âncora gravada. `Wine` continua serializando como a string `wine`, então backup antigo continua abrindo |
| Resolução do destino | `src/scan/semantic.rs`, `emulator_restore_target` | Devolve `Settled`, `Redirected(path)` ou `Unresolved(reason)` |
| Rede de proteção | `src/scan/layout.rs`, `snapshot_before_restore` e a recusa em `restore` | Ponto de desfazer antes da primeira escrita, e destino não resolvido não é escrito |
| Raiz de emulador | `src/resource/config/root.rs`, `Emulator { path, app }` | No molde de `root::Lutris`, que é o precedente de raiz com campo extra |

### O truque que faz o nome do jogo funcionar

O `.mcd` é uma imagem raw de 131.072 bytes, sem cabeçalho, no formato público do memory card do PS1:

- 16 blocos de 8.192 bytes, cada um com 64 frames de 128 bytes.
- Bloco 0 é o diretório. Frames 1 a 15 descrevem os blocos de save: estado em `0x00..0x04`
  (`0x51` = em uso, primeiro bloco do save), nome do arquivo em `0x0A..0x1F` (ASCII terminado em
  `0x00`), que carrega o código do jogo, por exemplo `BASLUS-00067SOTN`.
- O primeiro frame de cada bloco de save é o "title frame": assinatura `SC` em `0x00..0x02` e o
  **título escrito pelo próprio jogo** em `0x04..0x44`, 64 bytes em Shift-JIS.

Fonte: especificação do nocash (psxspx), "Memory Card Data Format".

Consequências que valem ouro e devem ser preservadas ao portar outros emuladores:

- Nome do jogo **sem base externa, sem internet, em qualquer máquina**.
- Cartão compartilhado nomeia **cada um dos 15 slots** separadamente.
- Cartão nomeado pelo **título** em vez do serial continua sendo identificado, porque a identidade
  vem de dentro do arquivo. Foi essa decisão que neutralizou a maior incerteza do projeto.

## 4. O que está provado, e como

Não é "compila". Foi rodado.

- **263 testes automatizados verdes**, 0 falhas. Clippy exit 0.
- **11 testes só do formato do memory card**, contra `.mcd` sintetizado a partir da especificação:
  cartão vazio, um jogo, cartão compartilhado, save em blocos encadeados contado uma vez, título em
  japonês, título em largura inteira normalizado, frame sem assinatura, arquivo truncado, tamanho
  errado, lixo aleatório.
- **Ciclo completo ponta a ponta**, com pasta de DuckStation sintética:
  1. `emulators` detectou 4 arquivos de save e ignorou `bios/`, `covers/` e `readme.txt`.
  2. `Final Fantasy VII_1.mcd`, nomeado pelo título, foi identificado como `SLUS-00594`.
  3. `shared_card_1.mcd` virou uma entrada só, listando `METAL GEAR SOLID, サガ フロンティア`.
  4. Backup gravou as duas áreas separadamente no `mapping.yaml`.
  5. **Pasta movida** de `DuckStation` para `PortableDuck`: restauração reancorou todos os arquivos
     sozinha, com zero redirect configurado.
  6. **Emulador ausente**: recusou com mensagem acionável, saída 1, e **nenhuma pasta fantasma**.
- Binário de release validado (`ludusavi 0.1.0`) e anexado à release.

O roteiro que monta a pasta falsa está em `scripts/dev/make-fake-duckstation.ps1`. **Replique por
emulador novo.** Teste de unidade não pega erro de integração: o defeito de segurança mais grave
desta base foi encontrado rodando o passo 6, não lendo código.

## 5. Próximos passos, em ordem

Cada item fecha sozinho: compila, tem teste, e pode ser commitado isolado. **Um emulador por
commit.**

### 5.1 PCSX2 (PS2) — FEITO em 2026-08-03

O perfil está em `src/scan/emulator.rs`, com os fatos de layout tirados do código do próprio PCSX2
(`EmuFolders` em `pcsx2/Pcsx2Config.cpp` e `VMManager::GetSaveStateFileName`), citados no comentário
do perfil: pasta de dados `Documents\PCSX2`, portátil por `portable.ini` **ou** `portable.txt`,
`memcards/` com `.ps2`, `sstates/` com `.p2s`, configuração em `inis/PCSX2.ini`.

Três coisas que a fatia ensinou, e que valem para os próximos:

- **A assinatura precisou de marca negativa, não só de uma segunda marca positiva.** `portable.txt`
  é convenção compartilhada: uma pasta portátil de PCSX2 tem `memcards/` e `portable.txt`, e casava
  com o perfil do DuckStation também. Empate faz `App::detect` devolver `None`, ou seja, o emulador
  ficaria **invisível**. `Signature` ganhou `none_of`, e o DuckStation agora recusa pasta com
  `inis/`. Isto chegou bem antes do previsto (esperava-se no par Sudachi/Eden).
- **Identidade do PS2 é opaca de propósito.** O cartão do PS2 é um sistema de arquivos interno, e o
  nome padrão (`Mcd001.ps2`) não carrega serial. Vira `GameId::Unidentified`, que preserva o
  progresso, em vez de palpite. Quem nomeia o cartão pelo serial ganha identificação de graça, pela
  mesma regra do estado salvo.
- **A cópia de segurança do estado (`.p2s.backup`) fica de fora sozinha**, porque a extensão é
  casada por sufixo. É o comportamento desejado: é cópia feita pelo emulador, não progresso novo.

Provado com `scripts/dev/make-fake-pcsx2.ps1` nos três casos obrigatórios: pasta no lugar, pasta
movida (reancorou os 5 arquivos sem redirect) e emulador ausente (recusou, saída 1, nenhuma pasta
fantasma). Suíte em **270 verdes**.

### 5.1b Eden (Switch) — FEITO em 2026-08-03

Pedido do Maycon para vir logo depois do PS2. É o primeiro emulador em que **a pasta identifica o
jogo**, e não o arquivo, então foi ele que estendeu o motor de verdade.

O que entrou: `Identity::TitleIdFolder`, com descida recursiva sob a área. A pasta do título é
procurada **por forma** (16 dígitos hexadecimais) e em **qualquer profundidade**, nunca por posição
fixa. Isso foi decisão forçada pela evidência: o código do yuzu, de onde o Eden deriva, está
indisponível (repositório derrubado por DMCA), então a profundidade exata do caminho não pôde ser
confirmada na fonte. Casar por forma faz o perfil continuar valendo se um fork acrescentar um nível.

**O achado da fatia, que o teste pegou e a leitura de código não pegaria:** o primeiro nível abaixo
de `nand/user/save` é o índice do espaço de save, `0000000000000000`, que tem a **mesma forma** do
Title ID. Casar o mais raso jogava os saves de todos os jogos num "jogo" só. A regra é: **o
casamento mais profundo vence**, travada por teste.

Provado com `scripts/dev/make-fake-eden.ps1` nos três casos, com dois jogos e subpasta dentro do
save. Suíte em **276 verdes**.

**Limitação conhecida, e ela é séria:** o caminho do save tem um nível de **perfil de usuário**
(32 hexadecimais) que é gerado pelo emulador e **muda de máquina para máquina**. A restauração
reancora a pasta de dados corretamente, mas preserva o id de perfil do backup, então numa máquina
nova o save cai numa pasta de perfil que o Eden dessa máquina não usa, e o jogo não enxerga o
progresso. Isto é exatamente o problema que o produto promete resolver, um nível abaixo do que a
V1 resolve hoje: não basta reancorar a pasta de dados, é preciso reancorar o **perfil** também.
É o mesmo problema que o RPCS3 vai trazer (`home/<perfil>`), e a solução deve servir aos dois.
Não foi inventada uma agora de propósito: exige decidir o que fazer quando a máquina de destino
tem zero perfis, um perfil, ou vários, e isso é decisão de produto.

### 5.2 shadPS4 (PS4) — FEITO em 2026-08-03

Entrou **sem estender o motor**: reusou `Identity::PlaystationFolder` (o CUSA tem a mesma forma de
quatro letras e cinco dígitos), o `*` do RPCS3 e a leitura de `PARAM.SFO`. É a evidência de que a
abstração aguenta um console novo sem código novo, só dado.

**O fato pesquisado antes estava errado, e o código do emulador corrigiu.** O caminho anotado aqui
era `user/savedata/<perfil>/<CUSA>/...`, que é o que dizem os guias de comunidade. Em
`src/core/libraries/save_data/save_instance.cpp`, `MakeDirSavePath` monta
`<home>/<id do usuário>/savedata/<serial>/<nome do diretório>`: o id do usuário vem **antes** de
`savedata`. Com o caminho da comunidade, o backup não acharia save nenhum. Vale a regra: fato de
layout se confirma no código do emulador, não em fórum.

Duas coisas específicas do PS4:

- **O `param.sfo` mora em `sce_sys/`, dentro de cada save**, e não na raiz da pasta do jogo. Como o
  jogo (`CUSA00207`) agrupa vários saves (`SPRJ0005`), o título é procurado também **um nível
  abaixo** da pasta que identifica o jogo. Sem isso o usuário vê o serial cru.
- **O `trophy/` da raiz não é progresso do usuário**: guarda ícones e XML que vêm do jogo. O que o
  usuário conquistou está em `home/<id do usuário>/trophy/<NPWR...>.xml`
  (`src/core/libraries/np/np_trophy.cpp`), e é essa a área declarada.

Provado com `scripts/dev/make-fake-shadps4.ps1` nos três casos, com dois jogos e um troféu:
backup agrupado sob `shadPS4/`, restauração numa instalação com **outro** id de usuário (1 → 3)
reancorando os 5 arquivos, e emulador ausente recusando sem criar pasta fantasma.

### 5.3 Sudachi (Switch) e Xenia (Xbox 360) — FEITOS em 2026-08-03

Com estes dois, **o escopo de emuladores fechou**: `App::PLANNED` está vazio e nada na aba diz "em
breve".

**Sudachi: o empate previsto aconteceu, e `None` não servia.** Ele é da mesma linhagem do Eden, com
a **mesma** assinatura, então toda pasta de yuzu passou a casar dois perfis, e `App::detect`
devolvia `None`: a pasta ficava invisível para quem não escolheu o emulador na aba. O desempate é
**evidência do disco, não preferência**: o nome da pasta de dados (`...\eden`, `...\sudachi`). Sem
nome que sirva, continua `None`. A área e a assinatura viraram `SWITCH_SAVE_AREAS` e
`SWITCH_SIGNATURE`, compartilhadas: é o mesmo dado, e se um fork divergir a divergência aparece no
diff.

Sudachi entrou **sem caminho padrão**, de propósito. O código também está indisponível, e o
`%APPDATA%\sudachi` que circula em guias não pôde ser confirmado em fonte do projeto. Anotar o
palpite faria o programa dizer "não encontrado" com toda a confiança numa máquina onde o emulador
está; vazio faz o usuário apontar a pasta uma vez, e a assinatura confere. Ver pendência na seção 6.

**Xenia trouxe a única extensão real desta leva: `AreaSpec.only_subdirs`.** O caminho é
`content/<title id>/<tipo de conteúdo>/`, e o tipo `00004000` é o **jogo instalado**, ao lado do
save (`00000001`). Sem filtrar, o backup levaria dezenas de GB de jogo em silêncio, e o usuário só
descobriria pelo tamanho.

E a armadilha do Switch reapareceu um nível abaixo: o código do tipo de conteúdo tem **oito**
hexadecimais, exatamente como o Title ID do Xbox. Sem parar a descida ali, `00000001` casaria a
forma, roubaria a identidade do jogo e ainda filtraria o próprio save. Daí o `settled`: depois de
passar pelo filtro, nada mais é filtrado e nada mais rouba identidade. Travado por teste.

Provados com `scripts/dev/make-fake-xenia.ps1` e `make-fake-eden.ps1` nos três casos, incluindo o
jogo de 4 KB fingindo de instalado, que ficou de fora do backup.

### 5.4 PPSSPP (PSP) e RPCS3 (PS3) — FEITOS em 2026-08-03

O leitor de `PARAM.SFO` (`src/scan/emulator/param_sfo.rs`) serve aos dois, com o formato tirado do
código do PPSSPP, que é aberto, e 8 testes sobre um SFO sintetizado: arquivo truncado, cabeçalho que
mente na contagem de entradas, valor que não é texto. Isto lê arquivo de disco de outra pessoa, e
arquivo corrompido tem que virar "não sei", nunca uma queda.

**PPSSPP.** A pasta apontada é o **memory stick**, não a pasta do programa. O detalhe que evita
restaurar metade do progresso: os saves `01` e `02` do mesmo jogo são pastas diferentes
(`ULUS1234501`, `ULUS1234502`) e têm que virar o **mesmo** jogo, porque o que identifica é o começo
do nome.

**RPCS3, o teste de fogo: o desenho aguentou, com uma extensão.** Ele não exigiu redesenho, mas
exigiu que o caminho da área pudesse ter um **segmento variável**. `AreaSpec.subdir` agora aceita
`*` num segmento, e o RPCS3 declara `dev_hdd0/home/*/savedata` e `dev_hdd0/home/*/trophy`.

Por que isso importa mais do que parece: o `*` é o **id do perfil**, que o emulador gera e que
difere de máquina para máquina. Ele é resolvido **na máquina de destino, na hora de restaurar**, e
não reaplicado do backup. Sem isso, restaurar noutra máquina recriaria a pasta de perfil da máquina
de origem: correta na aparência, no disco, e **invisível para o jogo**. Com mais de um perfil na
máquina de destino, a restauração **recusa** (`Unresolved::AreaAmbiguous`) em vez de escolher.

Isto é a mesma doença registrada como limitação do Eden, e foi curada lá também: ver 5.9.

Troféu é `Area::Trophies`, separada de propósito: é o que permite restaurar save sem troféu, ou o
contrário. Era exatamente para isto que `Area` existia.

### 5.5 Interface gráfica do fluxo novo — FEITA em 2026-08-03

A decisão pendente era "aba nova ou reformular a tela existente". **O Maycon decidiu: aba nova**,
ao lado de Jogos personalizados, com **todas as configurações de cada emulador dentro do bloco do
próprio emulador**, e os emuladores ainda não implementados listados com o rótulo **Em breve**.

O que isso resolve, e não é cosmético: a raiz de emulador deixou de ser "mais uma linha na lista de
raízes, com a loja escolhida num menu". Na aba, o emulador é o contexto, então a raiz nasce já com
o `app` preenchido (`config::Event::AddEmulatorRoot`). Isso importa porque a detecção por assinatura
não funciona quando a pasta ainda está vazia, que é justamente o caso de quem acabou de instalar.

Listar o que **não** existe ainda é decisão de honestidade: sem isso, o usuário de PPSSPP aponta a
pasta, não acontece nada, e ele conclui que o programa não funciona.

O diagnóstico é lido ao entrar na aba e pelo botão de recarregar, nunca no laço de desenho: ele
lista pastas em disco.

Ainda pendente daqui: o ponto de desfazer existe em disco (`pre-restore/`) mas não tem botão.

### 5.5b Atualização em um clique — FEITA em 2026-08-03

Botão **Verificar atualização** na mesma linha das abas: verifica, baixa o `.zip` da release,
troca o executável e pede para reabrir. O programa em uso não pode ser sobrescrito, mas **pode ser
renomeado**, então a build velha vira `.old` e a nova assume o nome. A build velha é deixada no
disco de propósito: apagar o binário que está executando é o passo que poderia deixar o usuário
sem programa nenhum.

**Um defeito herdado foi corrigido no caminho:** `Release::fetch` fazia
`tag_name.trim_start_matches('v')`, o que deixa `savevault-v0.2.0`, que não é versão semântica.
Ou seja, a verificação de atualização **falhava em silêncio para toda release real do SaveVault**.
Travado por teste em `src/metadata.rs`.

O repositório era privado, e a API pública do GitHub responde **404** para release de repositório
privado, então o botão nunca acharia nada. **O Maycon decidiu tornar o repositório público**, e
está feito: a API anônima já devolve `savevault-v0.2.0` e o `.zip` anexado. Embutir credencial no
programa nunca foi opção: o token iria junto com o binário para a máquina de qualquer usuário.

O que está provado, e o que não está:

- **Provado:** a release é lida sem credencial, a tag é parseada e comparada. Rodando
  `ludusavi api` com `checkAppUpdate` na v0.2.0, a resposta é `update: null`, ou seja, leu e
  concluiu que já está atualizado.
- **Provado que o defeito era real:** o binário da v0.1.0, com o código antigo, responde
  `unexpected character 's' while parsing major version number` ao ler a tag `savevault-v0.2.0`.
  Ou seja, quem está na v0.1.0 **não** se atualiza pelo botão, e precisa baixar a v0.2.0 à mão uma
  vez. Daí em diante o botão funciona.
- **Não exercitado ponta a ponta:** o caminho "existe versão nova → baixa → troca o executável",
  porque não há release mais nova que a atual para baixar. A próxima release é o teste natural
  disso, e é a primeira coisa a conferir quando ela sair.

**Descoberto no caminho, e vale registrar:** a configuração gravada pela v0.2.0 **não abre** numa
versão anterior (`roots: unknown variant 'eden', expected 'duckStation'`). Voltar de versão é porta
de mão única, pela mesma razão que a configuração do SaveVault não abre no Ludusavi upstream:
`Root` é `#[serde(tag = "store")]` sem degradação. Aceitável, mas não é surpresa que o usuário deva
descobrir sozinho.

### 5.6 Externalizar o perfil para `emulators.yaml`

O PRD promete perfil como arquivo de dados atualizável sem soltar versão nova, para o app sobreviver
quando um emulador reorganiza pastas. Hoje o perfil é dado, mas em literal de Rust.

**Fazer isso só depois de dois ou três emuladores.** Desenhar o formato antes de conhecer o que oito
emuladores precisam é desenhar o formato que vai ter que quebrar. Quando fizer, o arquivo precisa de
`schemaVersion`, e versão desconhecida deve **rejeitar o perfil inteiro** em vez de entender pela
metade: perfil meio entendido resolve destino errado e destrói save.

### 5.7 Layout do backup: uma pasta por emulador — FEITO em 2026-08-03

Pedido do Maycon, com motivo concreto vindo do backup real dele: sete pastas de DuckStation e duas
de PPSSPP espalhadas em ordem alfabética no meio de 138 jogos de PC. Agora é
`backup/<Emulador>/<jogo>`.

Duas garantias que não devem ser "simplificadas":

- **De qual emulador o jogo é vem do scan, nunca do nome do jogo.** A chave é
  `"<Emulador> <identidade>"`, então adivinhar pelo nome parece óbvio, e está errado: um jogo de PC
  chamado `Eden Ring` cairia dentro da pasta do emulador Eden. A primeira implementação foi por
  nome e o teste a reprovou.
- **Mover a pasta é seguro porque o backup é encontrado pelo nome dentro do `mapping.yaml`**, nunca
  por onde a pasta está, e esse arquivo guarda os caminhos **originais** do jogo, não caminhos
  dentro do backup. Se a mudança falhar, o backup continua onde está: layout arrumado nunca vale
  falhar em salvar progresso.

A migração dos backups antigos acontece no próximo backup daquele jogo.

### 5.8 A assinatura marcava o dado do usuário — CORRIGIDO em 2026-08-03

**O defeito mais sério encontrado até agora, e ele só apareceu rodando.** Ao apagar `memcards/`
para testar a restauração, o PCSX2 deixou de ser reconhecido e a restauração recusou com "emulador
não encontrado".

A causa: a pasta de **saves** estava na assinatura. Ou seja, a hora em que o usuário mais precisa
restaurar, que é exatamente a hora em que essa pasta sumiu, era a hora em que o programa se
recusava a agir. O produto recusava o cenário que existe para resolver.

Regra que ficou: **a assinatura aponta para a instalação (configuração, pastas de sistema do
emulador), nunca para o dado do usuário.** Ao acrescentar um emulador, a pergunta é "o que continua
aqui depois de o save sumir?".

### 5.6b Filtro "De onde vem" — FEITO em 2026-08-03

A aba de Emuladores organiza a **configuração**, mas a lista de jogos continuava misturando PC e
console, e é lá que o usuário confere o que foi salvo. O filtro novo (`game_filter::Origin`) tem
uma opção para o PC e uma para cada emulador conhecido, montada de `App::ALL`: emulador novo
aparece sozinho.

Duas decisões que carregam o comportamento:

- **Quem decide é o que o scan gravou (`scan.semantics.emulator()`), nunca o nome do jogo.** Pela
  terceira vez nesta base, filtrar pelo nome poria `Eden Ring`, jogo de PC, dentro do emulador
  Eden, porque a chave de um jogo de emulador é `"<Emulador> <identidade>"`. Travado por teste em
  `game_filter.rs`.
- **O controle só aparece se houver raiz de emulador configurada.** Para quem só tem jogo de PC,
  ele não separaria nada, e seria mais um controle na barra de filtros para não usar nunca.

### 5.9 O perfil do Eden — CORRIGIDO em 2026-08-03

Era a limitação séria registrada em 5.1b: o save voltava para a pasta do perfil da máquina de
**origem**, que existe no disco e o jogo não enxerga.

A cura é a do RPCS3, com uma diferença que importa. O PS3 fixa o perfil pela **posição**
(`dev_hdd0/home/*/savedata`); no Eden isso seria uma aposta, porque o código do yuzu está
indisponível e a profundidade nunca pôde ser confirmada, e o perfil foi casado pela **forma**:
`AreaSpec.subdir` aceita o segmento `{profile}` (`emulator::PROFILE_SEGMENT`), que procura, em até
três níveis, a pasta cujo nome tem 32 hexadecimais. Assim `nand/user/save/{profile}` continua
valendo se um fork acrescentar um nível.

**A decisão de desenho que não deve ser "simplificada": a resolução é assimétrica**
(`ProfileFallback`). Sem perfil identificado, o **backup** varre a partir da pasta container, e a
**restauração recusa**. Copiar demais é seguro; escrever no lugar errado não é. Uniformizar os dois
lados parece limpeza e joga fora justamente a garantia.

E o veredito de "sem perfil" ganhou variante própria (`Unresolved::ProfileMissing`). Antes caía em
`EmulatorMissing`, que dizia "o Eden não foi encontrado, adicione a pasta de dados como raiz" — com
o Eden ali, e a raiz já adicionada. A mensagem mandava o usuário fazer o que ele já tinha feito.
Encontrado rodando o caso, não lendo código.

Provado com `scripts/dev/make-fake-eden.ps1` (que agora aceita `-Profile` e `-Empty`, para simular
outra máquina): backup dos dois jogos, restauração numa instalação com **outro** id de perfil
reancorando os 4 arquivos sozinha, recusa com dois perfis, e recusa com nenhum, sempre com saída 1 e
**nenhuma pasta fantasma**.

## 6. Pendências de verificação, herdadas

Confirmado na documentação oficial: pasta de dados do DuckStation, `portable.txt`, e o formato do
`.mcd` com offsets exatos. **Não** confirmado contra instalação real, porque não há DuckStation nesta
máquina:

1. Nome exato do `settings.ini` e se ele fica na raiz da pasta de dados. Afeta a assinatura.
2. Se `memcards/` existe antes do primeiro save. Se nascer depois, instalação nova e vazia não é
   detectada. Aceitável, mas muda a mensagem ao usuário.
3. Nomenclatura do cartão nos três modos (compartilhado, por serial, por título). **Já mitigado**
   pelo desenho: serial e título vêm de dentro do arquivo.
4. Extensão e nomenclatura do savestate. A área casa por glob e identifica por serial quando o nome
   permite, caindo em identidade opaca quando não.
5. Se `portable.txt` vazio basta na build atual.

Do **PCSX2**, tirado do código-fonte do emulador e ainda não confirmado contra instalação real:

6. Se `memcards/` e `inis/` existem antes do primeiro jogo rodar. `inis/` é a marca que distingue do
   DuckStation; se ela nascer só depois do primeiro fechamento do emulador, uma instalação nova e
   nunca aberta não é detectada.
7. **Memory card em pasta** (a opção "folder memory card") é um **diretório**, não um arquivo, e a
   varredura só olha arquivos. Hoje esse usuário fica sem backup do cartão, em silêncio. É a maior
   lacuna conhecida do perfil, e resolver exige área com descida recursiva.
8. Se o `.ps2` de cartão tem alguma âncora estável de serial legível sem montar o sistema de
   arquivos do PS2. Se tiver, a identidade deixa de ser opaca no caso padrão.

Do **Eden**, com a agravante de que o código-fonte da linhagem está indisponível:

9. A profundidade exata de `nand/user/save/<índice>/<perfil>/<title id>`. Mitigado pelo desenho: a
   pasta do título é casada por forma, em qualquer profundidade.
10. Se o Eden aceita a pasta `user` ao lado do executável como instalação portátil, como a
    linhagem do yuzu aceita. O rastreador do projeto sugere que a pasta de dados é sempre
    `%APPDATA%\eden`, então o perfil **não** promete portátil.
11. ~~O empate com o Sudachi.~~ **Resolvido em 5.3**: o desempate é o nome da pasta de dados.

Do **Sudachi**, com a mesma agravante:

12. O caminho padrão da pasta de dados no Windows. Os guias dizem `%APPDATA%\sudachi`, e nenhuma
    fonte do projeto confirma, então o perfil entrou **sem caminho padrão**: o usuário aponta a
    pasta. Confirmando, basta acrescentar o `Anchor` — nada mais muda.

Do **Xenia**:

13. Se `content/` existe antes do primeiro jogo rodar. É a marca da assinatura; se nascer depois,
    uma instalação nova e nunca aberta não é detectada. O `any_of` com `cache/` e
    `xenia.config.toml` reduz o risco, mas não o elimina.
14. Se o Xenia canário (`xenia-canary`), que é a build que a maioria usa, mantém o mesmo
    `content/<title id>/<tipo>/`. O perfil foi tirado do repositório principal.

**O Maycon vai testar no outro PC dele**, que tem DuckStation real. O caminho é:

```bash
cd C:\proj\savevault; cargo run -- emulators
```

A saída desse comando é o que confirma ou corrige os cinco itens acima. Quando ela chegar, ajustar o
perfil e atualizar esta seção.

## 7. Guardas para trabalho não assistido

Se esta sessão está rodando sem o Maycon acompanhando:

- **Não** faça push em `master` com teste vermelho. O baseline é 263 verdes.
- **Não** toque em pasta real de emulador de ninguém. Todo teste usa pasta sintética em diretório
  temporário, e modo portátil (`ludusavi.portable` ao lado do executável) para não mexer na
  configuração real.
- **Não** crie release nem tag sem entrada no `CHANGELOG.md`. É regra da casa.
- **Não** apague nada fora de `target/` e do diretório temporário. O disco desta máquina vive perto
  do limite; se faltar espaço, o alvo grande e regenerável é `%LOCALAPPDATA%\npm-cache`.
- **Um emulador por commit**, com mensagem que diga o que foi verificado e o que ficou pendente.
- Quando um fato de layout não puder ser confirmado, **registre na seção 6 deste arquivo** em vez de
  escrever palpite no código.
- Se algo exigir decisão de produto (nome, escopo, o desenho da tela nova), **pare e deixe
  registrado** em vez de decidir sozinho.

## 8. Onde está o resto do contexto

Tudo está em dois lugares de propósito, porque a próxima sessão pode abrir o repositório **ou**
apontar para o hub.

| Coisa | Onde |
|---|---|
| PRD completo | Produto `SaveVault` no hub (`kwA6qMaEK6YU4d88IdZy`), doc `zyOp7TCOp8y2rh67IIV0`. Cópia local em `docs/prd.md` |
| Esta passagem de bastão | Doc `UHtQ0xDkkZzaw03D2fSH` no mesmo produto, tipo `po`. É este arquivo |
| Regras de desenvolvimento | `.claude/skills/desenvolvimento-savevault/SKILL.md`, e a skill `desenvolvimento-savevault` no hub (sincroniza com `npm run sync:skills`) |
| Instruções carregadas automaticamente | `CLAUDE.md` na raiz |
| Changelog e release | `CHANGELOG.md`, seção `SaveVault v0.1.0`; release `savevault-v0.1.0` no GitHub |
| Ajuda herdada do Ludusavi | `docs/help/`, principalmente `roots.md`, `redirects.md` e `backup-structure.md` |
| Roteiro da pasta falsa | `scripts/dev/make-fake-duckstation.ps1` |
