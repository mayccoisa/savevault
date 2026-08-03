//! Emulador como cidadão de primeira classe.
//!
//! Um emulador guarda o progresso do usuário numa pasta de dados própria, cujo caminho absoluto
//! muda de máquina para máquina: instalado, portátil, versão antiga do emulador, outro sistema.
//! Isso é conceitualmente o mesmo problema que um prefixo do Wine, então este módulo é a segunda
//! instância do padrão que [`crate::scan::semantic`] já resolve para Wine: uma âncora abstrata
//! (o emulador e a área) mais um caminho relativo estável.
//!
//! Todo o conhecimento específico de cada emulador vive aqui, como **dado**, em [`Profile`].
//! Acrescentar um emulador é acrescentar um literal, não escrever lógica nova.

use crate::{path::CommonPath, prelude::StrictPath, resource::config::Root};

pub mod param_sfo;
pub mod psx_card;

/// Um emulador conhecido.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum App {
    DuckStation,
    Pcsx2,
    Eden,
    Ppsspp,
    Rpcs3,
    ShadPs4,
}

/// Uma área declarada de dado de usuário dentro da pasta de dados do emulador.
///
/// A área existe, e não só a pasta de dados, porque um mesmo jogo tem tipos de arquivo em lugares
/// diferentes, e porque o usuário pode relocar uma área sozinha na configuração do emulador. É o
/// que vai permitir ao RPCS3 separar `savedata` de `trophy` sem redesenho.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Area {
    /// Memory cards, que é o progresso de verdade.
    Memcards,
    /// Estados salvos, que são grandes e atados à build do emulador.
    Savestates,
    /// Save do jogo, quando o emulador não separa cartão de estado. É o caso do Switch, onde o
    /// save é uma pasta por título dentro da NAND emulada.
    Saves,
    /// Troféus. Área própria porque o usuário pode querer restaurar o progresso do jogo sem
    /// mexer nos troféus, ou o contrário.
    Trophies,
}

/// Um lugar padrão onde procurar a pasta de dados.
#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    /// Subpasta de uma pasta conhecida do sistema.
    Common(CommonPath, &'static str),
}

impl Anchor {
    fn resolve(&self) -> Option<StrictPath> {
        match self {
            Self::Common(base, sub) => Some(StrictPath::new(base.get()?).joined(sub)),
        }
    }
}

/// Uma marca usada para reconhecer a pasta de dados de um emulador.
#[derive(Clone, Copy, Debug)]
pub enum Marker {
    Dir(&'static str),
    File(&'static str),
}

impl Marker {
    fn present_in(&self, data_root: &StrictPath) -> bool {
        match self {
            Self::Dir(name) => data_root.joined(name).is_dir(),
            Self::File(name) => data_root.joined(name).is_file(),
        }
    }
}

/// Como reconhecer que uma pasta é a pasta de dados deste emulador.
///
/// Emuladores compartilham nome de subpasta (`memcards` existe no DuckStation e no PCSX2), então
/// uma marca só não basta para não confundir um com o outro.
///
/// > **A marca tem que ser a instalação, nunca o dado do usuário.** Exigir a pasta de saves
/// > parece razoável e está errado: a hora em que o usuário mais precisa restaurar é exatamente
/// > a hora em que essa pasta sumiu. Com `memcards/` na assinatura, apagar essa pasta fazia o
/// > emulador deixar de ser reconhecido, e a restauração recusava com "emulador não encontrado" —
/// > justamente no cenário que o produto existe para resolver. Encontrado rodando, não lendo.
/// > Por isso as assinaturas apontam para arquivo de configuração e pastas de sistema do
/// > emulador, que continuam lá quando o save some.
#[derive(Clone, Copy, Debug)]
pub struct Signature {
    /// Todas estas precisam estar presentes.
    pub all_of: &'static [Marker],
    /// Ao menos uma destas precisa estar presente. Vazio significa "nada a exigir".
    pub any_of: &'static [Marker],
    /// Nenhuma destas pode estar presente.
    ///
    /// Existe porque marcador de portátil é convenção compartilhada: `portable.txt` marca
    /// instalação portátil no DuckStation **e** no PCSX2, e os dois têm `memcards/`. Sem uma
    /// marca negativa, uma pasta portátil de PCSX2 casaria com os dois perfis, e
    /// [`App::detect`] devolveria `None` no empate: o emulador ficaria invisível para o usuário.
    pub none_of: &'static [Marker],
}

impl Marker {
    /// Nome legível da marca, para poder dizer ao usuário **o que** falta na pasta que ele
    /// escolheu, em vez de só dizer que está errada.
    fn label(&self) -> String {
        match self {
            Self::Dir(name) => format!("{name}/"),
            Self::File(name) => name.to_string(),
        }
    }
}

impl Signature {
    /// O que falta nesta pasta para ela ser a pasta de dados deste emulador.
    ///
    /// Vazio significa que não falta nada. Existe porque "essa pasta está errada" sem dizer o que
    /// se esperava encontrar deixa o usuário adivinhando, e adivinhar pasta de save é justamente
    /// o que este programa promete acabar.
    fn missing_in(&self, data_root: &StrictPath) -> Vec<String> {
        let mut missing: Vec<String> = self
            .all_of
            .iter()
            .filter(|marker| !marker.present_in(data_root))
            .map(|marker| marker.label())
            .collect();

        if !self.any_of.is_empty() && !self.any_of.iter().any(|marker| marker.present_in(data_root)) {
            missing.push(
                self.any_of
                    .iter()
                    .map(|marker| marker.label())
                    .collect::<Vec<_>>()
                    .join(" ou "),
            );
        }

        missing
    }

    fn matches(&self, data_root: &StrictPath) -> bool {
        if !data_root.is_dir() {
            return false;
        }
        self.all_of.iter().all(|m| m.present_in(data_root))
            && (self.any_of.is_empty() || self.any_of.iter().any(|m| m.present_in(data_root)))
            && !self.none_of.iter().any(|m| m.present_in(data_root))
    }
}

/// De onde sai a identidade do jogo para os arquivos de uma área.
#[derive(Clone, Copy, Debug)]
pub enum Identity {
    /// De dentro do arquivo, lendo o formato de memory card do PS1.
    ///
    /// É mais robusto que o nome do arquivo: funciona mesmo quando o emulador nomeia o cartão
    /// pelo título do jogo em vez do código de mídia, e num cartão compartilhado enxerga cada
    /// jogo separadamente.
    PsxCard,
    /// Do código de mídia presente no nome do arquivo.
    FilenameMediaCode,
    /// Do nome de uma **pasta** que é o Title ID do console (16 dígitos hexadecimais), com todos
    /// os arquivos abaixo dela pertencendo àquele jogo.
    ///
    /// A pasta é procurada por **forma**, em qualquer profundidade abaixo da área, e não por
    /// posição fixa. É de propósito: no Switch o caminho tem um nível de perfil de usuário no
    /// meio, cujo identificador muda de máquina para máquina, e cada fork do emulador é livre
    /// para acrescentar um nível. Casar por forma sobrevive aos dois.
    TitleIdFolder,
    /// Do nome de uma **pasta** que começa com um identificador da PlayStation (quatro letras e
    /// cinco dígitos), com o nome do jogo lido do `PARAM.SFO` que mora dentro dela.
    ///
    /// Serve ao PSP e ao PS3, que usam a mesma forma de identificador e o mesmo arquivo de
    /// metadado. O nome da pasta traz sufixos que variam (`ULUS1234501`, `BLES00932-AUTO`), então
    /// o que identifica é o começo.
    PlaystationFolder,
    /// Do identificador da PlayStation no começo do nome do arquivo, sem hífen
    /// (`ULUS12345_1.00_1.ppst`).
    FilenamePlaystationId,
}

impl Identity {
    /// Quando a unidade é a pasta, e não o arquivo, esta é a regra que reconhece a pasta do jogo.
    ///
    /// `None` significa que a área é varrida arquivo a arquivo, sem descida recursiva.
    fn folder_matcher(&self) -> Option<fn(&str) -> Option<String>> {
        match self {
            Self::TitleIdFolder => Some(title_id_in),
            Self::PlaystationFolder => Some(param_sfo::title_id_prefix),
            Self::PsxCard | Self::FilenameMediaCode | Self::FilenamePlaystationId => None,
        }
    }
}

/// Uma área da pasta de dados.
#[derive(Clone, Copy, Debug)]
pub struct AreaSpec {
    pub area: Area,
    /// Subpasta relativa à pasta de dados.
    ///
    /// Um segmento igual a `*` casa **um** nível de diretório, qualquer que seja o nome. Existe
    /// para o PS3: o caminho do save é `dev_hdd0/home/<perfil>/savedata`, e o identificador do
    /// perfil é criado pelo emulador e **muda de máquina para máquina**. Sem isso, restaurar
    /// noutra máquina poria o save numa pasta de perfil que o emulador de lá não usa, e o jogo
    /// não enxergaria o progresso: o backup pareceria ter funcionado e não teria.
    pub subdir: &'static str,
    /// Extensões dos arquivos que interessam, sem o ponto. Vazio significa "qualquer arquivo",
    /// que é o caso quando é a pasta que identifica o jogo e o conteúdo dela é opaco.
    pub extensions: &'static [&'static str],
    pub identity: Identity,
}

/// Tudo o que se sabe sobre um emulador. É dado, não lógica.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Nome próprio, não traduzido. Aparece na chave do jogo e no filtro por emulador.
    pub name: &'static str,
    /// Lugares padrão da pasta de dados, em ordem de prioridade.
    pub data_roots: &'static [Anchor],
    /// Arquivos que, ao lado do executável, marcam instalação portátil. Nesse caso a pasta de
    /// dados é a própria pasta do executável. Mais de um porque o PCSX2 aceita dois nomes.
    pub portable_markers: &'static [&'static str],
    pub signature: Signature,
    pub areas: &'static [AreaSpec],
}

