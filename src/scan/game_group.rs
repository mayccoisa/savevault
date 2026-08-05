//! De onde vem um jogo: a "pasta" em que ele aparece na lista.
//!
//! A lista mistura save de PC e de console, e com 140 jogos achar os do emulador é rolagem cega.
//! Agrupar exige responder "de onde vem este jogo", e essa resposta vem de **duas fontes
//! disjuntas**, nunca do nome do jogo:
//!
//! - **emulador**, do que o scan gravou ([`crate::scan::layout::BackupSemantics::emulator`]);
//! - **loja**, do mapa de instalações dos launchers ([`crate::scan::launchers::Launchers`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    resource::manifest::{Game, Source, Store},
    scan::emulator,
};

/// O grupo de um jogo na lista.
///
/// A ordem dos grupos na tela é a ordem desta declaração, via `Ord` derivado: emuladores, lojas,
/// personalizados e, por último, o balde. É de propósito que `Other` seja o último: ele é o que
/// sobra, e o que sobra não abre a tela.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GameGroup {
    Emulator(emulator::App),
    Store(Store),
    /// Jogo que só existe porque o usuário cadastrou à mão.
    Custom,
    /// Nenhuma origem pôde ser atribuída. Não é "jogo local": é "não sabemos", e chamar de local
    /// seria afirmar o que não se sabe.
    Other,
}

/// O mapa de lojas por jogo, derivado dos launchers uma vez por varredura.
pub type GameOrigins = HashMap<String, BTreeSet<Store>>;

/// Tudo o que a classificação precisa saber sobre um jogo, sem depender da GUI nem do `ScanInfo`.
///
/// Existe para a regra ser testável sozinha, e para o chamador ser obrigado a dizer de onde tirou
/// cada coisa em vez de a regra sair adivinhando.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameFacts<'a> {
    pub name: &'a str,
    /// O emulador que o scan gravou, quando é save de emulador.
    pub emulator: Option<emulator::App>,
    /// O jogo foi cadastrado à mão pelo usuário.
    pub customized: bool,
    /// O jogo existe no manifesto público. Cadastrado à mão **e** ausente do manifesto é o que
    /// caracteriza o jogo personalizado puro.
    pub in_manifest: bool,
}

impl<'a> GameFacts<'a> {
    /// Os fatos tirados do que está registrado no jogo, e não inferidos do nome.
    ///
    /// `sources` é dado gravado: um jogo cadastrado à mão traz [`Source::Custom`], e um que vem do
    /// manifesto público traz [`Source::Primary`]. Quem tem os dois é um jogo do manifesto que o
    /// usuário só ajustou, e continua sendo da loja dele.
    pub fn of(name: &'a str, game: &Game, emulator: Option<emulator::App>) -> Self {
        Self {
            name,
            emulator,
            customized: game.sources.contains(&Source::Custom),
            in_manifest: game.sources.contains(&Source::Primary),
        }
    }
}

impl GameGroup {
    /// A que grupo este jogo pertence.
    ///
    /// A ordem das perguntas não é arbitrária:
    ///
    /// 1. **emulador primeiro**, porque um save de emulador nunca pode cair numa loja;
    /// 2. personalizado puro, mesma distinção que a lista já faz para o selo de "custom";
    /// 3. a loja, quando os launchers acharam a instalação;
    /// 4. o balde.
    ///
    /// > **Nunca classifique pelo nome.** A chave de um jogo de emulador é
    /// > `"<Emulador> <identidade>"`, então casar por prefixo parece óbvio e põe `Eden Ring`, que
    /// > é jogo de PC, dentro do emulador Eden. É a mesma armadilha que já reprovou uma
    /// > implementação do agrupamento das pastas de backup e do filtro de origem.
    pub fn classify(facts: GameFacts, origins: &GameOrigins) -> Self {
        if let Some(app) = facts.emulator {
            return Self::Emulator(app);
        }

        if facts.customized && !facts.in_manifest {
            return Self::Custom;
        }

        match Self::primary_store(facts.name, origins) {
            Some(store) => Self::Store(store),
            None => Self::Other,
        }
    }

    /// **Um jogo, um grupo.** Achado em duas lojas (acontece quando o Heroic ou o Lutris apontam
    /// para uma biblioteca da Steam), fica na de menor índice em [`Store::ALL`], que é estável
    /// entre execuções. Repetir a linha nos dois grupos quebraria a contagem e o tamanho do
    /// cabeçalho, e faria o checkbox de um grupo mexer no outro.
    fn primary_store(name: &str, origins: &GameOrigins) -> Option<Store> {
        let found = origins.get(name)?;
        Store::ALL
            .iter()
            .copied()
            // A raiz de emulador também é uma `Store`, e não é loja nenhuma: o grupo de um save de
            // emulador vem do que o scan gravou, e já foi decidido antes de chegar aqui.
            .filter(|store| *store != Store::Emulator)
            .find(|store| found.contains(store))
    }
}

impl GameGroup {
    /// Nome do grupo para relatório de linha de comando. A tela usa a tradução, não isto.
    pub fn label(&self) -> String {
        match self {
            Self::Emulator(app) => app.name().to_string(),
            Self::Store(store) => format!("{store:?}"),
            Self::Custom => "Custom".to_string(),
            Self::Other => "Other".to_string(),
        }
    }
}

