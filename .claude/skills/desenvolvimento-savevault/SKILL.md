---
name: desenvolvimento-savevault
description: Regras de desenvolvimento do SaveVault (fork do Ludusavi em Rust, backup e restauração inteligente de saves de PC e de emuladores), destiladas de erros reais cometidos em produção — armadilhas do fork (gh resolve para o upstream, tags colidem), invariantes de segurança da restauração, como estender structs que os testes constroem por extenso, e como provar uma fatia ponta a ponta sem emulador instalado. Use SEMPRE que for mexer no SaveVault, principalmente ao acrescentar um emulador novo.
version: 1.3.0
---

# Desenvolvimento do SaveVault

## O que é o produto, em uma frase

Backup e **restauração inteligente** de saves de jogos, de PC e de emuladores. Backup de save já
existe em várias ferramentas. O que não existia é **restauração que descobre o destino sozinha**:
todas as alternativas presumem que o caminho de destino é igual ao de origem, ou pedem que o
usuário mapeie o caminho novo na mão.

Toda decisão técnica se subordina a isso. Se uma mudança facilita o backup e piora a resolução do
destino, ela está errada.

## Onde as coisas vivem

| Coisa | Lugar |
|---|---|
| Repositório | `C:\proj\savevault`, remoto `mayccoisa/savevault` (publico desde 2026-08-03, para a checagem de atualizacao funcionar), `upstream` = `mtkennerly/ludusavi` |
| PRD | Produto **SaveVault** no hub, `kwA6qMaEK6YU4d88IdZy`, doc `zyOp7TCOp8y2rh67IIV0`; cópia local em `docs/prd.md` |
| Handoff e próximos passos | `HANDOFF.md` na raiz do repositório |
| Motor de emuladores | `src/scan/emulator.rs` (perfil como dado) e `src/scan/emulator/psx_card.rs` |
| Resolução do destino | `src/scan/semantic.rs`, função `emulator_restore_target` |
| Metadado no backup | `src/scan/layout.rs`, `SemanticDirKind::Emulator { app, area }` |

## As cinco armadilhas do fork

Estas custaram tempo de verdade. Nenhuma é hipotética.

### 1. O `gh` resolve para o repositório do upstream

Existe o remote `upstream` apontando para `mtkennerly/ludusavi`, então **qualquer** `gh release`,
`gh pr` ou `gh issue` sem alvo explícito tenta agir no repositório de outra pessoa.

**Sempre** passar `--repo mayccoisa/savevault`.

### 2. As tags `v0.1.0` a `v0.31.0` são do Ludusavi

São 55 tags herdadas. A linha de release do SaveVault usa o prefixo **`savevault-vX.Y.Z`**. Sem o
prefixo, a tag colide, e vai colidir nas próximas ~30 versões.

A versão no `Cargo.toml` é a do SaveVault (linha própria, começou em `0.1.0`), e a checagem de
atualização em `src/metadata.rs` aponta para `mayccoisa/savevault`. Se voltar a apontar para o
upstream, o app avisa de uma "atualização" que é de outro projeto.

### 3. Os testes de registro do Windows falham até rodar a preparação

24 testes falham com `Unable to open subkey: NotFound` porque falta o passo único do
`CONTRIBUTING.md`:

```
reg import tests/ludusavi.reg
```

**Não é regressão.** Enquanto esse passo não for dado, rodar a suíte assim:

```bash
cd C:\proj\savevault; cargo test --lib -- --skip scan::registry --skip _with_registry --skip _registry_
```

Baseline conhecido em 2026-08-03 (depois da v0.4.0): **316 passam, 0 falham**, 24 filtrados.

### 4. Compilar exige MSVC e espaço em disco

`rusqlite` vem com SQLite embutido e `reqwest` usa `ring`: as duas compilam C e as duas são
obrigatórias, não dá para desligar por feature. Precisa do **VS Build Tools com a carga de C++**
(já instalado na máquina do Maycon). A toolchain GNU do rustup **não** basta: ela traz linker, mas
não compilador C.

`target/` passa de 6 GB e o perfil de release usa `lto = "thin"`, então a linkagem é lenta.

