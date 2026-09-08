//! What the backup folder actually holds, read from the backups themselves.
//!
//! This is the answer to "what do I have stored, and how old is it" — a question the app could not
//! answer before. The Logs screen lists *executions*, so a game that has not changed in months
//! produces no new run and disappears from it entirely. The Restore screen listed the games from
//! the last scan, out of the cache, which is not the same thing as what is in the vault and is
//! empty until you scan.
//!
//! Nothing here is recorded separately, for the same reason the log is not: a second record of the
//! same fact drifts from the first, and the backup itself is the one that has to be right.

use std::collections::BTreeMap;

use crate::{
    resource::config::Config,
    scan::layout::{Backup, BackupLayout, IndividualMappingFile},
};

/// What one backup did to a game's saves, measured against the backup before it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub when: chrono::DateTime<chrono::Local>,
    pub game: String,
    /// A full backup stands on its own; a differential one only carries what moved.
    pub full: bool,
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    /// Files the game has after this backup, not files this backup wrote.
    pub files: usize,
    pub bytes: u64,
    pub comment: Option<String>,
    /// The emulator this game's saves came from, when the backup is filed under one.
    ///
    /// Read from the shape of the folder, not guessed from the name: emulator backups live one
    /// level down, inside a folder named after the emulator.
    pub emulator: Option<String>,
}

/// One game in the vault: what is kept for it, and how old the newest copy is.
#[derive(Clone, Debug)]
pub struct Game {
    pub name: String,
    /// The newest backup. This is what the inventory is sorted by, oldest first, because the
    /// oldest is the one worth knowing about.
    pub last_backup: chrono::DateTime<chrono::Local>,
    pub backups: usize,
    /// Files and bytes as of the newest backup — the state of the game now, not a sum over every
    /// backup ever made, which would count the same save once per copy.
    pub files: usize,
    pub bytes: u64,
    pub emulator: Option<String>,
}

/// Walks the backup folder and rebuilds what it holds, newest backup first.
pub fn entries(config: &Config) -> Vec<Entry> {
    let layout = BackupLayout::new(config.restore.path.clone());
    let base = config.restore.path.interpret().ok();
    let mut entries = vec![];

    for (name, game_dir) in BackupLayout::load(&config.restore.path) {
        let Some(game) = layout.try_game_layout(&name) else {
            continue;
        };

        // An emulator game is filed one level down, under the emulator's own folder. Comparing the
        // parent with the base is what distinguishes that from a plain game at the top level.
        let emulator = game_dir.parent().and_then(|parent| {
            let parent_is_base = base
                .as_ref()
                .and_then(|base| parent.interpret().ok().map(|p| p == *base))
                .unwrap_or(false);
            (!parent_is_base).then(|| parent.leaf()).flatten()
        });

        // The effective set of files the game has, carried forward from one backup to the next. A
        // differential backup only names what moved, so the rest has to be inherited.
        let mut state: BTreeMap<String, IndividualMappingFile> = BTreeMap::new();

        for backup in game.restorable_backups_flattened() {
            let (mut added, mut changed, mut removed) = (0, 0, 0);

            match &backup {
                Backup::Full(full) => {
                    for (path, file) in &full.files {
                        match state.get(path) {
                            None => added += 1,
                            Some(previous) if previous.hash != file.hash => changed += 1,
                            Some(_) => {}
                        }
                    }
                    removed = state.keys().filter(|path| !full.files.contains_key(*path)).count();
                    state = full.files.clone();
                }
                Backup::Differential(diff) => {
                    for (path, file) in &diff.files {
                        match file {
                            Some(file) => {
                                match state.get(path) {
                                    None => added += 1,
                                    Some(previous) if previous.hash != file.hash => changed += 1,
                                    Some(_) => {}
                                }
                                state.insert(path.clone(), file.clone());
                            }
                            None => {
                                if state.remove(path).is_some() {
                                    removed += 1;
                                }
                            }
                        }
                    }
                }
            }

            entries.push(Entry {
                when: backup.when_local(),
                game: name.clone(),
                full: matches!(backup, Backup::Full(_)),
                added,
                changed,
                removed,
                files: state.len(),
                bytes: state.values().map(|file| file.size).sum(),
                comment: backup.comment().cloned(),
                emulator: emulator.clone(),
            });
        }
    }

    // Newest first: the log is read to find out what just happened.
    entries.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.game.cmp(&b.game)));

    entries
}

/// The inventory: one row per game, oldest backup first.
///
/// Oldest first is the whole point of the ordering. A list sorted by name answers "is X in there";
/// sorted by age it answers "what have I not backed up in a long time", which is the question a
/// backup tool exists to keep you from having to ask yourself.
pub fn games(config: &Config) -> Vec<Game> {
    let mut by_game: BTreeMap<String, Game> = BTreeMap::new();

    // `entries` comes newest first, so the first one seen for a game is its newest.
    for entry in entries(config) {
        match by_game.get_mut(&entry.game) {
            Some(game) => game.backups += 1,
            None => {
                by_game.insert(
                    entry.game.clone(),
                    Game {
                        name: entry.game,
                        last_backup: entry.when,
                        backups: 1,
                        files: entry.files,
                        bytes: entry.bytes,
                        emulator: entry.emulator,
                    },
                );
            }
        }
    }

    let mut games: Vec<Game> = by_game.into_values().collect();
    games.sort_by(|a, b| a.last_backup.cmp(&b.last_backup).then_with(|| a.name.cmp(&b.name)));
    games
}
