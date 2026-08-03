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

### 5.1 PCSX2 (PS2) — o mais barato, começar por aqui

Reusa quase tudo, inclusive o decodificador Shift-JIS que já entrou como dependência.

- Pesquisar na documentação oficial: pasta de dados no Windows, marcador de portátil, nome do
  arquivo de configuração, subpasta de memory cards, extensão do cartão.
- Assinatura precisa de **dois** marcadores: `memcards/` sozinho não distingue do DuckStation.
- Identidade: serial `SLUS`/`SCES`, e o `psx_card::media_code_in` já existe e serve.
- O formato do memory card do PS2 **não é** o do PS1. Se a leitura de título exigir muito, aceitar
  `GameId::Unidentified` e registrar como pendência: é melhor que um palpite.

### 5.2 shadPS4 (PS4) — prova a variante portátil

- Fato já pesquisado: `user/savedata/<perfil>/<CUSA>/<pasta do jogo>`, e a escolha entre AppData e
  portátil se dá pela presença da pasta `user` ao lado do executável.
- A identidade é o **nome da pasta** (`CUSA-xxxxx`), não o nome de um arquivo. Isso pede um
  `Identity` novo, do tipo "código no nome do diretório". É a primeira extensão real da abstração:
  se ela exigir mais que um `Identity` novo, vale parar e revisar o desenho.

### 5.3 Xenia (Xbox 360), Sudachi e Eden (Switch)

- Identidade por Title ID cru, aceitando que não haja título legível na V1.
- Sudachi e Eden são forks do mesmo emulador: provavelmente compartilham estrutura, e é o caso que
  vai exercitar a detecção quando **dois** emuladores casam com a mesma pasta.
  `App::detect` hoje devolve `None` no empate, de propósito. Confirmar se é o comportamento certo
  aqui ou se o usuário precisa escolher.

### 5.4 PPSSPP (PSP) e RPCS3 (PS3) — os que trazem código novo

- Ambos guardam o título em `PARAM.SFO`. Um leitor de SFO serve aos dois: vale escrever direito, num
  módulo próprio ao lado de `psx_card.rs`, com testes sobre um SFO sintetizado a partir do formato.
- **RPCS3 é o teste de fogo da abstração:** separa `savedata`, `trophy` e `home/<perfil>`, e o id de
  perfil muda de máquina para máquina. É exatamente por isso que `Area` existe e que o `tail`
  gravado é relativo à área. Se o RPCS3 exigir redesenho, o desenho estava errado.

### 5.5 Interface gráfica do fluxo novo

O motor está pronto; a tela de restauração é a herdada, uma lista de jogos com caixa de seleção.
O fluxo "aponte a pasta do emulador" é outro fluxo, e a decisão pendente é se vira aba nova ou se
reformula a tela existente. **Isso é trabalho de design, não de implementação:** vale desenhar antes
de codar, e passar pelo Maycon.

Junto disso: o ponto de desfazer existe em disco (`pre-restore/`) mas não tem botão.

### 5.6 Externalizar o perfil para `emulators.yaml`

O PRD promete perfil como arquivo de dados atualizável sem soltar versão nova, para o app sobreviver
quando um emulador reorganiza pastas. Hoje o perfil é dado, mas em literal de Rust.

**Fazer isso só depois de dois ou três emuladores.** Desenhar o formato antes de conhecer o que oito
emuladores precisam é desenhar o formato que vai ter que quebrar. Quando fizer, o arquivo precisa de
`schemaVersion`, e versão desconhecida deve **rejeitar o perfil inteiro** em vez de entender pela
metade: perfil meio entendido resolve destino errado e destrói save.

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

| Coisa | Onde |
|---|---|
| PRD completo | Produto `SaveVault` no hub (`kwA6qMaEK6YU4d88IdZy`), doc `zyOp7TCOp8y2rh67IIV0`. Cópia local em `docs/prd.md` |
| Regras de desenvolvimento | `.claude/skills/desenvolvimento-savevault/SKILL.md` |
| Instruções carregadas automaticamente | `CLAUDE.md` na raiz |
| Changelog e release | `CHANGELOG.md`, seção `SaveVault v0.1.0`; release `savevault-v0.1.0` no GitHub |
| Ajuda herdada do Ludusavi | `docs/help/`, principalmente `roots.md`, `redirects.md` e `backup-structure.md` |
| Roteiro da pasta falsa | `scripts/dev/make-fake-duckstation.ps1` |