const DUCKSTATION: Profile = Profile {
    name: "DuckStation",
    // O DuckStation usa %LOCALAPPDATA%\DuckStation. Instalações antigas usam Documents\DuckStation.
    data_roots: &[
        Anchor::Common(CommonPath::DataLocal, "DuckStation"),
        Anchor::Common(CommonPath::Document, "DuckStation"),
    ],
    portable_markers: &["portable.txt"],
    // A marca é a CONFIGURAÇÃO, não a pasta de saves. Ver a nota em `Signature`: exigir
    // `memcards/` faria o emulador desaparecer justamente na máquina onde o save se perdeu.
    signature: Signature {
        all_of: &[],
        any_of: &[Marker::File("settings.ini"), Marker::File("portable.txt")],
        // O PCSX2 também usa `portable.txt`. O que ele tem e o DuckStation não é a pasta
        // `inis/`, onde mora o `PCSX2.ini`.
        none_of: &[Marker::Dir("inis")],
    },
    areas: &[
        AreaSpec {
            area: Area::Memcards,
            subdir: "memcards",
            extensions: &["mcd", "mcr"],
            identity: Identity::PsxCard,
        },
        AreaSpec {
            area: Area::Savestates,
            subdir: "savestates",
            extensions: &["sav"],
            identity: Identity::FilenameMediaCode,
        },
    ],
};

/// Fonte: código do próprio PCSX2, `EmuFolders` em `pcsx2/Pcsx2Config.cpp`.
///
/// - Pasta de dados no Windows: `Documents\PCSX2` (`SHGetKnownFolderPath(FOLDERID_Documents)`
///   combinado com o literal `"PCSX2"`).
/// - Portátil: `portable.ini` **ou** `portable.txt` ao lado do executável.
/// - Subpastas padrão em `EmuFolders::SetDefaults`: `bios`, `memcards`, `sstates`, `cache`,
///   `covers`; a de configuração é `inis`, onde fica o `PCSX2.ini`.
/// - Nome do estado salvo, em `VMManager::GetSaveStateFileName`:
///   `{serial} ({crc:08X}).{slot:02}.p2s`, por exemplo `SLUS-20062 (7ACF7E77).00.p2s`. O sufixo
///   `.backup` fica de fora por não terminar em `.p2s`, o que é o desejado: é cópia da anterior.
const PCSX2: Profile = Profile {
    name: "PCSX2",
    data_roots: &[Anchor::Common(CommonPath::Document, "PCSX2")],
    portable_markers: &["portable.ini", "portable.txt"],
    // `inis/` é a pasta de configuração, e é a marca própria do PCSX2. `memcards/` não entra:
    // é dado do usuário, e pode ter sumido justamente quando ele vai restaurar.
    signature: Signature {
        all_of: &[Marker::Dir("inis")],
        any_of: &[],
        none_of: &[],
    },
    areas: &[
        AreaSpec {
            area: Area::Memcards,
            subdir: "memcards",
            // O cartão do PS2 é `.ps2`; `.mcd` aparece em cartão importado de outra ferramenta.
            extensions: &["ps2", "mcd"],
            // O cartão do PS2 não é o do PS1: é um sistema de arquivos interno, e o padrão
            // `Mcd001.ps2` não carrega serial nenhum. Identidade opaca é melhor que palpite;
            // ver a pendência do PCSX2 no HANDOFF.md.
            identity: Identity::FilenameMediaCode,
        },
        AreaSpec {
            area: Area::Savestates,
            subdir: "sstates",
            extensions: &["p2s"],
            identity: Identity::FilenameMediaCode,
        },
    ],
};

/// O Eden é um fork da linhagem do yuzu, e herda dela a pasta de dados.
///
/// Fatos usados aqui:
///
/// - Pasta de dados no Windows: `%APPDATA%\eden`, ou seja, o `Roaming`. Confirmado no rastreador
///   do próprio projeto (`eden-emulator/Issue-Reports` nº 252, que pede justamente poder mudar
///   isso: "the emulator data is always stored in %appdata%\eden on windows").
/// - Dentro dela, a NAND emulada em `nand/`, e o save do usuário sob `nand/user/save/`, agrupado
///   numa pasta por **Title ID** (16 dígitos hexadecimais), depois de um nível de perfil de
///   usuário cujo identificador muda de máquina para máquina.
///
/// Por isso a identidade é [`Identity::TitleIdFolder`], que procura a pasta do jogo **por forma
/// e em qualquer profundidade**. O código do emulador está indisponível (o repositório do yuzu
/// foi derrubado por DMCA), então a profundidade exata é o que **não** está confirmado contra
/// instalação real: ver a pendência do Eden no HANDOFF.md. Casar por forma é o que faz o perfil
/// continuar valendo se a profundidade for outra.
const EDEN: Profile = Profile {
    name: "Eden",
    data_roots: &[Anchor::Common(CommonPath::Data, "eden")],
    // A linhagem do yuzu aceita uma PASTA `user` ao lado do executável, e não um arquivo
    // marcador. Como isso não está confirmado para o Eden, aqui não se promete portátil: o
    // usuário aponta a pasta como raiz e a assinatura decide.
    portable_markers: &[],
    signature: Signature {
        all_of: &[Marker::Dir("nand"), Marker::Dir("config")],
        any_of: &[],
        none_of: &[],
    },
    areas: &[AreaSpec {
        area: Area::Saves,
        // O perfil é resolvido na máquina de DESTINO, e não reaplicado do backup. Sem isso o save
        // ia parar numa pasta de perfil que o Eden de lá não usa: correto no disco, invisível
        // para o jogo. Ver `PROFILE_SEGMENT` para o porquê da forma em vez da posição.
        subdir: "nand/user/save/{profile}",
        // O conteúdo do save do Switch é opaco e sem extensão fixa: quem identifica é a pasta.
        extensions: &[],
        identity: Identity::TitleIdFolder,
    }],
};

/// Fonte: código do próprio PPSSPP, que é aberto.
///
/// - A pasta apontada é o **memory stick**, e não a pasta do programa. Em
///   `Windows/main.cpp`, `InitMemstickDirectory`: sem `installed.txt`, o memory stick é a subpasta
///   `memstick` ao lado do executável; quando esse lugar não é gravável (o caso de instalar em
///   Program Files), cai para `Documentos\PPSSPP`.
/// - Em `Core/Util/PathUtil.cpp`, `GetSysDirectory`: `PSP/SAVEDATA` guarda os saves de verdade,
///   `PSP/PPSSPP_STATE` os estados salvos, e `PSP/SYSTEM` a configuração.
/// - Cada save é uma **pasta** cujo nome começa com o identificador do jogo (`ULUS1234501` é o
///   save "01" de `ULUS12345`), com um `PARAM.SFO` dentro declarando o nome do jogo.
const PPSSPP: Profile = Profile {
    name: "PPSSPP",
    data_roots: &[Anchor::Common(CommonPath::Document, "PPSSPP")],
    // O `memstick` do modo portátil é uma PASTA ao lado do executável, e não um arquivo marcador,
    // então não cabe aqui: o usuário aponta essa pasta como raiz e a assinatura decide.
    portable_markers: &[],
    // `PSP/SYSTEM` é a configuração; `PSP/SAVEDATA` é o dado, e serve só como alternativa para
    // quem ainda não abriu o emulador nesta máquina.
    signature: Signature {
        all_of: &[Marker::Dir("PSP")],
        any_of: &[Marker::Dir("PSP/SYSTEM"), Marker::Dir("PSP/SAVEDATA")],
        none_of: &[],
    },
    areas: &[
        AreaSpec {
            area: Area::Saves,
            subdir: "PSP/SAVEDATA",
            extensions: &[],
            identity: Identity::PlaystationFolder,
        },
        AreaSpec {
            area: Area::Savestates,
            subdir: "PSP/PPSSPP_STATE",
            // O `.jpg` ao lado do estado é a miniatura que o próprio emulador mostra na lista;
            // sem ela o usuário restaura e não reconhece o que é cada estado.
            extensions: &["ppst", "jpg"],
            identity: Identity::FilenamePlaystationId,
        },
    ],
};

/// Fonte: código do próprio RPCS3, que é aberto, e a documentação do projeto.
///
/// - Não há caminho padrão a procurar: em `Utilities/File.cpp`, `fs::get_config_dir` devolve a
///   pasta **do próprio executável** no Windows (ou a subpasta `portable/`, quando existe). Por
///   isso `data_roots` está vazio: o usuário aponta a pasta, e a assinatura confere.
/// - A máquina emulada mora em `dev_hdd0/`, e o save do usuário em
///   `dev_hdd0/home/<perfil>/savedata/<TITLE ID + sufixo>/`, com um `PARAM.SFO` dentro.
/// - Os troféus ficam em `dev_hdd0/home/<perfil>/trophy/`, e são **área separada** de propósito:
///   é o que permite ao usuário restaurar save sem troféu, ou o contrário.
///
/// O `*` no meio do caminho é o identificador do perfil, que o emulador cria e que **muda de
/// máquina para máquina**. Ele é resolvido na máquina de destino, no momento da restauração.
const RPCS3: Profile = Profile {
    name: "RPCS3",
    data_roots: &[],
    portable_markers: &[],
    signature: Signature {
        all_of: &[Marker::Dir("dev_hdd0")],
        any_of: &[Marker::Dir("dev_flash"), Marker::Dir("dev_hdd1"), Marker::Dir("config")],
        none_of: &[],
    },
    areas: &[
        AreaSpec {
            area: Area::Saves,
            subdir: "dev_hdd0/home/*/savedata",
            extensions: &[],
            identity: Identity::PlaystationFolder,
        },
        AreaSpec {
            area: Area::Trophies,
            subdir: "dev_hdd0/home/*/trophy",
            extensions: &[],
            identity: Identity::PlaystationFolder,
        },
    ],
};