> **Isto acontece de verdade, e o sintoma engana.** Numa sessão de várias fatias seguidas,
> `target/debug` chegou a **19 GB** e o disco a **zero byte**. O erro que aparece **não** fala em
> disco cheio: é
> `LINK : fatal error LNK1318: Erro PDB inesperado; LIMIT (12)`, apontando para o `.pdb`, o que
> parece defeito de ferramenta. Antes de investigar o linker, rodar
> `Get-PSDrive C`.
>
> Ordem de limpeza, do mais gordo ao menos: `target/debug` (regenerável, ~19 GB, e recompilar
> custa uns 10 minutos), `target/release`, o cache do npm (`%LOCALAPPDATA%\npm-cache`), e as
> pastas sintéticas de teste no temporário. Nunca apagar nada fora de `target/` e do temporário.

### 5. Antes de começar qualquer coisa, mesclar o upstream

```bash
cd C:\proj\savevault; git fetch upstream; git merge upstream/master
```

O `src/scan/semantic.rs` é código **ativo** do upstream, e é a base do motor. Construir sobre uma
base velha gera conflito depois.

## Invariantes de segurança que não se negociam

O produto promete proteger progresso de jogo. Uma restauração errada destrói exatamente o que ele
promete proteger. Estas três regras existem por isso.

### Destino não resolvido NUNCA cai no caminho absoluto do backup

Se um arquivo tem semântica de emulador e o destino não pôde ser determinado (emulador ausente, ou
duas instalações ambíguas), ele **não é escrito**. Entra em `failed_files` com mensagem acionável.

O motivo é concreto: numa máquina diferente, o caminho gravado é a pasta de usuário de outra
pessoa. A escrita **teria sucesso**, criaria a árvore de pastas inteira, e o usuário acreditaria que
restaurou enquanto o emulador não lê nada.

> **O erro que eu cometi aqui:** a checagem dependia de existir contexto de restauração
> (`Option<&semantic::Wine>`). Mas o caso perigoso é exatamente quando **não** existe contexto, ou
> seja, quando nenhum emulador foi encontrado. Contexto ausente significa conjunto vazio de pastas
> de emulador, e é assim que deve ser tratado, nunca como permissão para escrever.

### Ponto de desfazer ANTES da primeira escrita, e aborta se falhar

`GameLayout::snapshot_before_restore` copia todo arquivo vivo que vai ser sobrescrito para
`pre-restore/<timestamp>/`, e é chamado **antes** do laço de escrita. Se falhar, a restauração não
começa. Inverter essa ordem é o que transforma defeito em save perdido.

### Redirect manual do usuário tem precedência

`game_file_target` já dá retorno antecipado quando um redirect configurado pelo usuário mudou o
caminho. Isso é contrato com o usuário. Todo teste de resolução nova precisa de um caso que trave
essa precedência.

## Como acrescentar um emulador

A abstração foi desenhada para isso custar pouco. `Store::Emulator` é **uma** variante, não uma por
emulador, e o emulador vive dentro da struct da raiz. Então:

1. **Acrescentar a variante em `emulator::App`** e um `const` de `Profile` em `src/scan/emulator.rs`.
   O perfil é **dado**: locais padrão, marcador de portátil, assinatura de pasta, e as áreas.
2. **Assinatura pela INSTALAÇÃO, e distinguindo do vizinho.** Duas regras, e as duas custaram
   caro: a marca não pode ser a pasta de saves (ver a seção própria abaixo: o save some justamente
   quando o usuário vai restaurar), e uma marca só não distingue emuladores que compartilham
   convenção (`portable.txt` é do DuckStation **e** do PCSX2). Na prática: configuração e pastas
   de sistema em `all_of`/`any_of`, e o que o vizinho tem e este não tem em `none_of`.
3. **Identidade do jogo vem do conteúdo, não do nome do arquivo.** Foi essa decisão que fez o modo
   "um cartão por título do jogo" funcionar sem código extra: um cartão chamado
   `Final Fantasy VII_1.mcd` é identificado como `SLUS-00594` porque o serial está **dentro** do
   arquivo. Ao portar um emulador novo, procurar primeiro onde o formato guarda a identidade
   (`PARAM.SFO` no PSP e no PS3, nome da pasta `CUSA` no PS4, Title ID no Switch e no 360).
