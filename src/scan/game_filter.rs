use crate::{
    lang::TRANSLATOR,
    resource::manifest,
    scan::{Duplication, ScanInfo, emulator},
};

use super::ScanChange;

#[derive(Clone, Debug)]
pub enum Event {
    Toggled,
    ToggledFilter { filter: FilterKind, enabled: bool },
    EditedGameName(String),
    Reset,
    EditedFilterUniqueness(Uniqueness),
    EditedFilterCompleteness(Completeness),
    EditedFilterEnablement(Enablement),
    EditedFilterChange(Change),
    EditedFilterManifest(Manifest),
    EditedFilterOrigin(Origin),
}

#[derive(Clone, Copy, Debug)]
pub enum FilterKind {
    Uniqueness,
    Completeness,
    Enablement,
    Change,
    Manifest,
    Origin,
}

/// De onde vem o save: do PC ou de um emulador, e de qual.
///
/// A lista mistura as duas coisas, e quem quer conferir o console não quer rolar por centenas de
/// jogos de PC para achar meia dúzia de emulador.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Origin {
    #[default]
    Pc,
    Emulator(emulator::App),
}

/// Ordem de tela: os emuladores primeiro, o PC por último.
///
/// É a mesma regra que [`crate::scan::game_group::GameGroup`] já usa, e pelo mesmo motivo: o PC é
/// o balde grande. Num backup real são 130 jogos de PC contra meia dúzia de emulador, e pôr o
/// balde na frente obrigaria a rolar a lista inteira para chegar ao punhado que se veio conferir.
impl Ord for Origin {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let rank = |origin: &Self| match origin {
            Self::Emulator(app) => (0, Some(*app)),
            Self::Pc => (1, None),
        };
        rank(self).cmp(&rank(other))
    }
}

impl PartialOrd for Origin {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Origin {
    /// De onde vem este jogo, segundo o que o scan gravou.
    ///
    /// Mesma regra do [`Self::qualifies`], do outro lado: lá se pergunta "este jogo é deste
    /// grupo?", aqui "de que grupo é este jogo?".
    pub fn of(scan: &ScanInfo) -> Self {
        match scan.semantics.emulator() {
            Some(app) => Self::Emulator(app),
            None => Self::Pc,
        }
    }

    /// O PC primeiro, depois cada emulador conhecido.
    pub fn all() -> Vec<Self> {
        std::iter::once(Self::Pc)
            .chain(emulator::App::ALL.iter().copied().map(Self::Emulator))
            .collect()
    }