/// Fonte: código do próprio shadPS4, que é aberto.
///
/// - Em `src/common/path_util.cpp`, a pasta de dados é tentada **primeiro** como portátil, na
///   subpasta `user` ao lado do executável (`PORTABLE_DIR = "user"`), e senão em
///   `%APPDATA%\shadPS4` no Windows (`SHGetFolderPath(CSIDL_APPDATA)`).
/// - As subpastas são criadas **na inicialização**, por `create_path`, antes de existir save
///   nenhum: `shader`, `sys_modules`, `log`, `data`, `home`. É por isso que a assinatura pode
///   apontar para elas, conforme a regra da nota em [`Signature`].
/// - Em `src/core/libraries/save_data/save_instance.cpp`, `MakeDirSavePath` monta
///   `<home>/<id do usuário>/savedata/<serial do jogo>/<nome do diretório>`. Repare na ordem: o
///   id do usuário vem **antes** de `savedata`, e não depois, ao contrário do que dizem os
///   guias de comunidade.
/// - O progresso de troféu do usuário é `<home>/<id do usuário>/trophy/<NPWR...>.xml`, em
///   `src/core/libraries/np/np_trophy.cpp`. A pasta `trophy/` da raiz **não** é isso: ela guarda
///   ícones e XML que vêm do jogo, e não o que o usuário conquistou.
///
/// O `*` no meio do caminho é o id do usuário, que o emulador cria e que **muda de máquina para
/// máquina**. Ele é resolvido na máquina de destino, no momento da restauração.
const SHADPS4: Profile = Profile {
    name: "shadPS4",
    data_roots: &[Anchor::Common(CommonPath::Data, "shadPS4")],
    // O modo portátil é uma PASTA `user` ao lado do executável, e não um arquivo marcador: a
    // pasta apontada É essa `user`, então não há o que marcar aqui.
    portable_markers: &[],
    signature: Signature {
        all_of: &[Marker::Dir("shader"), Marker::Dir("sys_modules")],
        any_of: &[Marker::Dir("home"), Marker::Dir("log")],
        none_of: &[],
    },
    areas: &[
        AreaSpec {
            area: Area::Saves,
            subdir: "home/*/savedata",
            extensions: &[],
            identity: Identity::PlaystationFolder,
        },
        AreaSpec {
            area: Area::Trophies,
            subdir: "home/*/trophy",
            extensions: &["xml"],
            identity: Identity::FilenamePlaystationId,
        },
    ],
};

impl App {
    pub const ALL: &'static [Self] = &[
        Self::DuckStation,
        Self::Pcsx2,
        Self::Eden,
        Self::Ppsspp,
        Self::Rpcs3,
        Self::ShadPs4,
    ];

    pub fn profile(&self) -> &'static Profile {
        match self {
            Self::DuckStation => &DUCKSTATION,
            Self::Pcsx2 => &PCSX2,
            Self::Eden => &EDEN,
            Self::Ppsspp => &PPSSPP,
            Self::Rpcs3 => &RPCS3,
            Self::ShadPs4 => &SHADPS4,
        }
    }

    pub fn name(&self) -> &'static str {
        self.profile().name
    }

    /// Os emuladores que estão no escopo do produto mas ainda não têm perfil.
    ///
    /// Existe para a tela poder mostrá-los como "em breve" em vez de fingir que o escopo é só o
    /// que já foi feito. É dado honesto: o usuário vê o que ainda não vai funcionar, e não
    /// descobre isso apontando a pasta e não acontecendo nada.
    pub const PLANNED: &'static [&'static str] = &["Sudachi", "Xenia"];

    /// Esta pasta é a pasta de dados deste emulador?
    pub fn matches_data_root(&self, data_root: &StrictPath) -> bool {
        self.profile().signature.matches(data_root)
    }

    /// Qual emulador é esta pasta, se algum.
    ///
    /// Devolve `None` quando nenhum reconhece **e também** quando mais de um reconhece, porque
    /// escolher no empate seria adivinhar em cima de dado do usuário.
    pub fn detect(data_root: &StrictPath) -> Option<Self> {
        let mut found = Self::ALL.iter().filter(|app| app.matches_data_root(data_root));
        let first = *found.next()?;
        found.next().is_none().then_some(first)
    }

    /// A pasta de dados de uma instalação apontada pelo usuário.
    ///
    /// Aceita tanto a pasta de dados em si quanto a pasta do executável de uma instalação
    /// portátil, que no DuckStation são a mesma coisa quando existe o marcador.
    pub fn data_root_at(&self, path: &StrictPath) -> Option<StrictPath> {
        self.matches_data_root(path).then(|| path.clone())
    }

    /// Lugares padrão desta máquina que de fato contêm dados deste emulador.
    pub fn installed_data_roots(&self) -> Vec<StrictPath> {
        self.profile()
            .data_roots
            .iter()
            .filter_map(|anchor| anchor.resolve())
            .filter(|candidate| self.matches_data_root(candidate))
            .collect()
    }
}

/// O que o programa tem a dizer sobre **uma** pasta que o usuário escolheu.
///
/// Existe porque o status por emulador não bastava: quando as pastas padrão do sistema existem
/// mas estão vazias, e a instalação de verdade está noutro disco, o programa dizia "usando
/// C:/.../DuckStation (0 arquivos de save)" e o backup vinha vazio. O usuário não tinha como
/// saber se errou a pasta ou se o programa estava quebrado.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderVerdict {
    /// Ainda não foi escolhida uma pasta.
    Empty,
    /// A pasta escolhida não existe.
    Missing,
    /// A pasta existe, mas não é a pasta de dados deste emulador. Lista o que faltou.
    NotThisEmulator { missing: Vec<String> },
    /// É a pasta certa, mas não há save nenhum nela. Lista onde se procurou.
    NoSaves { areas: Vec<String> },
    /// É a pasta certa e tem saves.
    Ready { saves: usize },
}

impl App {
    /// Examina uma pasta escolhida pelo usuário e diz, em um veredito, o que há de errado com
    /// ela, ou quantos saves ela tem.
    ///
    /// Devolve **fato**, não texto de interface: quem traduz é a camada de apresentação.
    pub fn inspect_folder(&self, path: &StrictPath) -> FolderVerdict {
        if path.raw().trim().is_empty() {
            return FolderVerdict::Empty;
        }

        let path = path.interpreted().unwrap_or_else(|_| path.clone());

        if !path.is_dir() {
            return FolderVerdict::Missing;
        }

        let missing = self.profile().signature.missing_in(&path);
        if !missing.is_empty() || !self.matches_data_root(&path) {
            return FolderVerdict::NotThisEmulator { missing };
        }

        let saves = discover_saves(*self, &path).len();
        if saves == 0 {
            return FolderVerdict::NoSaves {
                areas: self.profile().areas.iter().map(|a| a.subdir.to_string()).collect(),
            };
        }

        FolderVerdict::Ready { saves }
    }
}

/// A identidade de um jogo dentro de um emulador.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GameId {
    /// Código de mídia, por exemplo `SLUS-00067`. É a identidade estável.
    Media(String),
    /// Um memory card com saves de mais de um jogo. O arquivo é indivisível, então ele é um
    /// "jogo" só, e não pode ser anexado a cada jogo que mora dentro dele.
    SharedCard,
    /// Arquivo que não deu para identificar. Preservado pelo nome, nunca descartado em silêncio.
    Unidentified(String),
}

impl GameId {
    /// Chave do jogo no manifesto, que é também o nome da pasta de backup.
    ///
    /// Nasce da identidade e **nunca** do título, porque a chave precisa ser a mesma em toda
    /// máquina e em toda execução: se ela dependesse de o título ter sido lido com sucesso, uma
    /// falha de leitura criaria uma segunda pasta de backup para o mesmo jogo.
    pub fn game_key(&self, app: App) -> String {
        let name = app.name();
        match self {
            Self::Media(code) => format!("{name} {code}"),
            Self::SharedCard => format!("{name} shared memory cards"),
            Self::Unidentified(label) => format!("{name} {label}"),
        }
    }
}

impl App {
    /// De qual emulador é esta chave de jogo, se de algum.
    ///
    /// A chave é `"<Emulador> <identidade>"`, montada por [`GameId::game_key`]. Reconhecê-la de
    /// volta é o que permite agrupar o backup numa pasta por emulador, sem gravar nada novo no
    /// backup e sem inventar um segundo lugar onde essa informação viva.
    pub fn from_game_key(key: &str) -> Option<(Self, String)> {
        Self::ALL.iter().find_map(|app| {
            let prefix = format!("{} ", app.name());
            key.strip_prefix(&prefix)
                .filter(|rest| !rest.is_empty())
                .map(|rest| (*app, rest.to_string()))
        })
    }
}

/// Um arquivo de save encontrado, já atribuído a um jogo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredSave {
    pub app: App,
    pub area: Area,
    /// A pasta da área, que é a âncora gravada no backup.
    pub area_root: StrictPath,
    pub game: GameId,
    /// Título para exibição, quando o próprio save carrega um.
    pub title: Option<String>,
    pub file: StrictPath,
}

/// Varre uma pasta de dados e devolve os saves encontrados.
///
/// Custo: uma listagem por área, sem descida recursiva.
pub fn discover_saves(app: App, data_root: &StrictPath) -> Vec<DiscoveredSave> {
    let mut found = vec![];

    for spec in app.profile().areas {
        for area_root in resolve_area_dirs_with(data_root, spec.subdir, ProfileFallback::Container) {
            found.extend(discover_in_area(app, spec, &area_root));
        }
    }

    found
}

/// O segmento que casa a pasta de perfil de usuário **pela forma**, em qualquer profundidade.
///
/// Existe para a linhagem do yuzu, onde o caminho é `nand/user/save/<índice>/<perfil>/<título>` e
/// o código-fonte está indisponível (repositório derrubado por DMCA), então a profundidade exata
/// não pôde ser confirmada na fonte e cada fork é livre para acrescentar um nível. Fixar a
/// posição com dois `*` seria uma aposta; casar por forma sobrevive à aposta errada.
pub const PROFILE_SEGMENT: &str = "{profile}";