4. **`Area` para cada tipo de arquivo com pasta própria.** O `tail` gravado é relativo à **área**,
   não à pasta de dados. É isso que vai deixar o RPCS3 separar `savedata` de `trophy` sem
   redesenho, e o usuário relocar só uma área na configuração do emulador.
5. **Chave do jogo é a identidade, nunca o título.** A chave é o nome da pasta de backup. Se ela
   dependesse de o título ter sido lido com sucesso, uma falha de leitura criaria uma segunda pasta
   para o mesmo jogo e orfanaria a primeira. Título é camada de exibição
   (`ScanInfo.title` e `IndividualMapping.title`).
6. **Arquivo não identificado não desaparece.** Vira `GameId::Unidentified(nome)`. Ele contém
   progresso; sumir em silêncio é pior que um nome feio.

## A assinatura marca a INSTALAÇÃO, nunca o dado do usuário

A pergunta ao escrever uma assinatura é: **o que continua nesta pasta depois de o save sumir?**

Custou caro descobrir. `memcards/` estava na assinatura do PCSX2. Apagando essa pasta para testar
a restauração, o emulador deixou de ser reconhecido e a restauração recusou com "emulador não
encontrado" — ou seja, o programa se recusava a agir exatamente no cenário que ele existe para
resolver, porque a hora de restaurar é a hora em que o save não está lá.

Aponte para arquivo de configuração (`settings.ini`, `inis/PCSX2.ini`) e pastas de sistema do
emulador (`PSP/SYSTEM`, `dev_flash`, `config`). Pasta de save entra no máximo como alternativa em
`any_of`, para quem ainda não abriu o emulador nesta máquina.

## Onde o backup do emulador mora, e por que não se decide pelo nome

O backup de emulador vai para `backup/<Emulador>/<jogo>`, e **de qual emulador o jogo é vem do
scan** (`BackupSemantics::emulator`), nunca do nome.

Adivinhar pelo nome parece óbvio, porque a chave é `"<Emulador> <identidade>"`. É armadilha: um
jogo de PC chamado **`Eden Ring`** cai dentro da pasta do emulador Eden. A primeira implementação
foi por nome, e o teste a reprovou.

Mover a pasta de um backup é seguro, e vale saber por quê: um backup é encontrado pelo nome gravado
**dentro** do `mapping.yaml`, nunca por onde a pasta está, e esse arquivo guarda os caminhos
originais do jogo, não caminhos dentro do backup. Ainda assim, se a mudança falhar, o backup
continua onde está: layout arrumado nunca vale falhar em salvar progresso.

## Assinatura de pasta: marca positiva não basta, às vezes precisa de marca NEGATIVA

Marcador de portátil é convenção compartilhada. `portable.txt` marca instalação portátil no
DuckStation **e** no PCSX2, e os dois têm `memcards/`. Resultado: uma pasta portátil de PCSX2
casava com os dois perfis, `App::detect` devolvia `None` no empate, e o emulador ficava
**invisível** para o usuário.

`Signature` tem `none_of` por isso. Ao acrescentar um emulador, a pergunta não é só "o que esta
pasta tem?", é **"o que ela tem que o outro não tem, e o que ela não pode ter"**.

O empate ainda vai voltar: Eden e Sudachi são forks da mesma linhagem e terão a mesma estrutura
interna. Ali a distinção vai ter que vir do **nome da pasta de dados**, que hoje `Marker` não
consegue olhar, porque só enxerga para dentro.

## Casar por forma sobrevive ao layout, mas cuidado com o vizinho de mesma forma

No Switch a identidade é o nome de uma **pasta**, o Title ID, 16 hexadecimais. Casar por forma, em
qualquer profundidade, foi a decisão certa: o código da linhagem do yuzu está indisponível (DMCA),
então a profundidade exata não é verificável, e cada fork pode acrescentar um nível.

> **A armadilha:** o primeiro nível abaixo da área é o índice do espaço de save,
> `0000000000000000`, que tem **exatamente a mesma forma**. Casar o mais raso jogava os saves de
> todos os jogos num "jogo" só. A regra que vale é **o casamento mais profundo vence**, e ela está
> travada por teste. Quem for portar Xenia ou Sudachi vai encontrar o mesmo tipo de coisa: quando a
> identidade é um número de tamanho fixo, procure quem mais no caminho tem aquele tamanho.

