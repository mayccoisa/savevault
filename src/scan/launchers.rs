mod generic;
pub mod heroic;
mod legendary;
mod lutris;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    prelude::StrictPath,
    resource::{
        config::Root,
        manifest::{Manifest, Os, Store},
    },
    scan::TitleFinder,
};

#[derive(Clone, Default, Debug)]
pub struct Launchers {
    games: HashMap<Root, HashMap<String, HashSet<LauncherGame>>>,
    empty: HashSet<LauncherGame>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LauncherGame {
    pub install_dir: Option<StrictPath>,
    pub prefix: Option<StrictPath>,
    pub platform: Option<Os>,
}

impl LauncherGame {
    pub fn is_empty(&self) -> bool {
        self.install_dir.is_none() && self.prefix.is_none() && self.platform.is_none()
    }

    pub fn replace_in_paths(&self, old: &StrictPath, new: &StrictPath) -> Self {
        Self {
            install_dir: self.install_dir.as_ref().map(|x| x.replace(old, new)),
            prefix: self.prefix.as_ref().map(|x| x.replace(old, new)),
            platform: self.platform,
        }
    }
}

impl Launchers {
    pub fn get_game(&self, root: &Root, game: &str) -> impl Iterator<Item = &LauncherGame> {
        self.games
            .get(root)
            .and_then(|root| root.get(game))
            .unwrap_or(&self.empty)
            .iter()
    }

    /// Quais lojas instalaram cada jogo, na visão dos launchers.
    ///
    /// Este mapa é **a única resposta que existe** para "de que loja é este jogo": o scan sabe
    /// qual raiz gerou cada arquivo enquanto roda, e descarta isso no fim. Aqui ele é derivado uma
    /// vez por varredura, em vez de uma consulta por jogo.
    ///
    /// > **A cobertura não é total, e isso é do desenho do Ludusavi, não deste método.** Para as
    /// > lojas sem metadado próprio, a instalação é descoberta por **nome de pasta**, com
    /// > casamento aproximado contra o manifesto (ver `launchers::generic`). Jogo achado por
    /// > registro, por prefixo do Wine ou por caminho fora de qualquer raiz não aparece aqui, e
    /// > quem consome precisa tratar a ausência como "não sei", nunca como "é local".
    pub fn all_game_stores(&self) -> HashMap<String, BTreeSet<Store>> {
        let mut stores: HashMap<String, BTreeSet<Store>> = HashMap::new();

        for (root, games) in &self.games {
            for game in games.keys() {
                stores.entry(game.clone()).or_default().insert(root.store());
            }
        }

        stores
    }

    pub fn scan(
        roots: &[Root],
        manifest: &Manifest,
        subjects: &[String],
        title_finder: &TitleFinder,
        legendary: Option<StrictPath>,
    ) -> Self {
        let mut instance = Self::default();

        for root in roots {
            if root.is_game_specific() {
                log::trace!("Skipping launcher info for game-specific root: {:?}", root);
                continue;
            }

            log::debug!("Scanning launcher info: {:?}", &root);
            let mut found = match root {
                Root::Heroic(root) => heroic::scan(root, title_finder, legendary.as_ref()),
                Root::Legendary(root) => legendary::scan(root, title_finder),
                Root::Lutris(root) => lutris::scan(root, title_finder),
                // An emulator's data folder has no games to discover by folder name. Letting the
                // generic scanner loose in there would fuzzy-match `bios`, `covers` and `cache`
                // against real game titles and invent games that do not exist.
                Root::Emulator(_) => Default::default(),
                _ => generic::scan(root, manifest, subjects),
            };
            found.retain(|_k, v| {
                v.retain(|x| !x.is_empty());
                !v.is_empty()
            });
            log::debug!("launcher games found ({:?}): {:#?}", &root, &found);
            if !found.is_empty() {
                instance.games.entry(root.clone()).or_default().extend(found);
            }
        }

        instance
    }

    #[cfg(test)]
    pub fn scan_dirs(roots: &[Root], manifest: &Manifest, subjects: &[String]) -> Self {
        Self::scan(roots, manifest, subjects, &TitleFinder::default(), None)
    }
}