/// Até onde descer procurando a pasta de perfil. Três níveis cobrem com folga o índice do espaço
/// de save mais um nível extra de um fork; sem teto, isto varreria a NAND emulada inteira.
const PROFILE_MAX_DEPTH: usize = 3;

/// O nome é um identificador de perfil de usuário do Switch: 32 dígitos hexadecimais.
fn is_profile_id(name: &str) -> bool {
    name.len() == 32 && name.chars().all(|c| c.is_ascii_hexdigit())
}

/// O que fazer quando [`PROFILE_SEGMENT`] não encontra pasta de perfil nenhuma.
///
/// A escolha é deliberadamente **assimétrica**, e é a regra da casa aplicada aos dois sentidos:
/// no backup, copiar demais é seguro, então vale varrer o que houver acima do perfil; na
/// restauração, escrever no lugar errado não é seguro, então sem perfil identificado ela recusa.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileFallback {
    /// Sem perfil, usa a pasta onde o perfil seria procurado. Para varrer.
    Container,
    /// Sem perfil, não devolve nada. Para escrever.
    None,
}

/// Expande o subdir de uma área nas pastas concretas desta máquina.
///
/// Sem curinga, é uma pasta só. Com `*`, é uma por diretório daquele nível. Com
/// [`PROFILE_SEGMENT`], é uma por pasta de perfil encontrada por forma abaixo daqui. Nos dois
/// casos pode ser nenhuma: o usuário que nunca criou um perfil no emulador não tem essa pasta.
pub fn resolve_area_dirs(data_root: &StrictPath, subdir: &str) -> Vec<StrictPath> {
    resolve_area_dirs_with(data_root, subdir, ProfileFallback::None)
}

pub fn resolve_area_dirs_with(data_root: &StrictPath, subdir: &str, fallback: ProfileFallback) -> Vec<StrictPath> {
    let mut current = vec![data_root.clone()];

    for segment in subdir.split('/').filter(|x| !x.is_empty()) {
        let mut next = vec![];

        for path in current {
            match segment {
                "*" => next.extend(child_dirs(&path)),
                PROFILE_SEGMENT => {
                    let before = next.len();
                    collect_profile_dirs(&path, PROFILE_MAX_DEPTH, &mut next);
                    if next.len() == before && fallback == ProfileFallback::Container {
                        next.push(path);
                    }
                }
                _ => next.push(path.joined(segment)),
            }
        }

        // Determinismo: a ordem de listagem do sistema de arquivos não é garantida.
        next.sort_by_key(|path| path.render());
        current = next;
    }

    current.retain(|path| path.is_dir());
    current
}

fn child_dirs(path: &StrictPath) -> Vec<StrictPath> {
    let Ok(entries) = path.read_dir() else { return vec![] };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| StrictPath::from(entry.path()))
        .collect()
}

/// Junta as pastas de perfil abaixo de `path`, sem passar de `depth` níveis.
///
/// Uma pasta de perfil encerra a descida: o que está dentro dela é save de jogo, e continuar
/// desceria pela árvore de saves toda.
fn collect_profile_dirs(path: &StrictPath, depth: usize, found: &mut Vec<StrictPath>) {
    if depth == 0 {
        return;
    }

    for child in child_dirs(path) {
        if child.leaf().is_some_and(|name| is_profile_id(&name)) {
            found.push(child);
        } else {
            collect_profile_dirs(&child, depth - 1, found);
        }
    }
}

fn discover_in_area(app: App, spec: &AreaSpec, area_root: &StrictPath) -> Vec<DiscoveredSave> {
    let mut found = vec![];

    if let Some(matcher) = spec.identity.folder_matcher() {
        return discover_in_game_folders(app, spec, matcher, area_root, area_root, None, None);
    }

    let Ok(entries) = area_root.read_dir() else {
        return found;
    };

    let mut files: Vec<StrictPath> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| StrictPath::from(entry.path()))
        .filter(|file| has_extension(file, spec.extensions))
        .collect();
    // Determinismo: a ordem de listagem do sistema de arquivos não é garantida.
    files.sort_by_key(|file| file.render());

    for file in files {
        found.extend(attribute(app, spec, area_root, &file));
    }

    found
}

fn has_extension(file: &StrictPath, extensions: &[&str]) -> bool {
    // Lista vazia significa "qualquer arquivo": é o caso da área em que quem identifica o jogo é
    // a pasta, e o conteúdo dela é opaco.
    if extensions.is_empty() {
        return true;
    }

    let rendered = file.render().to_lowercase();
    extensions
        .iter()
        .any(|extension| rendered.ends_with(&format!(".{}", extension.to_lowercase())))
}

/// Um Title ID do Switch: 16 dígitos hexadecimais.
///
/// Casado por forma, e não por posição, para o perfil não depender da profundidade exata do
/// caminho, que varia com o perfil de usuário e entre forks do emulador.
fn title_id_in(name: &str) -> Option<String> {
    (name.len() == 16 && name.chars().all(|c| c.is_ascii_hexdigit())).then(|| name.to_ascii_uppercase())
}

/// Desce a área procurando a pasta que identifica o jogo.
///
/// `game` carrega o jogo já reconhecido acima na árvore: uma vez dentro da pasta do título, tudo
/// abaixo é daquele jogo, em qualquer profundidade, porque o formato interno do save é do console
/// e não cabe a este programa interpretar.
#[allow(clippy::too_many_arguments)]
fn discover_in_game_folders(
    app: App,
    spec: &AreaSpec,
    matcher: fn(&str) -> Option<String>,
    area_root: &StrictPath,
    current: &StrictPath,
    game: Option<&GameId>,
    title: Option<&str>,
) -> Vec<DiscoveredSave> {
    let mut found = vec![];

    let Ok(entries) = current.read_dir() else {
        return found;
    };

    let mut children: Vec<(bool, String, StrictPath)> = entries
        .flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            Some((kind.is_dir(), name, StrictPath::from(entry.path())))
        })
        .collect();
    // Determinismo: a ordem de listagem do sistema de arquivos não é garantida.
    children.sort_by_key(|child| child.2.render());

    for (is_dir, name, path) in children {
        if is_dir {
            // O casamento MAIS PROFUNDO vence. Não é detalhe: no Switch o primeiro nível abaixo
            // da área é o índice do espaço de save, que também tem 16 hexadecimais
            // (`0000000000000000`) e portanto também casa a forma. Sem esta regra, todo jogo do
            // usuário seria atribuído a um "jogo" só, o índice. Foi assim que o teste pegou.
            let matched = matcher(&name).map(GameId::Media);
            // O nome do jogo é declarado num arquivo dentro da pasta do jogo, quando o formato
            // tem um. Lido uma vez por pasta de jogo, e não por arquivo.
            let declared = matched.is_some().then(|| read_declared_title(&path)).flatten();
            let deeper = matched.or_else(|| game.cloned());
            let title = declared.as_deref().or(title);

            found.extend(discover_in_game_folders(
                app,
                spec,
                matcher,
                area_root,
                &path,
                deeper.as_ref(),
                title,
            ));
            continue;
        }

        if !has_extension(&path, spec.extensions) {
            continue;
        }

        // Arquivo fora de qualquer pasta de título não some: ele pode ser progresso, e sumir em
        // silêncio é pior que um nome feio.
        let game = game.cloned().unwrap_or_else(|| GameId::Unidentified(file_stem(&path)));
        found.push(DiscoveredSave {
            app,
            area: spec.area,
            area_root: area_root.clone(),
            game,
            title: title.map(|x| x.to_string()),
            file: path,
        });
    }

    found
}

/// O nome do jogo que a própria pasta de save declara, quando há um `PARAM.SFO` ali.
/// Onde o metadado do save pode estar, dentro da pasta do jogo.
///
/// O PSP e o PS3 põem o `PARAM.SFO` na raiz da pasta; o PS4 põe em `sce_sys/param.sfo` (a
/// constante `sce_sys` está em `save_instance.cpp`, no código do shadPS4). É o mesmo formato, e
/// por isso a mesma leitura serve aos três.
const TITLE_METADATA_FILES: &[&str] = &["PARAM.SFO", "sce_sys/param.sfo"];

fn read_declared_title(folder: &StrictPath) -> Option<String> {
    if let Some(title) = title_declared_in(folder) {
        return Some(title);
    }

    // Um nível abaixo, porque no PS4 a pasta do jogo (`CUSA00207`) agrupa os saves e o metadado
    // fica dentro de cada save (`SPRJ0005/sce_sys/param.sfo`). Todos os saves do mesmo jogo
    // declaram o mesmo título, então o primeiro que responder serve.
    child_dirs(folder).iter().find_map(title_declared_in)
}

fn title_declared_in(folder: &StrictPath) -> Option<String> {
    TITLE_METADATA_FILES.iter().find_map(|name| {
        let file = folder.joined(name);
        let bytes = std::fs::read(file.as_std_path_buf().ok()?).ok()?;
        param_sfo::title(&bytes)
    })
}