## Quando o caminho tem um pedaço que muda de máquina, use segmento variável

`AreaSpec.subdir` aceita `*` num segmento, e o RPCS3 declara `dev_hdd0/home/*/savedata`.

O `*` ali é o **id do perfil**, gerado pelo emulador, diferente em cada máquina. A regra que faz
isso valer a pena está em `emulator_restore_target`: o `*` é resolvido **na máquina de destino, na
hora de restaurar**, e nunca reaplicado do backup. Reaplicar o id gravado recriaria a pasta de
perfil da máquina de origem, que fica **correta no disco e invisível para o jogo** — o pior tipo de
defeito deste produto, porque parece sucesso.

Com dois perfis no destino, a restauração **recusa** (`Unresolved::AreaAmbiguous`). Vale aqui a
mesma regra de sempre: ambiguidade não vira palpite.

**Por posição ou por forma.** Quando a profundidade do caminho não está confirmada na fonte, fixar
a posição é aposta. Por isso existe também o segmento `{profile}` (`emulator::PROFILE_SEGMENT`),
que acha a pasta de perfil pela **forma** (32 hexadecimais) em até três níveis: é o que o Eden usa
em `nand/user/save/{profile}`, porque o código do yuzu está indisponível e um fork é livre para
acrescentar um nível.

**E a resolução é assimétrica, de propósito** (`ProfileFallback`): sem perfil identificado, o
**backup** varre a partir do container e a **restauração recusa**. Copiar demais é seguro; escrever
no lugar errado não é. Uniformizar os dois lados parece limpeza e joga fora a garantia.

**Recusar exige dizer a coisa certa.** "Sem perfil" tem variante própria
(`Unresolved::ProfileMissing`) porque, caindo em `EmulatorMissing`, a mensagem mandava adicionar
como raiz uma pasta que já estava adicionada: mandava o usuário fazer o que ele já tinha feito. Ao
acrescentar um motivo de recusa, pergunte **o que o usuário faz a seguir**; se a resposta for a
mesma de um motivo existente, é o mesmo motivo, e se não for, é variante nova.

## A pasta do jogo nem sempre é só save

`AreaSpec.only_subdirs` limita quais subpastas da pasta do jogo são varridas. Existe pelo Xenia,
onde o **jogo instalado** (`00004000`) mora na mesma pasta de título que o save (`00000001`): sem
o filtro, o backup levaria dezenas de GB em silêncio, e o usuário só perceberia pelo tamanho.

Ao acrescentar um emulador, a pergunta é: **o que mais mora nesta pasta além de progresso?** Se
houver jogo, DLC ou cache ali, é `only_subdirs`, não "depois a gente vê".

O filtro vale **um nível**, e isso é essencial: o código do tipo de conteúdo do Xenia tem oito
hexadecimais, igual ao Title ID, então continuar casando a forma abaixo dele faria `00000001`
roubar a identidade do jogo e filtrar o próprio save. É a irmã gêmea da armadilha do índice do
Switch: **quando a identidade é um número de tamanho fixo, procure quem mais no caminho tem aquele
tamanho** — inclusive abaixo.

## Dois emuladores podem casar a mesma pasta, e `None` não resolve

Eden e Sudachi são o mesmo emulador por baixo e têm a **mesma** assinatura. `App::detect` devolvia
`None` no empate, o que parecia prudente e deixava toda pasta da linhagem invisível.

O desempate tem que ser **evidência**, nunca preferência: hoje é o nome da pasta de dados
(`...\eden`, `...\sudachi`). Sem nome que sirva, `None` de novo. A regra geral: no empate, procure
um fato no disco que separe os dois; se não houver, recuse.

## Voltar de versão quebra a configuração

A configuração gravada por uma versão nova **não abre** numa anterior:
`roots: unknown variant 'eden', expected 'duckStation'`. É a mesma razão pela qual a config do
SaveVault não abre no Ludusavi upstream: `Root` é `#[serde(tag = "store")]` sem degradação.

