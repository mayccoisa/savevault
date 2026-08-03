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
#[derive(Clone, Copy, Debug)]
pub struct Signature {
    /// Todas estas precisam estar presentes.
    pub all_of: &'static [Marker],
    /// Ao menos uma destas precisa estar presente. Vazio significa "nada a exigir".
    pub any_of: &'static [Marker],
}

impl Signature {
    fn matches(&self, data_root: &StrictPath) -> bool {
        if !data_root.is_dir() {
            return false;
        }
        self.all_of.iter().all(|m| m.present_in(data_root))
            && (self.any_of.is_empty() || self.any_of.iter().any(|m| m.present_in(data_root)))
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
}

/// Uma área da pasta de dados.
#[derive(Clone, Copy, Debug)]
pub struct AreaSpec {
    pub area: Area,
    /// Subpasta relativa à pasta de dados.
    pub subdir: &'static str,
    /// Extensões dos arquivos que interessam, sem o ponto.
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
    /// Arquivo que, ao lado do executável, marca instalação portátil. Nesse caso a pasta de
    /// dados é a própria pasta do executável.
    pub portable_marker: Option<&'static str>,
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
    portable_marker: Some("portable.txt"),
    // `memcards` sozinho não distingue do PCSX2, daí a segunda marca.
    signature: Signature {
        all_of: &[Marker::Dir("memcards")],
        any_of: &[Marker::File("settings.ini"), Marker::File("portable.txt")],
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

impl App {
    pub const ALL: &'static [Self] = &[Self::DuckStation];

    pub fn profile(&self) -> &'static Profile {
        match self {
            Self::DuckStation => &DUCKSTATION,
        }
    }

    pub fn name(&self) -> &'static str {
        self.profile().name
    }

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
        let area_root = data_root.joined(spec.subdir);
        if !area_root.is_dir() {
            continue;
        }

        let Ok(entries) = area_root.read_dir() else {
            continue;
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
            found.extend(attribute(app, spec, &area_root, &file));
        }
    }

    found
}

fn has_extension(file: &StrictPath, extensions: &[&str]) -> bool {
    let rendered = file.render().to_lowercase();
    extensions
        .iter()
        .any(|extension| rendered.ends_with(&format!(".{}", extension.to_lowercase())))
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

    pub fn iter(&self) -> impl Iterator<Item = (App, &StrictPath)> {
        self.entries.iter().map(|(app, path)| (*app, path))
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

    #[test]
    fn rejects_a_folder_without_memcards() {
        let install = FakeInstall::new().file("settings.ini", b"[Main]\n");
        assert!(!App::DuckStation.matches_data_root(&install.root));
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
