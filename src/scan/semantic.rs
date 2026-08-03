use crate::{
    api::{Config, StrictPath},
    scan::{
        emulator,
        layout::{BackupSemantics, SemanticDirKind},
    },
};

pub use self::{convert::KnownFolders, prefix::Prefix};

mod convert;
mod prefix;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Base {
    WinHome,
    WinDocuments,
    WinAppData,
    WinLocalAppData,
    WinLocalAppDataLow,
    WinSavedGames,
    WinPublic,
    WinProgramData,
    WinDir,
    WinDrive(char),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
    pub base: Base,
    pub tail: String,
}

/// Context for resolving a restore target on the current machine.
///
/// Despite the name, this carries every source of re-anchoring, not only Wine. The name is kept
/// so that the eight signatures threading it through the restore path stay identical to upstream,
/// which is what keeps merging from upstream cheap. Use [`RestoreContext`] in new code.
pub struct Wine {
    /// First valid `wine_prefix` from the matching custom game.
    pub preferred_prefix: Option<Prefix>,
    /// Current Windows known folders, only populated on Windows.
    pub known_folders: Option<KnownFolders>,
    /// Emulator data folders present on this machine.
    pub emulators: emulator::Roots,
}

/// What [`Wine`] actually is: the whole restore-target resolution context.
pub type RestoreContext = Wine;

impl Wine {
    /// Build a context from the current game's config and system state.
    /// Returns None when there is nothing on this machine to re-anchor onto.
    pub fn for_game(game_name: &str, config: &Config) -> Option<Self> {
        // Emulator roots are independent of the Wine setting: turning off Wine redirects must not
        // also turn off emulator restore.
        let emulators = emulator::Roots::for_config(&config.roots);

        let (preferred_prefix, known_folders) = if config.scan.redirect_wine {
            // Find the first valid wine_prefix from a matching custom game.
            let preferred_prefix = config
                .custom_games
                .iter()
                .find(|cg| cg.name == game_name)
                .and_then(|cg| {
                    cg.wine_prefix
                        .iter()
                        .filter(|wp| !wp.trim().is_empty())
                        .find_map(|wp| Prefix::validated(&StrictPath::new(wp)))
                });

            // On Windows, populate known_folders so that Wine→Windows restore can
            // convert semantic paths to physical paths.
            (preferred_prefix, KnownFolders::windows())
        } else {
            (None, None)
        };

        if preferred_prefix.is_some() || known_folders.is_some() || !emulators.is_empty() {
            Some(Self {
                preferred_prefix,
                known_folders,
                emulators,
            })
        } else {
            None
        }
    }
}

/// Generate a redirect for restoring a file from a backup with Wine semantics.
///
/// Linux/Wine backup → Windows restore: convert Wine path to Windows known-folder path.
/// Windows backup → Linux/Wine restore: convert Windows path to Wine prefix path.
pub fn generate_restore_redirect(
    stored_path: &StrictPath,
    semantics: &BackupSemantics,
    context: &Wine,
) -> Option<StrictPath> {
    // Emulator first: it is an explicit, recorded anchor, so it must win over the Windows
    // special-folder heuristic further down, which guesses.
    if let EmulatorTarget::Redirected(target) = emulator_restore_target(stored_path, semantics, &context.emulators) {
        return Some(target);
    }

    let stored_raw = stored_path.raw();

    let wine_match = semantics
        .directories
        .iter()
        .find(|(dir, semantics)| stored_raw.starts_with(dir.as_str()) && semantics.kind == SemanticDirKind::Wine);

    if let Some((prefix_path, _)) = wine_match {
        // Linux/Wine backup → Windows restore: preferred_prefix is None, known_folders is Some.
        if let Some(kf) = &context.known_folders
            && context.preferred_prefix.is_none()
        {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            let semantic = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)?;
            return materialize_to_windows(&semantic, kf);
        }

        // Wine backup → Wine restore (same or different prefix):
        // Use semantic conversion to handle username changes correctly.
        if let Some(prefix) = &context.preferred_prefix {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            if let Some(semantic) = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)
                .and_then(|s| materialize_to_wine(&s, prefix))
            {
                return Some(semantic);
            }
        }
    }

    // Windows backup → Linux/Wine restore: detect Windows special folders heuristically.
    // This handles the case where the stored path is a Windows path (e.g., C:/Users/...)
    // and we're restoring into a Wine prefix.
    if let Some(prefix) = &context.preferred_prefix
        && let Some(semantic) = convert::windows_physical_to_semantic(stored_path, &KnownFolders::default())
        && let Some(target) = materialize_to_wine(&semantic, prefix)
    {
        return Some(target);
    }

    None
}