Consequência prática ao testar: para rodar um binário antigo, use um `--config` limpo, nunca o
diretório de configuração que a versão nova já tocou.

## O executor da interface exige `Send`, e o erro da casa não é `Send`

`crate::prelude::AnyError` é `Box<dyn Error>`, que **não** é `Send`. Se ele ficar vivo atravessando
um `await` dentro de um `Task::future`, a compilação falha com "future cannot be sent between
threads safely", e a mensagem aponta para o bloco inteiro, não para a linha culpada.

A regra: **converta o erro para `String` na primeira oportunidade**, antes do próximo `await`. Foi o
que resolveu tanto no `Release::install` (que expõe `Result<(), String>` e guarda o erro boxado numa
função interna) quanto no `Task::future` que o chama.

## A checagem de atualização precisa da API pública, e repositório privado responde 404

A API do GitHub responde **404**, e não 403, para release de repositório privado sem credencial.
Então "não achou atualização" e "não tenho permissão" chegam como a mesma coisa. O repositório é
público desde 2026-08-03 justamente por isso.

E a tag: `Release::parse_tag` aceita **`savevault-vX.Y.Z`** e as tags herdadas `vX.Y.Z`. O código
do upstream fazia só `trim_start_matches('v')`, o que deixa `savevault-v0.2.0`, que não é versão
semântica, e **toda** checagem falhava em silêncio. Se mexer no formato da tag, mexa nesse parse e
no teste que o trava.

## Estender uma struct que os testes constroem por extenso

Este é o custo escondido de tocar o código herdado, e ele varia MUITO:

| Struct | Custo de um campo novo |
|---|---|
| `ScanInfo` | ~2 edições. Os 79 literais quase todos usam `..Default::default()` |
| `ScannedFile` | ~33 edições. Os literais são exaustivos |
| `IndividualMapping` | ~37 edições. Idem |
| `semantic::Wine` | ~13 edições nos testes |

Antes de escolher onde um campo mora, medir isso. Um campo em `ScanInfo` é quase grátis; o mesmo
campo em `ScannedFile` custa 33 edições no código de outra pessoa, que é superfície de conflito em
todo merge do upstream.

> **O erro que eu cometi aqui:** usei uma expressão regular para inserir o campo depois de cada
> `ScannedFile {` ou `Self {`. O padrão também casou `-> Self {` em assinaturas de função e
> `impl ScannedFile {`, e inseriu **47 linhas inválidas** em 6 arquivos. Custou uma rodada inteira
> de conserto.
>
> **Regra:** inserção automática de campo em literal exige âncora que só case literal
> (`^\s*<Nome> \{$`, nunca `Self \{`), e **conferir com `git diff` antes de compilar**. Quando
> forem menos de 10, editar à mão sai mais barato que consertar o estrago.

## Não deixe proteção que não pode disparar

Escrevi uma cerca de contenção contra `..` no caminho gravado. Ela **nunca dispara**, porque
`StrictPath::case_insensitive_tail_for` normaliza `..` antes de comparar, então um caminho com
travessia simplesmente não é reconhecido como estando dentro da área e não devolve cauda nenhuma.

Removi a cerca e travei a **propriedade** com um teste
(`refuses_to_re_anchor_a_path_that_escapes_its_area`), com um comentário dizendo de onde a garantia
vem. Proteção morta com mensagem de usuário morta é pior que nenhuma: dá falsa confiança e o
próximo leitor acredita nela.

## Detalhes de API do Ludusavi que já morderam

- **Chave de arquivo no manifesto usa `StrictPath::globbable()`, nunca `render()`.** O caminho é
  reinterpretado como glob, então um `[` numa pasta como `D:/Emus [novo]/DuckStation` faria a
  entrada casar com nada, em silêncio.
- **`parse_paths` descarta qualquer caminho que ainda contenha `<`.** Placeholder não expandido
  significa save desaparecido sem aviso.
- **`when: [{store: X}]` do manifesto é praticamente ignorado pelo scanner.** A única leitura é um
  caso especial de Uplay. Não dá para filtrar entrada de manifesto por loja sem escrever código.
