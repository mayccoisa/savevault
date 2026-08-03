# SaveVault — instruções para o Claude

## Leia primeiro

1. **`.claude/skills/desenvolvimento-savevault/SKILL.md`** — as regras de desenvolvimento deste
   projeto, destiladas de erros reais. Carregue antes de escrever qualquer linha.
2. **`HANDOFF.md`** — o que está pronto, o que foi provado, o que falta e em que ordem.
3. **`docs/prd.md`** — o produto, o escopo e as decisões fechadas.

## O que é este projeto

Fork do [Ludusavi](https://github.com/mtkennerly/ludusavi) (MIT, Rust + iced). Backup e
**restauração inteligente** de saves de jogos, de PC e de emuladores.

O diferencial é a restauração: o usuário aponta a pasta do emulador, ou nem isso, e o app resolve
sozinho onde cada save tem que voltar. Backup de save já existe em várias ferramentas; restauração
que descobre o destino não existia.

Estado atual: fatia do **DuckStation (PS1)** completa e provada ponta a ponta, release
`savevault-v0.1.0`. Windows apenas.

## Idioma

Toda comunicação em **português do Brasil**. Código, comentários de código, mensagens de log e
texto de interface em **inglês**, para não divergir do upstream.

## Comandos essenciais

O Rust não está no PATH por padrão nesta máquina. Prefixe quando necessário:
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`

```bash
cd C:\proj\savevault; cargo test --lib -- --skip scan::registry --skip _with_registry --skip _registry_
```

```bash
cd C:\proj\savevault; cargo clippy --all-targets
```

```bash
cd C:\proj\savevault; cargo run -- emulators
```

Baseline conhecido: **263 testes passam, 0 falham**, 24 filtrados. Clippy sai 0 com dois avisos
herdados que não são para consertar.

## As regras que mais doem se esquecidas

- **`gh` sem `--repo mayccoisa/savevault` age no repositório do upstream.** O remote `upstream`
  existe e o `gh` o prefere.
- **Tags do SaveVault usam o prefixo `savevault-vX.Y.Z`.** As tags `v0.1.0` a `v0.31.0` são as 55
  herdadas do Ludusavi.
- **Os 24 testes de registro falham até rodar `reg import tests/ludusavi.reg`.** Não é regressão.
- **Mescle o upstream antes de começar:** `git fetch upstream; git merge upstream/master`.
- **Destino de emulador não resolvido NUNCA é escrito.** Cair no caminho absoluto do backup escreve
  na pasta de usuário de outra máquina, tem sucesso, e o usuário acredita que restaurou.
- **Chave do jogo é o serial, nunca o título.** A chave é o nome da pasta de backup.
- **Antes de estender uma struct, meça o custo:** `ScanInfo` custa ~2 edições, `ScannedFile` custa
  ~33, `IndividualMapping` ~37.
- **Nunca insira campo em literal com expressão regular frouxa.** Já custou 47 linhas inválidas em
  6 arquivos numa rodada.

## Nunca invente

Layout de pasta de emulador é fato verificável, não palpite. Todo perfil cita a fonte, e o que não
foi confirmado contra instalação real fica registrado como pendência no `HANDOFF.md`, nunca
escondido no código como certeza.

Quando o app não reconhece algo com confiança, ele pergunta ou recusa. Não adivinha em cima de save
de ninguém.