    /// **Quem decide é o que o scan gravou, nunca o nome do jogo.** A chave é
    /// `"<Emulador> <identidade>"`, então filtrar por nome parece óbvio e põe um jogo de PC
    /// chamado `Eden Ring` dentro do Eden. É a mesma armadilha do agrupamento das pastas.
    pub fn qualifies(&self, scan: &ScanInfo) -> bool {
        match self {
            Self::Pc => scan.semantics.emulator().is_none(),
            Self::Emulator(app) => scan.semantics.emulator() == Some(*app),
        }
    }
}

impl ToString for Origin {
    fn to_string(&self) -> String {
        match self {
            Self::Pc => TRANSLATOR.filter_origin_pc(),
            Self::Emulator(app) => app.name().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Uniqueness {
    Unique,
    #[default]
    Duplicate,
}

impl Uniqueness {
    pub const ALL: &'static [Self] = &[Self::Unique, Self::Duplicate];

    pub fn qualifies(&self, duplicated: Duplication) -> bool {
        match self {
            Self::Unique => duplicated.unique(),
            Self::Duplicate => !duplicated.unique(),
        }
    }
}

impl ToString for Uniqueness {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_uniqueness(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Completeness {
    Complete,
    #[default]
    Partial,
}

impl Completeness {
    pub const ALL: &'static [Self] = &[Self::Complete, Self::Partial];

    pub fn qualifies(&self, scan: &ScanInfo) -> bool {
        match self {
            Self::Complete => !scan.any_ignored(),
            Self::Partial => scan.any_ignored(),
        }
    }
}

impl ToString for Completeness {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_completeness(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Enablement {
    Enabled,
    #[default]
    Disabled,
}

impl Enablement {
    pub const ALL: &'static [Self] = &[Self::Enabled, Self::Disabled];

    pub fn qualifies(&self, enabled: bool) -> bool {
        match self {
            Self::Enabled => enabled,
            Self::Disabled => !enabled,
        }
    }
}

impl ToString for Enablement {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_enablement(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Change {
    New,
    Updated,
    Unchanged,
    #[default]
    Unscanned,
}

impl ToString for Change {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_freshness(*self)
    }
}

impl Change {
    pub const ALL: &'static [Self] = &[Self::New, Self::Updated, Self::Unchanged, Self::Unscanned];

    pub fn qualifies(&self, scan: &ScanInfo) -> bool {
        match self {
            Change::New => scan.overall_change() == ScanChange::New,
            Change::Updated => scan.overall_change() == ScanChange::Different,
            Change::Unchanged => scan.overall_change() == ScanChange::Same,
            Change::Unscanned => scan.overall_change() == ScanChange::Unknown,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Manifest {
    source: manifest::Source,
}

impl Manifest {
    pub fn new(source: manifest::Source) -> Self {
        Self { source }
    }
}

impl ToString for Manifest {
    fn to_string(&self) -> String {
        match &self.source {
            manifest::Source::Primary => TRANSLATOR.primary_manifest_label(),
            manifest::Source::Custom => TRANSLATOR.custom_games_label(),
            manifest::Source::Secondary(id) => id.to_string(),
        }
    }
}

impl Manifest {
    pub fn qualifies(&self, game: Option<&manifest::Game>, customized: bool) -> bool {
        game.map(|game| game.sources.contains(&self.source)).unwrap_or_default()
            || (self.source == manifest::Source::Custom && customized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::layout::{BackupSemantics, DirectorySemantics, SemanticDirKind};
    use velcro::btree_map;

    fn scan_of(name: &str, app: Option<emulator::App>) -> ScanInfo {
        ScanInfo {
            game_name: name.to_string(),
            semantics: match app {
                None => BackupSemantics::default(),
                Some(app) => BackupSemantics {
                    directories: btree_map! {
                        "C:/emu/saves".to_string(): DirectorySemantics {
                            kind: SemanticDirKind::Emulator { app, area: emulator::Area::Saves },
                        },
                    },
                },
            },
            ..Default::default()
        }
    }

    /// A armadilha do nome, agora no filtro: a chave de um jogo de emulador é
    /// `"<Emulador> <identidade>"`, então filtrar por nome poria `Eden Ring`, que é jogo de PC,
    /// dentro do emulador Eden. Quem decide é o que o scan gravou.
    #[test]
    fn a_pc_game_named_like_an_emulator_is_not_filtered_as_one() {
        let pc = scan_of("Eden Ring", None);
        let switch = scan_of("Eden 0100000000010000", Some(emulator::App::Eden));

        assert!(Origin::Pc.qualifies(&pc));
        assert!(!Origin::Emulator(emulator::App::Eden).qualifies(&pc));

        assert!(!Origin::Pc.qualifies(&switch));
        assert!(Origin::Emulator(emulator::App::Eden).qualifies(&switch));
        // E um emulador não pega o save do outro.
        assert!(!Origin::Emulator(emulator::App::Sudachi).qualifies(&switch));
    }

    /// A mesma armadilha do nome, agora no agrupamento da tela.
    #[test]
    fn the_group_of_a_game_comes_from_the_scan_not_from_its_name() {
        assert_eq!(Origin::Pc, Origin::of(&scan_of("Eden Ring", None)));
        assert_eq!(
            Origin::Emulator(emulator::App::Eden),
            Origin::of(&scan_of("Eden 0100000000010000", Some(emulator::App::Eden)))
        );
    }

    /// O PC é o balde grande e vai por último, senão os poucos jogos de emulador ficariam depois
    /// de uma lista de centenas.
    #[test]
    fn the_pc_group_sorts_last() {
        let mut groups = [
            Origin::Pc,
            Origin::Emulator(emulator::App::Ppsspp),
            Origin::Emulator(emulator::App::DuckStation),
        ];
        groups.sort();

        assert_eq!(Some(&Origin::Pc), groups.last());
    }

    #[test]
    fn every_known_emulator_is_offered_next_to_the_pc() {
        let all = Origin::all();
        assert_eq!(Some(&Origin::Pc), all.first());
        assert_eq!(emulator::App::ALL.len() + 1, all.len());
    }
}