/// Atribui um arquivo a um ou mais jogos, conforme a regra de identidade da área.
fn attribute(app: App, spec: &AreaSpec, area_root: &StrictPath, file: &StrictPath) -> Vec<DiscoveredSave> {
    let make = |game: GameId, title: Option<String>| DiscoveredSave {
        app,
        area: spec.area,
        area_root: area_root.clone(),
        game,
        title,
        file: file.clone(),
    };

    match spec.identity {
        Identity::PsxCard => {
            let entries = file
                .as_std_path_buf()
                .ok()
                .and_then(|path| std::fs::read(path).ok())
                .map(|bytes| psx_card::read_entries(&bytes))
                .unwrap_or_default();

            let mut codes: Vec<&psx_card::CardEntry> = vec![];
            for entry in &entries {
                if !codes.iter().any(|seen| seen.serial == entry.serial) {
                    codes.push(entry);
                }
            }

            match codes.as_slice() {
                // Cartão vazio, ilegível ou fora do formato: não some, vira jogo pelo nome.
                [] => vec![make(GameId::Unidentified(file_stem(file)), None)],
                // Um jogo só dentro do cartão: o cartão é desse jogo, mesmo que o nome do
                // arquivo diga outra coisa. É o que faz o modo "um cartão por título" funcionar.
                [only] => vec![make(GameId::Media(only.serial.clone()), only.title.clone())],
                // Vários jogos num arquivo indivisível.
                many => {
                    let title = many
                        .iter()
                        .map(|entry| entry.title.clone().unwrap_or_else(|| entry.serial.clone()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    vec![make(GameId::SharedCard, Some(title))]
                }
            }
        }
        // Tratadas antes de chegar aqui, em `discover_saves`, porque a unidade não é o arquivo:
        // é a pasta, e ela precisa de descida recursiva.
        Identity::TitleIdFolder | Identity::PlaystationFolder => vec![],
        Identity::FilenamePlaystationId => {
            let stem = file_stem(file);
            let game = match param_sfo::title_id_prefix(&stem) {
                Some(code) => GameId::Media(code),
                None => GameId::Unidentified(stem),
            };
            vec![make(game, None)]
        }
        Identity::FilenameMediaCode => {
            let stem = file_stem(file);
            let game = match psx_card::media_code_in(&stem) {
                Some(code) => GameId::Media(code),
                None => GameId::Unidentified(stem),
            };
            vec![make(game, None)]
        }
    }
}

fn file_stem(file: &StrictPath) -> String {
    let rendered = file.render();
    let name = rendered.rsplit(['/', '\\']).next().unwrap_or(&rendered);
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name.to_string(),
    }
}

/// Pastas de dados de emulador conhecidas nesta máquina.
///
/// É o análogo de [`crate::scan::semantic::Prefix`] para o Wine: o estado do sistema que a
/// restauração precisa para reancorar um caminho gravado no backup.
#[derive(Clone, Debug, Default)]
pub struct Roots {
    entries: Vec<(App, StrictPath)>,
}

impl Roots {
    /// Monta a lista a partir das raízes configuradas pelo usuário e, para emuladores sem raiz
    /// configurada, dos lugares padrão desta máquina.
    pub fn for_config(roots: &[Root]) -> Self {
        let mut entries: Vec<(App, StrictPath)> = vec![];

        for root in roots {
            let Root::Emulator(emulator) = root else {
                continue;
            };
            let Ok(path) = emulator.path.interpreted() else {
                continue;
            };
            let Some(app) = emulator.app.or_else(|| App::detect(&path)) else {
                continue;
            };
            if let Some(data_root) = app.data_root_at(&path) {
                push_unique(&mut entries, app, data_root);
            }
        }

        for app in App::ALL {
            if entries.iter().any(|(known, _)| known == app) {
                continue;
            }
            for data_root in app.installed_data_roots() {
                push_unique(&mut entries, *app, data_root);
            }
        }

        Self { entries }
    }

    /// A pasta de dados deste emulador nesta máquina.
    ///
    /// `None` quando não há nenhuma **e também** quando há mais de uma, porque duas instalações
    /// do mesmo emulador (a antiga em Documentos e a nova em AppData, por exemplo) tornam o
    /// destino indeterminado, e escrever save no lugar errado é o pior defeito possível aqui.
    pub fn data_root(&self, app: App) -> Option<&StrictPath> {
        let mut matching = self.entries.iter().filter(|(known, _)| *known == app);
        let first = matching.next()?;
        matching.next().is_none().then_some(&first.1)
    }

    /// Todas as candidatas de um emulador, para o diagnóstico poder explicar uma ambiguidade.
    pub fn candidates(&self, app: App) -> Vec<&StrictPath> {
        self.entries
            .iter()
            .filter(|(known, _)| *known == app)
            .map(|(_, path)| path)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Builds the list directly, so tests of the restore engine do not need a real install.
    #[cfg(test)]
    pub fn for_test(entries: Vec<(App, StrictPath)>) -> Self {
        Self { entries }
    }

    pub fn iter(&self) -> impl Iterator<Item = (App, &StrictPath)> {
        self.entries.iter().map(|(app, path)| (*app, path))
    }
}

/// What the app currently believes about the emulators on this system.
///
/// This exists so that checking the profile against a real install is one command and one printout
/// instead of a debugging session. The folder layouts here were taken from each emulator's own
/// documentation, and documentation and reality drift.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnosis {
    pub emulators: Vec<AppDiagnosis>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDiagnosis {
    pub name: &'static str,
    /// Every folder looked at, and what the signature said about it.
    pub candidates: Vec<CandidateDiagnosis>,
    /// The folder that will actually be used, if the answer is unambiguous.
    pub data_root: Option<String>,
    /// Why no folder will be used, when that is the case.
    pub problem: Option<String>,
    pub games: Vec<GameDiagnosis>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDiagnosis {
    pub path: String,
    pub exists: bool,
    pub matches_signature: bool,
    /// Whether it came from the user's configuration rather than from probing.
    pub configured: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDiagnosis {
    pub key: String,
    pub title: Option<String>,
    pub area: Area,
    pub file: String,
}

/// Collects the diagnosis. Read-only: it never writes anything.
pub fn diagnose(roots: &[Root]) -> Diagnosis {
    let resolved = Roots::for_config(roots);

    let emulators = App::ALL
        .iter()
        .map(|app| {
            let mut candidates: Vec<CandidateDiagnosis> = vec![];

            for root in roots {
                let Root::Emulator(emulator) = root else { continue };
                if emulator.app.is_some_and(|configured| configured != *app) {
                    continue;
                }
                let path = emulator.path.interpreted().unwrap_or_else(|_| emulator.path.clone());
                candidates.push(CandidateDiagnosis {
                    exists: path.is_dir(),
                    matches_signature: app.matches_data_root(&path),
                    path: path.render(),
                    configured: true,
                });
            }

            for anchor in app.profile().data_roots {
                let Some(path) = anchor.resolve() else { continue };
                if candidates.iter().any(|seen| seen.path == path.render()) {
                    continue;
                }
                candidates.push(CandidateDiagnosis {
                    exists: path.is_dir(),
                    matches_signature: app.matches_data_root(&path),
                    path: path.render(),
                    configured: false,
                });
            }

            let data_root = resolved.data_root(*app);
            let problem = match (data_root, resolved.candidates(*app).len()) {
                (Some(_), _) => None,
                (None, 0) => Some(format!(
                    "{} was not found on this system. Saves cannot be backed up or restored for it.",
                    app.name()
                )),
                (None, found) => Some(format!(
                    "{found} data folders were found for {}, so the destination is ambiguous. \
                     Keep only the one you want as a root.",
                    app.name()
                )),
            };

            let games = data_root
                .map(|root| {
                    discover_saves(*app, root)
                        .into_iter()
                        .map(|save| GameDiagnosis {
                            key: save.game.game_key(*app),
                            title: save.title,
                            area: save.area,
                            file: save.file.render(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            AppDiagnosis {
                name: app.name(),
                candidates,
                data_root: data_root.map(|x| x.render()),
                problem,
                games,
            }
        })
        .collect();

    Diagnosis { emulators }
}

impl Diagnosis {
    /// Human-readable report.
    pub fn render(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        for app in &self.emulators {
            let _ = writeln!(out, "{}", app.name);

            for candidate in &app.candidates {
                let verdict = match (candidate.exists, candidate.matches_signature) {
                    (false, _) => "not present",
                    (true, false) => "present, but does not look like this emulator's data folder",
                    (true, true) => "matches",
                };
                let source = if candidate.configured { "configured" } else { "probed" };
                let _ = writeln!(out, "  [{source}] {} : {verdict}", candidate.path);
            }
            if app.candidates.is_empty() {
                let _ = writeln!(out, "  no candidate folders");
            }

            match (&app.data_root, &app.problem) {
                (Some(root), _) => {
                    let _ = writeln!(out, "  using: {root}");
                }
                (None, Some(problem)) => {
                    let _ = writeln!(out, "  PROBLEM: {problem}");
                }
                (None, None) => {}
            }

            if app.games.is_empty() {
                let _ = writeln!(out, "  no saves found");
            } else {
                let _ = writeln!(out, "  {} save file(s):", app.games.len());
                for game in &app.games {
                    let title = game.title.as_deref().unwrap_or("(no title in the save)");
                    let _ = writeln!(out, "    {} | {title} | {:?} | {}", game.key, game.area, game.file);
                }
            }

            let _ = writeln!(out);
        }

        out
    }
}

fn push_unique(entries: &mut Vec<(App, StrictPath)>, app: App, path: StrictPath) {
    if !entries
        .iter()
        .any(|(known, known_path)| *known == app && known_path.equivalent(&path))
    {
        entries.push((app, path));
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// Monta uma pasta de dados de DuckStation num diretório temporário.
    struct FakeInstall {
        _dir: tempfile::TempDir,
        root: StrictPath,
    }

    impl FakeInstall {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = StrictPath::from(dir.path().to_path_buf());
            Self { _dir: dir, root }
        }

        fn dir(self, name: &str) -> Self {
            std::fs::create_dir_all(self.root.joined(name).as_std_path_buf().unwrap()).unwrap();
            self
        }

        fn file(self, name: &str, contents: &[u8]) -> Self {
            let path = self.root.joined(name);
            if let Some((parent, _)) = name.rsplit_once('/') {
                std::fs::create_dir_all(self.root.joined(parent).as_std_path_buf().unwrap()).unwrap();
            }
            std::fs::write(path.as_std_path_buf().unwrap(), contents).unwrap();
            self
        }

        /// Uma instalação normal: pasta de memory cards e arquivo de configuração.
        fn installed() -> Self {
            Self::new().dir("memcards").file("settings.ini", b"[Main]\n")
        }

        /// Uma instalação portátil: o marcador em vez do arquivo de configuração.
        fn portable() -> Self {
            Self::new().dir("memcards").file("portable.txt", b"")
        }

        /// Uma instalação de PCSX2: `memcards` e a pasta de configuração `inis`.
        fn pcsx2() -> Self {
            Self::new().dir("memcards").dir("inis").file("inis/PCSX2.ini", b"[UI]\n")
        }

        /// Uma instalação portátil de PCSX2, que compartilha o `portable.txt` com o DuckStation.
        fn pcsx2_portable() -> Self {
            Self::pcsx2().file("portable.txt", b"")
        }
    }

    /// Um memory card válido com um jogo dentro, para os testes de descoberta.
    fn card_with(games: &[(&str, &str)]) -> Vec<u8> {
        let mut bytes = vec![0u8; psx_card::CARD_SIZE];
        for slot in 1..=15usize {
            let at = 128 * slot;
            bytes[at..at + 4].copy_from_slice(&0x0000_00A0u32.to_le_bytes());
        }
        for (index, (filename, title)) in games.iter().enumerate() {
            let slot = index + 1;
            let at = 128 * slot;
            bytes[at..at + 4].copy_from_slice(&0x0000_0051u32.to_le_bytes());
            let raw = filename.as_bytes();
            bytes[at + 0x0A..at + 0x0A + raw.len()].copy_from_slice(raw);

            let at = 8192 * slot;
            bytes[at..at + 2].copy_from_slice(b"SC");
            let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(title);
            bytes[at + 0x04..at + 0x04 + encoded.len()].copy_from_slice(&encoded);
        }
        bytes
    }

    #[test]
    fn recognizes_an_installed_data_root() {
        let install = FakeInstall::installed();
        assert!(App::DuckStation.matches_data_root(&install.root));
        assert_eq!(Some(App::DuckStation), App::detect(&install.root));
    }

    #[test]
    fn recognizes_a_portable_data_root() {
        let install = FakeInstall::portable();
        assert!(App::DuckStation.matches_data_root(&install.root));
    }

    #[test]
    fn rejects_a_folder_without_the_required_marker() {
        // Só `memcards` não basta: o PCSX2 também tem essa pasta.
        let install = FakeInstall::new().dir("memcards");
        assert!(!App::DuckStation.matches_data_root(&install.root));
        assert_eq!(None, App::detect(&install.root));
    }

    /// Sem a pasta de saves, a instalação continua sendo reconhecida.
    ///
    /// Este teste já afirmou o contrário, e o contrário estava errado: a hora em que o usuário
    /// mais precisa restaurar é exatamente a hora em que a pasta de saves não existe mais. Com
    /// ela na assinatura, a restauração recusava dizendo que o emulador não foi encontrado.
    #[test]
    fn recognizes_an_install_whose_saves_are_gone() {
        let install = FakeInstall::new().file("settings.ini", b"[Main]\n");
        assert!(App::DuckStation.matches_data_root(&install.root));

        let pcsx2 = FakeInstall::new().dir("inis").file("inis/PCSX2.ini", b"[UI]\n");
        assert!(App::Pcsx2.matches_data_root(&pcsx2.root));
    }

    #[test]
    fn rejects_a_missing_folder() {
        assert!(!App::DuckStation.matches_data_root(&StrictPath::new("Z:/nao/existe")));
    }

    #[test]
    fn discovers_a_game_by_the_content_of_its_card() {
        let install = FakeInstall::installed().file(
            "memcards/SLUS-00067_1.mcd",
            &card_with(&[("BASLUS-00067SOTN", "CASTLEVANIA SOTN")]),
        );

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Media("SLUS-00067".to_string()), found[0].game);
        assert_eq!(Some("CASTLEVANIA SOTN".to_string()), found[0].title);
        assert_eq!(Area::Memcards, found[0].area);
        assert_eq!(install.root.joined("memcards"), found[0].area_root);
    }

    /// O modo "um cartão por título do jogo" nomeia o arquivo com o título, não com o código.
    /// Como a identidade vem de DENTRO do arquivo, isso continua funcionando.
    #[test]
    fn identifies_a_card_named_after_the_title_not_the_serial() {
        let install = FakeInstall::installed().file(
            "memcards/Final Fantasy VII_1.mcd",
            &card_with(&[("BASLUS-00594FF7", "FF7-01")]),
        );

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Media("SLUS-00594".to_string()), found[0].game);
    }

    #[test]
    fn a_card_with_several_games_becomes_one_shared_entry() {
        let install = FakeInstall::installed().file(
            "memcards/shared_card_1.mcd",
            &card_with(&[
                ("BASLUS-00067SOTN", "CASTLEVANIA SOTN"),
                ("BASLUS-00594FF7", "FF7-01"),
            ]),
        );

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::SharedCard, found[0].game);
        assert_eq!(Some("CASTLEVANIA SOTN, FF7-01".to_string()), found[0].title);
        assert_eq!(
            "DuckStation shared memory cards",
            found[0].game.game_key(App::DuckStation)
        );
    }

    /// Um cartão ilegível não pode desaparecer em silêncio: ele contém progresso.
    #[test]
    fn an_unreadable_card_is_kept_as_unidentified() {
        let install = FakeInstall::installed().file("memcards/broken_1.mcd", b"nada a ver");

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Unidentified("broken_1".to_string()), found[0].game);
        assert_eq!(None, found[0].title);
        assert_eq!("DuckStation broken_1", found[0].game.game_key(App::DuckStation));
    }

    #[test]
    fn ignores_files_of_other_kinds_and_other_folders() {
        let install = FakeInstall::installed()
            .dir("bios")
            .dir("covers")
            .dir("cache")
            .file("bios/scph1001.bin", b"nao e save")
            .file("covers/SLUS-00067.jpg", b"nao e save")
            .file("memcards/readme.txt", b"nao e save")
            .file("settings.ini", b"[Main]\n");

        assert_eq!(Vec::<DiscoveredSave>::new(), discover_saves(App::DuckStation, &install.root));
    }

    #[test]
    fn discovers_savestates_by_media_code_in_the_filename() {
        let install = FakeInstall::installed()
            .dir("savestates")
            .file("savestates/SLUS-00067_1.sav", b"estado");

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(Area::Savestates, found[0].area);
        assert_eq!(GameId::Media("SLUS-00067".to_string()), found[0].game);
    }

    #[test]
    fn keeps_a_savestate_whose_name_has_no_media_code() {
        let install = FakeInstall::installed()
            .dir("savestates")
            .file("savestates/resume.sav", b"estado");

        let found = discover_saves(App::DuckStation, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Unidentified("resume".to_string()), found[0].game);
    }

    #[test]
    fn recognizes_a_pcsx2_data_root() {
        let install = FakeInstall::pcsx2();
        assert!(App::Pcsx2.matches_data_root(&install.root));
        assert_eq!(Some(App::Pcsx2), App::detect(&install.root));
    }

    /// O caso que obrigou a marca negativa: `memcards` e `portable.txt` existem nos dois
    /// emuladores, então sem ela a pasta casaria com ambos e `detect` devolveria `None`,
    /// deixando o emulador invisível.
    #[test]
    fn a_portable_pcsx2_folder_is_not_confused_with_duckstation() {
        let install = FakeInstall::pcsx2_portable();
        assert!(App::Pcsx2.matches_data_root(&install.root));
        assert!(!App::DuckStation.matches_data_root(&install.root));
        assert_eq!(Some(App::Pcsx2), App::detect(&install.root));
    }

    #[test]
    fn a_duckstation_folder_is_not_confused_with_pcsx2() {
        let install = FakeInstall::installed();
        assert!(!App::Pcsx2.matches_data_root(&install.root));
        assert_eq!(Some(App::DuckStation), App::detect(&install.root));
    }

    /// O nome do estado salvo do PCSX2 traz o serial antes do CRC entre parênteses.
    #[test]
    fn discovers_a_pcsx2_savestate_by_its_serial() {
        let install = FakeInstall::pcsx2()
            .dir("sstates")
            .file("sstates/SLUS-20062 (7ACF7E77).00.p2s", b"estado");

        let found = discover_saves(App::Pcsx2, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(Area::Savestates, found[0].area);
        assert_eq!(GameId::Media("SLUS-20062".to_string()), found[0].game);
        assert_eq!("PCSX2 SLUS-20062", found[0].game.game_key(App::Pcsx2));
    }

    /// A cópia de segurança do estado é do emulador, não do usuário: não vira jogo.
    #[test]
    fn ignores_the_pcsx2_savestate_backup() {
        let install = FakeInstall::pcsx2()
            .dir("sstates")
            .file("sstates/SLUS-20062 (7ACF7E77).00.p2s.backup", b"estado");

        assert_eq!(Vec::<DiscoveredSave>::new(), discover_saves(App::Pcsx2, &install.root));
    }

    /// O cartão do PS2 é um sistema de arquivos interno, e o nome padrão não carrega serial.
    /// Ele não some: vira identidade opaca, preservando o progresso que está dentro dele.
    #[test]
    fn a_default_pcsx2_memory_card_is_kept_as_unidentified() {
        let install = FakeInstall::pcsx2().file("memcards/Mcd001.ps2", b"cartao");

        let found = discover_saves(App::Pcsx2, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(Area::Memcards, found[0].area);
        assert_eq!(GameId::Unidentified("Mcd001".to_string()), found[0].game);
        assert_eq!("PCSX2 Mcd001", found[0].game.game_key(App::Pcsx2));
    }

    /// Quem nomeia o cartão por jogo ganha a identificação de graça, pela mesma regra do
    /// estado salvo.
    #[test]
    fn a_pcsx2_memory_card_named_after_the_serial_is_identified() {
        let install = FakeInstall::pcsx2().file("memcards/SLUS-20062.ps2", b"cartao");

        let found = discover_saves(App::Pcsx2, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Media("SLUS-20062".to_string()), found[0].game);
    }

    /// Uma pasta de dados de Eden, com o save do Switch na profundidade documentada:
    /// `nand/user/save/<índice>/<perfil de usuário>/<title id>`.
    fn eden_install() -> FakeInstall {
        FakeInstall::new().dir("nand").dir("config")
    }

    #[test]
    fn recognizes_an_eden_data_root() {
        let install = eden_install();
        assert!(App::Eden.matches_data_root(&install.root));
        assert_eq!(Some(App::Eden), App::detect(&install.root));
    }

    #[test]
    fn discovers_a_switch_save_by_the_title_id_folder() {
        let install = eden_install()
            .file(
                "nand/user/save/0000000000000000/ABCDEF0123456789ABCDEF0123456789/0100000000010000/progress.dat",
                b"save",
            )
            .file(
                "nand/user/save/0000000000000000/ABCDEF0123456789ABCDEF0123456789/0100000000010000/meta/header.bin",
                b"save",
            );

        let found = discover_saves(App::Eden, &install.root);

        assert_eq!(2, found.len());
        for save in &found {
            assert_eq!(Area::Saves, save.area);
            assert_eq!(GameId::Media("0100000000010000".to_string()), save.game);
            // A âncora é a pasta do PERFIL, e não `nand/user/save`: é o perfil que muda de
            // máquina para máquina, então é ele que a restauração precisa poder trocar.
            assert!(save.area_root.equivalent(
                &install
                    .root
                    .joined("nand/user/save/0000000000000000/ABCDEF0123456789ABCDEF0123456789")
            ));
        }
        assert_eq!("Eden 0100000000010000", found[0].game.game_key(App::Eden));
    }

    /// Sem pasta de perfil, o backup ainda varre: perder save por causa de um caminho que não
    /// bate com o esperado é pior do que copiar demais. A restauração é que recusa, e isso está
    /// travado em [`crate::scan::semantic`].
    #[test]
    fn backs_up_a_switch_save_even_when_there_is_no_profile_folder() {
        let install = eden_install().file("nand/user/save/0000000000000000/0100000000010000/a.dat", b"save");

        let found = discover_saves(App::Eden, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Media("0100000000010000".to_string()), found[0].game);
        assert_eq!(install.root.joined("nand/user/save"), found[0].area_root);
    }

    /// O perfil é achado pela FORMA (32 hexadecimais), em qualquer profundidade, e não pela
    /// posição. O código do yuzu está indisponível, então fixar a posição seria uma aposta, e um
    /// fork é livre para acrescentar um nível.
    #[test]
    fn finds_the_profile_folder_at_any_depth() {
        let install = eden_install().file(
            "nand/user/save/0000000000000000/extra/ABCDEF0123456789ABCDEF0123456789/0100000000010000/a.dat",
            b"save",
        );

        let found = discover_saves(App::Eden, &install.root);

        assert_eq!(1, found.len());
        assert!(found[0].area_root.equivalent(
            &install
                .root
                .joined("nand/user/save/0000000000000000/extra/ABCDEF0123456789ABCDEF0123456789")
        ));
    }

    /// A pasta do jogo é casada por FORMA, então um nível a mais ou a menos no caminho (perfil de
    /// usuário, índice de espaço de save, ou o que um fork resolva acrescentar) não quebra o
    /// perfil. É o que substitui a profundidade fixa, que não pôde ser confirmada no código.
    #[test]
    fn finds_the_title_id_folder_at_any_depth() {
        let shallow = eden_install().file("nand/user/save/0100000000010000/progress.dat", b"save");
        let deep = eden_install().file(
            "nand/user/save/0000000000000000/perfil/mais/um/nivel/0100000000010000/progress.dat",
            b"save",
        );

        for install in [shallow, deep] {
            let found = discover_saves(App::Eden, &install.root);
            assert_eq!(1, found.len());
            assert_eq!(GameId::Media("0100000000010000".to_string()), found[0].game);
        }
    }

    /// Uma pasta cujo nome não é Title ID não vira jogo, mas o arquivo dentro dela também não
    /// desaparece: ele pode ser progresso.
    #[test]
    fn a_switch_file_outside_a_title_folder_is_kept_as_unidentified() {
        let install = eden_install().file("nand/user/save/avulso.bin", b"save");

        let found = discover_saves(App::Eden, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Unidentified("avulso".to_string()), found[0].game);
    }

    /// O índice do espaço de save (`0000000000000000`) tem a mesma forma do Title ID, e vem
    /// ANTES dele no caminho. Se o casamento mais raso vencesse, os saves de todos os jogos
    /// cairiam num "jogo" só.
    #[test]
    fn the_save_space_index_does_not_steal_the_identity_of_the_game() {
        let install = eden_install()
            .file(
                "nand/user/save/0000000000000000/ABCDEF0123456789ABCDEF0123456789/0100000000010000/a.dat",
                b"save",
            )
            .file(
                "nand/user/save/0000000000000000/ABCDEF0123456789ABCDEF0123456789/0100000000020000/b.dat",
                b"save",
            );

        let found = discover_saves(App::Eden, &install.root);

        assert_eq!(2, found.len());
        assert_eq!(GameId::Media("0100000000010000".to_string()), found[0].game);
        assert_eq!(GameId::Media("0100000000020000".to_string()), found[1].game);
    }

    /// Uma pasta de RPCS3, com um perfil de usuário.
    fn rpcs3_install(profile: &str) -> FakeInstall {
        FakeInstall::new()
            .dir("dev_hdd0")
            .dir("dev_flash")
            .dir(&format!("dev_hdd0/home/{profile}/savedata"))
            .dir(&format!("dev_hdd0/home/{profile}/trophy"))
    }

    #[test]
    fn recognizes_an_rpcs3_folder() {
        let install = rpcs3_install("00000001");
        assert!(App::Rpcs3.matches_data_root(&install.root));
        assert_eq!(Some(App::Rpcs3), App::detect(&install.root));
    }

    /// O identificador do perfil está no meio do caminho e muda de máquina para máquina, então o
    /// caminho da área é declarado com `*` e resolvido na hora.
    #[test]
    fn discovers_ps3_saves_under_any_profile_id() {
        let install = rpcs3_install("00000042")
            .file(
                "dev_hdd0/home/00000042/savedata/BLES00932-AUTO/PARAM.SFO",
                &param_sfo_with_title("DEMONS SOULS"),
            )
            .file("dev_hdd0/home/00000042/savedata/BLES00932-AUTO/SAVEDATA", b"save")
            .file("dev_hdd0/home/00000042/trophy/BLES00932_00/TROPUSR.DAT", b"trofeu");

        let found = discover_saves(App::Rpcs3, &install.root);

        let saves: Vec<_> = found.iter().filter(|x| x.area == Area::Saves).collect();
        assert_eq!(2, saves.len());
        assert_eq!(GameId::Media("BLES00932".to_string()), saves[0].game);
        assert_eq!(Some("DEMONS SOULS".to_string()), saves[0].title);
        assert_eq!(
            install.root.joined("dev_hdd0/home/00000042/savedata"),
            saves[0].area_root
        );

        // O troféu é área própria: o usuário pode restaurar um sem o outro.
        assert!(found.iter().any(|x| x.area == Area::Trophies));
    }

    /// Uma pasta de dados de shadPS4, que é a mesma no modo instalado e no portátil.
    fn shadps4_install(user: &str) -> FakeInstall {
        FakeInstall::new()
            .dir("shader")
            .dir("sys_modules")
            .dir("log")
            .dir(&format!("home/{user}/savedata"))
            .dir(&format!("home/{user}/trophy"))
    }

    #[test]
    fn recognizes_a_shadps4_folder() {
        let install = shadps4_install("1");
        assert!(App::ShadPs4.matches_data_root(&install.root));
        assert_eq!(Some(App::ShadPs4), App::detect(&install.root));
    }

    /// A assinatura marca a INSTALAÇÃO: apagar o save não pode fazer o emulador sumir, senão a
    /// restauração recusa justamente na máquina que precisa dela.
    #[test]
    fn recognizes_a_shadps4_install_whose_saves_are_gone() {
        let install = FakeInstall::new().dir("shader").dir("sys_modules").dir("log");
        assert!(App::ShadPs4.matches_data_root(&install.root));
    }

    /// A ordem do caminho é `home/<id do usuário>/savedata/<serial>`, com o id do usuário ANTES
    /// de `savedata`. Os guias de comunidade dizem o contrário; quem manda é o código.
    #[test]
    fn discovers_ps4_saves_and_trophies_under_any_user_id() {
        let install = shadps4_install("1")
            .file(
                "home/1/savedata/CUSA00207/SPRJ0005/sce_sys/param.sfo",
                &param_sfo_with_title("BLOODBORNE"),
            )
            .file("home/1/savedata/CUSA00207/SPRJ0005/userdata0", b"save")
            .file("home/1/trophy/NPWR12345_00.xml", b"<trophy/>");

        let found = discover_saves(App::ShadPs4, &install.root);

        let saves: Vec<_> = found.iter().filter(|x| x.area == Area::Saves).collect();
        assert_eq!(2, saves.len());
        for save in &saves {
            assert_eq!(GameId::Media("CUSA00207".to_string()), save.game);
            // O `param.sfo` do PS4 mora em `sce_sys/`, e não na raiz da pasta do save.
            assert_eq!(Some("BLOODBORNE".to_string()), save.title);
        }
        assert!(saves[0].area_root.equivalent(&install.root.joined("home/1/savedata")));

        let trophies: Vec<_> = found.iter().filter(|x| x.area == Area::Trophies).collect();
        assert_eq!(1, trophies.len());
        assert_eq!(GameId::Media("NPWR12345".to_string()), trophies[0].game);
    }

    #[test]
    fn a_wildcard_segment_expands_to_every_profile() {
        let install = rpcs3_install("00000001").dir("dev_hdd0/home/00000002/savedata");

        let dirs = resolve_area_dirs(&install.root, "dev_hdd0/home/*/savedata");

        assert_eq!(2, dirs.len());
        assert_eq!(install.root.joined("dev_hdd0/home/00000001/savedata"), dirs[0]);
        assert_eq!(install.root.joined("dev_hdd0/home/00000002/savedata"), dirs[1]);
    }

    #[test]
    fn a_wildcard_segment_with_nothing_to_match_expands_to_nothing() {
        let install = FakeInstall::new().dir("dev_hdd0/home");
        assert!(resolve_area_dirs(&install.root, "dev_hdd0/home/*/savedata").is_empty());
    }

    /// Os cinco vereditos, que são o que o usuário lê ao lado da pasta que escolheu.
    ///
    /// O caso que motivou tudo isto é o quarto: as pastas padrão do sistema **existem** e casam a
    /// assinatura, mas estão vazias, porque a instalação de verdade está noutro disco. Sem esta
    /// distinção o programa dizia "usando <pasta>" e o backup vinha vazio, sem explicação.
    #[test]
    fn tells_the_user_what_is_wrong_with_the_folder_they_chose() {
        assert_eq!(
            FolderVerdict::Empty,
            App::DuckStation.inspect_folder(&StrictPath::new("".to_string()))
        );
        assert_eq!(
            FolderVerdict::Missing,
            App::DuckStation.inspect_folder(&StrictPath::new("Z:/nao/existe".to_string()))
        );

        let wrong = FakeInstall::new().dir("bios");
        assert_eq!(
            FolderVerdict::NotThisEmulator {
                missing: vec!["settings.ini ou portable.txt".to_string()],
            },
            App::DuckStation.inspect_folder(&wrong.root)
        );

        let empty = FakeInstall::installed();
        assert_eq!(
            FolderVerdict::NoSaves {
                areas: vec!["memcards".to_string(), "savestates".to_string()],
            },
            App::DuckStation.inspect_folder(&empty.root)
        );

        let with_saves = FakeInstall::installed().file(
            "memcards/SLUS-00067_1.mcd",
            &card_with(&[("BASLUS-00067SOTN", "CASTLEVANIA SOTN")]),
        );
        assert_eq!(
            FolderVerdict::Ready { saves: 1 },
            App::DuckStation.inspect_folder(&with_saves.root)
        );
    }

    /// Apontar a pasta de um emulador para o perfil de outro tem que dizer o que faltou, e não
    /// só que está errada.
    #[test]
    fn the_verdict_names_what_the_folder_is_missing() {
        let pcsx2 = FakeInstall::pcsx2();

        let FolderVerdict::NotThisEmulator { missing } = App::DuckStation.inspect_folder(&pcsx2.root) else {
            panic!("a pasta do PCSX2 não é a do DuckStation");
        };

        assert!(missing.iter().any(|x| x.contains("settings.ini")));
    }

    /// Dezesseis hexadecimais é a forma; qualquer outra coisa não é Title ID.
    #[test]
    fn only_sixteen_hex_digits_count_as_a_title_id() {
        assert_eq!(Some("0100000000010000".to_string()), title_id_in("0100000000010000"));
        assert_eq!(Some("ABCDEF0123456789".to_string()), title_id_in("abcdef0123456789"));
        assert_eq!(None, title_id_in("010000000001000"));
        assert_eq!(None, title_id_in("01000000000100000"));
        assert_eq!(None, title_id_in("0100000000g10000"));
    }

    /// Um `PARAM.SFO` válido segundo o formato, para os testes de PSP e PS3.
    fn param_sfo_with_title(title: &str) -> Vec<u8> {
        let key = b"TITLE\0";
        let mut value = title.as_bytes().to_vec();
        value.push(0);

        let mut out = vec![];
        out.extend_from_slice(&0x4653_5000u32.to_le_bytes());
        out.extend_from_slice(&0x0000_0101u32.to_le_bytes());
        out.extend_from_slice(&36u32.to_le_bytes()); // tabela de chaves
        out.extend_from_slice(&(36 + key.len() as u32).to_le_bytes()); // tabela de dados
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // chave em 0
        out.extend_from_slice(&0x0204u16.to_le_bytes()); // texto UTF-8
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // dado em 0
        out.extend_from_slice(key);
        out.extend_from_slice(&value);
        out
    }

    /// Um memory stick de PPSSPP.
    fn ppsspp_install() -> FakeInstall {
        FakeInstall::new().dir("PSP").dir("PSP/SAVEDATA").dir("PSP/SYSTEM")
    }

    #[test]
    fn recognizes_a_ppsspp_memory_stick() {
        let install = ppsspp_install();
        assert!(App::Ppsspp.matches_data_root(&install.root));
        assert_eq!(Some(App::Ppsspp), App::detect(&install.root));
    }

    /// O nome do jogo sai do `PARAM.SFO` que mora dentro da pasta do save, que é o análogo, para
    /// a PlayStation portátil, do título escrito dentro do memory card do PS1.
    #[test]
    fn discovers_a_psp_save_and_reads_its_title() {
        let install = ppsspp_install()
            .file(
                "PSP/SAVEDATA/ULUS1234501/PARAM.SFO",
                &param_sfo_with_title("MONSTER HUNTER FREEDOM UNITE"),
            )
            .file("PSP/SAVEDATA/ULUS1234501/DATA.BIN", b"save");

        let found = discover_saves(App::Ppsspp, &install.root);

        assert_eq!(2, found.len());
        for save in &found {
            assert_eq!(Area::Saves, save.area);
            assert_eq!(GameId::Media("ULUS12345".to_string()), save.game);
            assert_eq!(Some("MONSTER HUNTER FREEDOM UNITE".to_string()), save.title);
        }
        assert_eq!("PPSSPP ULUS12345", found[0].game.game_key(App::Ppsspp));
    }

    /// Os saves "01" e "02" do mesmo jogo são pastas diferentes, e têm que virar o MESMO jogo:
    /// senão o usuário acha que tem dois jogos e restaura só metade do progresso.
    #[test]
    fn two_save_slots_of_the_same_psp_game_are_one_game() {
        let install = ppsspp_install()
            .file("PSP/SAVEDATA/ULUS1234501/DATA.BIN", b"save")
            .file("PSP/SAVEDATA/ULUS1234502/DATA.BIN", b"save");

        let found = discover_saves(App::Ppsspp, &install.root);

        assert_eq!(2, found.len());
        assert_eq!(GameId::Media("ULUS12345".to_string()), found[0].game);
        assert_eq!(GameId::Media("ULUS12345".to_string()), found[1].game);
    }

    /// Save sem `PARAM.SFO` não some nem fica sem identidade: perde só o nome bonito.
    #[test]
    fn a_psp_save_without_metadata_still_has_its_id() {
        let install = ppsspp_install().file("PSP/SAVEDATA/ULUS1234501/DATA.BIN", b"save");

        let found = discover_saves(App::Ppsspp, &install.root);

        assert_eq!(1, found.len());
        assert_eq!(GameId::Media("ULUS12345".to_string()), found[0].game);
        assert_eq!(None, found[0].title);
    }

    #[test]
    fn discovers_a_ppsspp_savestate_and_its_thumbnail() {
        let install = ppsspp_install()
            .dir("PSP/PPSSPP_STATE")
            .file("PSP/PPSSPP_STATE/ULUS12345_1.00_1.ppst", b"estado")
            .file("PSP/PPSSPP_STATE/ULUS12345_1.00_1.jpg", b"miniatura");

        let found = discover_saves(App::Ppsspp, &install.root);

        assert_eq!(2, found.len());
        for save in &found {
            assert_eq!(Area::Savestates, save.area);
            assert_eq!(GameId::Media("ULUS12345".to_string()), save.game);
        }
    }

    #[test]
    fn game_key_is_built_from_identity_never_from_title() {
        assert_eq!(
            "DuckStation SLUS-00067",
            GameId::Media("SLUS-00067".to_string()).game_key(App::DuckStation)
        );
    }

    #[test]
    fn roots_ignores_an_emulator_root_that_does_not_match_the_signature() {
        let install = FakeInstall::new().dir("memcards");
        let roots = vec![Root::Emulator(crate::resource::config::root::Emulator {
            path: install.root.clone(),
            app: None,
        })];

        assert_eq!(None, Roots::for_config(&roots).data_root(App::DuckStation));
    }

    #[test]
    fn roots_uses_a_configured_emulator_root() {
        let install = FakeInstall::portable();
        let roots = vec![Root::Emulator(crate::resource::config::root::Emulator {
            path: install.root.clone(),
            app: Some(App::DuckStation),
        })];

        assert_eq!(
            Some(&install.root),
            Roots::for_config(&roots).data_root(App::DuckStation)
        );
    }

    /// Duas instalações do mesmo emulador tornam o destino indeterminado, e o motor tem que
    /// admitir isso em vez de escolher.
    #[test]
    fn roots_refuses_to_choose_between_two_installs_of_the_same_emulator() {
        let one = FakeInstall::portable();
        let two = FakeInstall::installed();
        let roots = vec![
            Root::Emulator(crate::resource::config::root::Emulator {
                path: one.root.clone(),
                app: Some(App::DuckStation),
            }),
            Root::Emulator(crate::resource::config::root::Emulator {
                path: two.root.clone(),
                app: Some(App::DuckStation),
            }),
        ];

        let roots = Roots::for_config(&roots);
        assert_eq!(None, roots.data_root(App::DuckStation));
        assert_eq!(2, roots.candidates(App::DuckStation).len());
    }
}