- **`Root::Emulator` precisa de arm explícito em `src/scan/launchers.rs`.** Sem ele o dispatch cai
  em `generic::scan`, que faz casamento aproximado de nome de pasta e transforma `bios/`, `covers/`
  e `cache/` em jogos que não existem.
- **`Store` tem `#[serde(other)]` em `Other`.** Uma loja desconhecida na configuração degrada em
  silêncio para `Other`, sem erro.
- **A config do SaveVault não abre no Ludusavi upstream.** `Root` é `#[serde(tag = "store")]` sem
  fallback, então `store: emulator` faz o upstream falhar ao ler a config inteira. Porta de mão
  única, aceitável num fork.

## Provar a fatia, sem emulador instalado

Teste de unidade não pega erro de integração. O defeito de segurança mais grave desta base foi
encontrado **rodando**, não lendo.

O caminho é montar uma pasta de dados falsa com arquivos de save **válidos segundo a especificação
do formato**, sintetizados em código (nunca copiados de save real de alguém). O roteiro em
`scripts/` do handoff monta isso para o DuckStation; replicar por emulador novo.

Sequência obrigatória antes de dizer que uma fatia está pronta:

```bash
cd C:\proj\savevault; cargo run -- emulators
cd C:\proj\savevault; cargo run -- backup --force
cd C:\proj\savevault; cargo run -- restore --preview --force
```

E os três casos que precisam ser exercitados de verdade:

1. **Pasta no mesmo lugar:** restaura, sem redirect.
2. **Pasta MOVIDA para outro caminho:** todos os arquivos reancorados sozinhos, zero configuração.
3. **Emulador ausente:** recusa com mensagem, código de saída 1, e **nenhuma pasta fantasma criada**.

O caso 3 é o que pegou o defeito. Nunca pule.

Rodar em modo portátil (`ludusavi.portable` ao lado do executável) para não tocar na configuração
real da máquina.

## Release só quando o usuário pedir

**Não gere release, tag nem build de release por iniciativa própria.** Terminar uma fatia, fechar
um emulador ou deixar a suíte verde **não** autoriza publicar. A publicação acontece só quando o
usuário pede com todas as letras ("gera a release", "manda pra produção", "quero testar aí").

O motivo é o ciclo dele: ele acumula desenvolvimento e testa uma versão só. Release a cada fatia
enche o repositório de versão que ninguém instalou e transforma cada commit num evento.

O fluxo normal enquanto isso: commit no master com os testes verdes, entrada no `CHANGELOG.md` sob
**`## SaveVault (não publicado)`**, e avisar o que ficou pronto. Quando o usuário pedir a release,
essa seção ganha número e data.

## Antes de publicar release (quando o usuário pedir)

1. `cargo test --lib` com os `--skip` de registro, no baseline da seção 3.
2. `cargo clippy --all-targets`: exit 0. Dois avisos são herdados (`src/scan.rs:2239` e
   `examples/api.rs:6`) e não são para consertar.
3. Entrada no `CHANGELOG.md`, na seção do SaveVault no topo, escrita **para quem usa** e não para
   quem commitou.
4. Tag `savevault-vX.Y.Z` e `gh release create ... --repo mayccoisa/savevault`.
5. Anexar o `.zip` do binário de release, para o usuário não precisar de Rust.

## Nunca invente

Vale a regra da casa, e aqui ela tem forma concreta: **layout de pasta de emulador é fato
verificável, não palpite.** Todo perfil deve citar a fonte (documentação oficial do emulador) e o
que não foi confirmado contra instalação real fica registrado como pendência no `HANDOFF.md`, não
escondido no código como se fosse certeza.

Quando o app não reconhece uma pasta com confiança, ele **pergunta ou recusa**, nunca adivinha.

**Fonte é o código do emulador, não fórum.** O caminho do shadPS4 estava anotado como
`user/savedata/<perfil>/<CUSA>`, que é o que dizem os guias de comunidade, e o código diz
`home/<id do usuário>/savedata/<serial>`: o id vem **antes** de `savedata`. Com o caminho errado, o
backup não acharia save nenhum e ninguém saberia por quê. Antes de escrever o perfil, abra o
arquivo do emulador que monta o caminho (`gh api repos/<org>/<repo>/contents/...` resolve, e é mais
confiável que a busca).