/// What the emulator engine concluded about where a backed-up file should go.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmulatorTarget {
    /// Not an emulator file, or the emulator lives in the same place it did at backup time.
    /// Restoring to the path stored in the backup is correct.
    Settled,
    /// Re-anchored onto this machine.
    Redirected(StrictPath),
    /// This IS an emulator file, and the destination could not be determined.
    ///
    /// This case must never fall back to the absolute path from the backup. On a different
    /// machine that path is somebody else's user folder: the write would succeed, create the
    /// whole tree, and leave the user believing the save was restored while the emulator reads
    /// nothing. Refusing is the only honest outcome.
    Unresolved(Unresolved),
}

/// Why an emulator file's destination could not be determined.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Unresolved {
    /// No install of this emulator was found on this machine.
    EmulatorMissing(emulator::App),
    /// Two or more installs were found, so the destination is ambiguous. Choosing one would be
    /// guessing with the user's save data.
    EmulatorAmbiguous(emulator::App),
}

impl Unresolved {
    pub fn app(&self) -> emulator::App {
        match self {
            Self::EmulatorMissing(app) | Self::EmulatorAmbiguous(app) => *app,
        }
    }
}

/// Re-anchor a file from the emulator data folder recorded in the backup onto the one on this
/// machine.
///
/// This is the whole point of the product: the tail is reapplied literally, so
/// `memcards/SLUS-00067_1.mcd` stays `memcards/SLUS-00067_1.mcd` and only the root changes. The
/// user points at the emulator, or does not even have to, and the file lands where the emulator
/// reads it.
pub fn emulator_restore_target(
    stored_path: &StrictPath,
    semantics: &BackupSemantics,
    emulators: &emulator::Roots,
) -> EmulatorTarget {
    for (recorded, dir) in &semantics.directories {
        let SemanticDirKind::Emulator { app, area } = dir.kind else {
            continue;
        };

        let recorded_root = StrictPath::new(recorded.clone());
        // Tail comparison instead of `starts_with` on the raw string, for two reasons: on Windows
        // the case of the stored path and of the current one do differ in practice
        // (`AppData\Local` against `appdata\local`), and `starts_with` would also match a
        // sibling folder whose name merely begins the same, like `DuckStation2`.
        // Containment comes from this call: `StrictPath` resolves `..` before comparing, so a
        // recorded path that walks out of the area simply is not under it, and no tail comes
        // back. That matters because the recorded path is read off disk and is not necessarily
        // one we wrote. See `refuses_to_re_anchor_a_path_that_escapes_its_area`.
        let Some(tail) = stored_path.case_insensitive_tail_for(&recorded_root) else {
            continue;
        };

        let Some(subdir) = area_subdir(app, area) else {
            return EmulatorTarget::Unresolved(Unresolved::EmulatorMissing(app));
        };

        let Some(current_root) = emulators.data_root(app) else {
            return EmulatorTarget::Unresolved(if emulators.candidates(app).len() > 1 {
                Unresolved::EmulatorAmbiguous(app)
            } else {
                Unresolved::EmulatorMissing(app)
            });
        };

        let current_area = current_root.joined(subdir);
        if current_area.equivalent(&recorded_root) {
            // Same place: the path in the backup is already right.
            return EmulatorTarget::Settled;
        }

        return EmulatorTarget::Redirected(current_area.joined(tail.join("/")));
    }

    EmulatorTarget::Settled
}

fn area_subdir(app: emulator::App, area: emulator::Area) -> Option<&'static str> {
    app.profile()
        .areas
        .iter()
        .find(|spec| spec.area == area)
        .map(|spec| spec.subdir)
}