/// O que a atribuição de origem conseguiu dizer sobre os jogos desta máquina.
///
/// É instrumento de medição, e por isso o total por grupo importa mais que a lista: o número que
/// decide se agrupar vale a pena é a fatia que caiu em `Other`.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesReport {
    pub games: Vec<SourcesReportGame>,
    pub totals: BTreeMap<String, usize>,
    pub unattributed_percent: u32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesReportGame {
    pub game: String,
    pub group: String,
}

impl SourcesReport {
    pub fn new(found: Vec<(String, GameGroup)>) -> Self {
        let mut totals: BTreeMap<String, usize> = BTreeMap::new();
        for (_, group) in &found {
            *totals.entry(group.label()).or_default() += 1;
        }

        let unattributed = totals.get(&GameGroup::Other.label()).copied().unwrap_or_default();
        let unattributed_percent = (unattributed * 100).checked_div(found.len()).unwrap_or_default() as u32;

        Self {
            games: found
                .into_iter()
                .map(|(game, group)| SourcesReportGame {
                    game,
                    group: group.label(),
                })
                .collect(),
            totals,
            unattributed_percent,
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        for game in &self.games {
            out.push_str(&format!("{:<24}  {}\n", game.group, game.game));
        }

        out.push_str("\nTotals:\n");
        for (group, count) in &self.totals {
            out.push_str(&format!("  {group:<24}  {count}\n"));
        }
        out.push_str(&format!(
            "\n{} game(s), {}% with no source attributed.\n",
            self.games.len(),
            self.unattributed_percent
        ));

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velcro::{btree_set, hash_map};

    fn facts(name: &str) -> GameFacts<'_> {
        GameFacts {
            name,
            in_manifest: true,
            ..Default::default()
        }
    }

    /// A armadilha do nome, pela terceira vez nesta base: `Eden Ring` é jogo de PC, e a chave de
    /// um jogo de emulador é `"<Emulador> <identidade>"`.
    #[test]
    fn a_pc_game_named_like_an_emulator_is_not_grouped_under_it() {
        let origins: GameOrigins = hash_map! {
            "Eden Ring".to_string(): btree_set![Store::Steam],
        };

        assert_eq!(
            GameGroup::Store(Store::Steam),
            GameGroup::classify(facts("Eden Ring"), &origins)
        );
        assert_eq!(
            GameGroup::Emulator(emulator::App::Eden),
            GameGroup::classify(
                GameFacts {
                    emulator: Some(emulator::App::Eden),
                    ..facts("Eden 0100000000010000")
                },
                &origins,
            )
        );
    }

    /// O emulador vence a loja mesmo quando o mapa dos launchers tem palpite para o mesmo nome.
    #[test]
    fn the_emulator_wins_over_any_store() {
        let origins: GameOrigins = hash_map! {
            "Eden 0100000000010000".to_string(): btree_set![Store::Steam],
        };

        assert_eq!(
            GameGroup::Emulator(emulator::App::Eden),
            GameGroup::classify(
                GameFacts {
                    emulator: Some(emulator::App::Eden),
                    ..facts("Eden 0100000000010000")
                },
                &origins,
            )
        );
    }

    #[test]
    fn a_hand_made_game_is_its_own_group_only_when_it_is_not_in_the_manifest() {
        let origins = GameOrigins::default();

        assert_eq!(
            GameGroup::Custom,
            GameGroup::classify(
                GameFacts {
                    customized: true,
                    in_manifest: false,
                    ..facts("Meu jogo")
                },
                &origins,
            )
        );
        // Customizar um jogo que existe no manifesto é ajustar o que já existe, não criar outro.
        assert_eq!(
            GameGroup::Other,
            GameGroup::classify(
                GameFacts {
                    customized: true,
                    ..facts("Hades")
                },
                &origins,
            )
        );
    }

    #[test]
    fn a_game_found_in_two_stores_lands_in_exactly_one() {
        let origins: GameOrigins = hash_map! {
            "Hades".to_string(): btree_set![Store::Steam, Store::Heroic],
        };

        // Heroic vem antes de Steam em `Store::ALL`, e o critério é esse, não preferência.
        assert_eq!(
            GameGroup::Store(Store::Heroic),
            GameGroup::classify(facts("Hades"), &origins)
        );
    }

    /// A raiz de emulador é uma `Store`, e não é loja: sozinha, ela não faz grupo de loja.
    #[test]
    fn the_emulator_root_is_not_a_store_group() {
        let origins: GameOrigins = hash_map! {
            "Hades".to_string(): btree_set![Store::Emulator],
        };

        assert_eq!(GameGroup::Other, GameGroup::classify(facts("Hades"), &origins));
    }

    #[test]
    fn a_game_nobody_could_place_falls_into_the_bucket() {
        assert_eq!(
            GameGroup::Other,
            GameGroup::classify(facts("Hades"), &GameOrigins::default())
        );
    }

    /// O balde é sempre o último da tela, mesmo grande.
    #[test]
    fn the_bucket_sorts_last() {
        let mut groups = [
            GameGroup::Other,
            GameGroup::Custom,
            GameGroup::Store(Store::Steam),
            GameGroup::Emulator(emulator::App::Xenia),
        ];
        groups.sort();

        assert_eq!(GameGroup::Emulator(emulator::App::Xenia), groups[0]);
        assert_eq!(GameGroup::Other, groups[3]);
    }
}
