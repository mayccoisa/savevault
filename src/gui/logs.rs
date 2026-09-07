//! The system log: every backup a game has, and what each one changed.
//!
//! This reads what is already written in each game's `mapping.yaml`. Nothing here is recorded
//! separately, on purpose: a second record of the same fact drifts from the first, and the
//! backup itself is the one that has to be right.

use std::collections::BTreeMap;

use crate::{
    lang::TRANSLATOR,
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
}

#[derive(Clone, Debug, Default)]
pub struct Logs {
    pub entries: Vec<Entry>,
    /// Set once the backup folder has been read, so an empty vector can say "nothing here yet"
    /// instead of "not looked yet". They are different answers and deserve different screens.
    pub loaded: bool,
}

impl Logs {
    /// Walks the backup folder and rebuilds the log from the backups themselves.
    pub fn load(config: &Config) -> Self {
        let layout = BackupLayout::new(config.restore.path.clone());
        let mut entries = vec![];

        for name in BackupLayout::load(&config.restore.path).keys() {
            let Some(game) = layout.try_game_layout(name) else {
                continue;
            };

            // The effective set of files the game has, carried forward from one backup to the
            // next. A differential backup only names what moved, so the rest has to be inherited.
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
                });
            }
        }

        // Newest first: the log is read to find out what just happened.
        entries.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.game.cmp(&b.game)));

        Self { entries, loaded: true }
    }
}

impl Entry {
    /// One phrase for the whole change, so the column reads instead of being decoded.
    pub fn change(&self) -> String {
        if self.added == 0 && self.changed == 0 && self.removed == 0 {
            return TRANSLATOR.logs_no_change();
        }

        let mut parts = vec![];
        if self.added > 0 {
            parts.push(TRANSLATOR.logs_added(self.added));
        }
        if self.changed > 0 {
            parts.push(TRANSLATOR.logs_changed(self.changed));
        }
        if self.removed > 0 {
            parts.push(TRANSLATOR.logs_removed(self.removed));
        }
        parts.join(" · ")
    }
}

impl Logs {
    pub fn view(&self) -> crate::gui::widget::Element<'_> {
        use crate::gui::{
            style,
            widget::{Column, Container, Element, Row, Scrollable, text},
        };
        use iced::{Alignment, Length};

        // One place decides the shape of the table, so the header can never drift from the rows.
        fn row<'a>(cells: Vec<(Element<'a>, u16)>) -> Row<'a> {
            let mut row = Row::new().spacing(12).align_y(Alignment::Center).padding([8, 14]);
            for (cell, portion) in cells {
                row = row.push(Container::new(cell).width(Length::FillPortion(portion)));
            }
            row
        }

        if !self.loaded || self.entries.is_empty() {
            return Container::new(text(TRANSLATOR.logs_empty()).size(14))
                .padding(40)
                .width(Length::Fill)
                .into();
        }

        let header = Container::new(row(vec![
            (text(TRANSLATOR.logs_column_when()).size(12).into(), 3),
            (text(TRANSLATOR.logs_column_game()).size(12).into(), 5),
            (text(TRANSLATOR.logs_column_backup()).size(12).into(), 2),
            (text(TRANSLATOR.logs_column_change()).size(12).into(), 5),
            (text(TRANSLATOR.logs_column_files()).size(12).into(), 2),
            (text(TRANSLATOR.logs_column_size()).size(12).into(), 2),
        ]))
        .class(style::Container::TableHeader);

        let mut body = Column::new();
        for entry in &self.entries {
            body = body.push(
                Container::new(row(vec![
                    (
                        text(entry.when.format("%Y-%m-%d %H:%M").to_string()).size(13).into(),
                        3,
                    ),
                    (text(entry.game.clone()).size(13).into(), 5),
                    (
                        text(if entry.full {
                            TRANSLATOR.logs_kind_full()
                        } else {
                            TRANSLATOR.logs_kind_differential()
                        })
                        .size(13)
                        .into(),
                        2,
                    ),
                    (
                        text(match &entry.comment {
                            // O comentário é do usuário, e é a única parte do registro que ele
                            // escreveu: some junto com a mudança, nunca numa coluna que ele
                            // precise procurar.
                            Some(comment) => format!("{} — {}", entry.change(), comment),
                            None => entry.change(),
                        })
                        .size(13)
                        .into(),
                        5,
                    ),
                    (text(entry.files.to_string()).size(13).into(), 2),
                    (text(TRANSLATOR.adjusted_size(entry.bytes)).size(13).into(), 2),
                ]))
                .class(style::Container::TableRow),
            );
        }

        Column::new()
            .width(Length::Fill)
            .push(header)
            .push(Scrollable::new(body).height(Length::Fill))
            .into()
    }
}