/// Materialize a semantic path to a Windows physical path using known folders.
fn materialize_to_windows(semantic: &Path, known_folders: &KnownFolders) -> Option<StrictPath> {
    let base_path = match &semantic.base {
        Base::WinHome => known_folders.user_profile.as_deref()?,
        Base::WinDocuments => known_folders.documents.as_deref()?,
        Base::WinAppData => known_folders.app_data.as_deref()?,
        Base::WinLocalAppData => known_folders.local_app_data.as_deref()?,
        Base::WinLocalAppDataLow => known_folders.local_low_app_data.as_deref()?,
        Base::WinSavedGames => known_folders.saved_games.as_deref()?,
        Base::WinPublic => known_folders.public.as_deref()?,
        Base::WinProgramData => known_folders.program_data.as_deref()?,
        Base::WinDir => known_folders.windows.as_deref()?,
        Base::WinDrive(_) => return None,
    };

    let path = format!("{}/{}", base_path.trim_end_matches('/'), semantic.tail);
    Some(StrictPath::new(path))
}

/// Materialize a semantic path into a Wine prefix path.
/// Maps semantic bases to their Wine directory equivalents under `drive_c/`.
fn materialize_to_wine(semantic: &Path, prefix: &Prefix) -> Option<StrictPath> {
    let base_path = match &semantic.base {
        Base::WinDocuments => format!("drive_c/users/{}/Documents", prefix.wine_user),
        Base::WinAppData => format!("drive_c/users/{}/AppData/Roaming", prefix.wine_user),
        Base::WinLocalAppData => format!("drive_c/users/{}/AppData/Local", prefix.wine_user),
        Base::WinLocalAppDataLow => format!("drive_c/users/{}/AppData/LocalLow", prefix.wine_user),
        Base::WinSavedGames => format!("drive_c/users/{}/Saved Games", prefix.wine_user),
        Base::WinPublic => "drive_c/users/Public".to_string(),
        Base::WinProgramData => "drive_c/ProgramData".to_string(),
        Base::WinDir => "drive_c/Windows".to_string(),
        Base::WinHome => format!("drive_c/users/{}", prefix.wine_user),
        Base::WinDrive(c) => {
            let drive = prefix.path.joined(format!("drive_{c}"));
            if *c != 'c' && !drive.is_dir() {
                return None;
            }
            format!("drive_{}", c)
        }
    };

    let path = format!(
        "{}/{}/{}",
        prefix.path.raw().trim_end_matches('/'),
        base_path,
        semantic.tail
    );
    Some(StrictPath::new(path))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::btree_map;

    use super::*;
    use crate::scan::{
        emulator::{App, Area},
        layout::DirectorySemantics,
    };

    const RECORDED: &str = "C:/Users/mayco/AppData/Local/DuckStation/memcards";
    const STORED: &str = "C:/Users/mayco/AppData/Local/DuckStation/memcards/SLUS-00067_1.mcd";

    fn semantics_for(recorded: &str, area: Area) -> BackupSemantics {
        BackupSemantics {
            directories: btree_map! {
                recorded.to_string(): DirectorySemantics {
                    kind: SemanticDirKind::Emulator { app: App::DuckStation, area },
                },
            },
        }
    }

    fn roots_at(path: &str) -> emulator::Roots {
        emulator::Roots::for_test(vec![(App::DuckStation, StrictPath::new(path.to_string()))])
    }

    /// The differentiator: the emulator moved, and the file follows it.
    #[test]
    fn re_anchors_onto_the_current_data_folder() {
        let target = emulator_restore_target(
            &StrictPath::new(STORED.to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(
            EmulatorTarget::Redirected(StrictPath::new(
                "D:/Emus/DuckStation/memcards/SLUS-00067_1.mcd".to_string()
            )),
            target
        );
    }

    /// Same place, so the path in the backup is already right and there is nothing to say.
    #[test]
    fn says_nothing_when_the_data_folder_did_not_move() {
        let target = emulator_restore_target(
            &StrictPath::new(STORED.to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &roots_at("C:/Users/mayco/AppData/Local/DuckStation"),
        );

        assert_eq!(EmulatorTarget::Settled, target);
    }

    /// On Windows the case of a recorded path and of the live one do differ in practice.
    #[test]
    fn matches_the_recorded_folder_ignoring_case() {
        let target = emulator_restore_target(
            &StrictPath::new("c:/users/mayco/appdata/local/duckstation/memcards/SLUS-00067_1.mcd".to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(
            EmulatorTarget::Redirected(StrictPath::new(
                "D:/Emus/DuckStation/memcards/SLUS-00067_1.mcd".to_string()
            )),
            target
        );
    }

    /// A sibling folder whose name merely starts the same is a different folder. This is why the
    /// match is by path tail and not by string prefix.
    #[test]
    fn does_not_match_a_sibling_folder_with_a_longer_name() {
        let target = emulator_restore_target(
            &StrictPath::new("C:/Users/mayco/AppData/Local/DuckStation2/memcards/SLUS-00067_1.mcd".to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(EmulatorTarget::Settled, target);
    }

    /// The case that must never silently fall back to the absolute path from the backup.
    #[test]
    fn refuses_when_the_emulator_is_not_installed() {
        let target = emulator_restore_target(
            &StrictPath::new(STORED.to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &emulator::Roots::default(),
        );

        assert_eq!(
            EmulatorTarget::Unresolved(Unresolved::EmulatorMissing(App::DuckStation)),
            target
        );
    }

    /// Two installs mean the destination is undetermined, and picking one would be guessing with
    /// the user's save data.
    #[test]
    fn refuses_when_two_installs_are_ambiguous() {
        let roots = emulator::Roots::for_test(vec![
            (App::DuckStation, StrictPath::new("D:/Emus/DuckStation".to_string())),
            (
                App::DuckStation,
                StrictPath::new("C:/Users/mayco/Documents/DuckStation".to_string()),
            ),
        ]);

        let target = emulator_restore_target(
            &StrictPath::new(STORED.to_string()),
            &semantics_for(RECORDED, Area::Memcards),
            &roots,
        );

        assert_eq!(
            EmulatorTarget::Unresolved(Unresolved::EmulatorAmbiguous(App::DuckStation)),
            target
        );
    }

    /// Containment. The recorded path is read off disk and is not necessarily one we wrote, so a
    /// path that walks out of its own area must never be re-anchored into the emulator folder on
    /// this machine. It is not, because `StrictPath` resolves `..` before comparing, which leaves
    /// the path outside the recorded area and produces no tail to reapply.
    #[test]
    fn refuses_to_re_anchor_a_path_that_escapes_its_area() {
        let target = emulator_restore_target(
            &StrictPath::new(format!("{RECORDED}/../../../Windows/System32/evil.dll")),
            &semantics_for(RECORDED, Area::Memcards),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(
            EmulatorTarget::Settled,
            target,
            "a traversal path must not be re-anchored into the current emulator folder"
        );
    }

    /// A file with no emulator semantics is none of this engine's business.
    #[test]
    fn says_nothing_about_a_regular_pc_game_file() {
        let target = emulator_restore_target(
            &StrictPath::new("C:/Users/mayco/Documents/My Game/save.dat".to_string()),
            &BackupSemantics::default(),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(EmulatorTarget::Settled, target);
    }

    /// Each area is re-anchored onto its own subfolder, which is what will later let one emulator
    /// keep save data and trophies apart.
    #[test]
    fn re_anchors_each_area_onto_its_own_subfolder() {
        let recorded = "C:/Users/mayco/AppData/Local/DuckStation/savestates";
        let target = emulator_restore_target(
            &StrictPath::new(format!("{recorded}/SLUS-00067_1.sav")),
            &semantics_for(recorded, Area::Savestates),
            &roots_at("D:/Emus/DuckStation"),
        );

        assert_eq!(
            EmulatorTarget::Redirected(StrictPath::new(
                "D:/Emus/DuckStation/savestates/SLUS-00067_1.sav".to_string()
            )),
            target
        );
    }

    /// Wine has to keep working with an emulator context present.
    #[test]
    fn wine_redirects_still_work_alongside_emulators() {
        let semantics = BackupSemantics {
            directories: btree_map! {
                "/home/user/prefix".to_string(): DirectorySemantics { kind: SemanticDirKind::Wine },
            },
        };
        let context = Wine {
            preferred_prefix: None,
            known_folders: Some(KnownFolders {
                documents: Some("C:/Users/Alice/Documents".to_string()),
                ..Default::default()
            }),
            emulators: roots_at("D:/Emus/DuckStation"),
        };

        let result = generate_restore_redirect(
            &StrictPath::new("/home/user/prefix/drive_c/users/wineuser/Documents/game/save.dat".to_string()),
            &semantics,
            &context,
        );

        assert!(result.is_some(), "wine redirect should still be produced");
        assert!(result.unwrap().raw().contains("C:/Users/Alice/Documents"));
    }
}
