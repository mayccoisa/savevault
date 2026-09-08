//! The system log: what each backup RUN did.
//!
//! The walk over the backup folder lives in [`crate::gui::vault`], because the same read answers
//! two different questions: this screen groups it by execution, and the Restore screen groups it
//! by game. Two walks would be two chances to disagree about what is on disk.

use crate::{
    gui::{design, vault::Entry},
    lang::TRANSLATOR,
    resource::config::Config,
};

/// One execution: everything a single press of "Back up" wrote.
///
/// This is what the screen lists now. A flat list of game-backups answers "what happened to this
/// game", which is the second question. The first one is "what did that run do", and before runs
/// had an identity it could not be answered at all.
#[derive(Clone, Debug)]
pub struct Run {
    /// Also the key that identifies the run in the interface: it is stable across a reload of the
    /// log in a way that a position in a list is not.
    pub when: chrono::DateTime<chrono::Local>,
    pub entries: Vec<Entry>,
}

impl Run {
    pub fn added(&self) -> usize {
        self.entries.iter().map(|x| x.added).sum()
    }

    pub fn changed(&self) -> usize {
        self.entries.iter().map(|x| x.changed).sum()
    }

    pub fn removed(&self) -> usize {
        self.entries.iter().map(|x| x.removed).sum()
    }

    pub fn files(&self) -> usize {
        self.entries.iter().map(|x| x.files).sum()
    }

    pub fn bytes(&self) -> u64 {
        self.entries.iter().map(|x| x.bytes).sum()
    }

    /// One phrase for what the whole run changed, in the same shape a single entry uses.
    pub fn change(&self) -> String {
        let (added, changed, removed) = (self.added(), self.changed(), self.removed());
        if added == 0 && changed == 0 && removed == 0 {
            return TRANSLATOR.logs_no_change();
        }

        let mut parts = vec![];
        if added > 0 {
            parts.push(TRANSLATOR.logs_added(added));
        }
        if changed > 0 {
            parts.push(TRANSLATOR.logs_changed(changed));
        }
        if removed > 0 {
            parts.push(TRANSLATOR.logs_removed(removed));
        }
        parts.join(" · ")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Logs {
    pub runs: Vec<Run>,
    /// Set once the backup folder has been read, so an empty vector can say "nothing here yet"
    /// instead of "not looked yet". They are different answers and deserve different screens.
    pub loaded: bool,
    /// The run the user opened, if any. Keyed by its instant and not by its position, so reloading
    /// the log while a run is open cannot quietly show a different one.
    pub opened: Option<chrono::DateTime<chrono::Local>>,
}

impl Logs {
    /// Walks the backup folder and rebuilds the log from the backups themselves.
    pub fn load(config: &Config) -> Self {
        Self {
            runs: Self::group_into_runs(crate::gui::vault::entries(config)),
            loaded: true,
            opened: None,
        }
    }

    /// Gathers the game-backups into the executions that produced them.
    ///
    /// From this version on the grouping is **exact**: every game of one run is stamped with the
    /// same instant, so an equal timestamp means the same execution and nothing is being guessed.
    ///
    /// For backups already on disk it cannot be exact, because each game was stamped with its own
    /// `now` as it finished, milliseconds to seconds apart and in whatever order the thread pool
    /// happened to complete them. That history has no run recorded in it and no amount of reading
    /// will recover one, so those entries are gathered by the gap between them: while each backup
    /// starts within [`RUN_GAP`] of the previous one, they are treated as one execution.
    ///
    /// The gap is deliberately not a fixed window. A window of, say, five minutes would split a
    /// run that took six, and would merge two runs four minutes apart. Chaining on the gap keeps a
    /// long run together for as long as it kept working, and still separates two runs that have a
    /// real pause between them.
    ///
    /// **A run that has proved itself exact stops accepting the gap.** As soon as two entries share
    /// an instant, the run is known to come from a version that stamps its games, and only an equal
    /// timestamp can join it. Without that seal, two real runs a minute apart would be shown as one
    /// — the guess meant for old data would start corrupting good data.
    ///
    /// What stays ambiguous, and cannot be resolved from what is on disk: two runs of a **single**
    /// game each, less than [`RUN_GAP`] apart, look exactly like one old-style run of two games.
    /// Those are shown as one. It is a narrow case and it shrinks to nothing as old backups age out.
    fn group_into_runs(entries: Vec<Entry>) -> Vec<Run> {
        /// How long a silence has to be before it counts as the end of an execution.
        ///
        /// Only ever consulted for backups written before runs had a shared timestamp.
        const RUN_GAP: chrono::TimeDelta = chrono::TimeDelta::seconds(90);

        let mut runs: Vec<Run> = vec![];
        // Whether the run being built has already shown two games sharing one instant.
        let mut sealed = false;

        for entry in entries {
            // The list is newest first, so "the previous one" is the later backup, and the gap is
            // measured backwards from it.
            let joins = match runs.last() {
                Some(run) if run.when == entry.when => Some(true),
                Some(run) if !sealed => Some(run.entries.last().is_some_and(|last| last.when - entry.when <= RUN_GAP)),
                Some(_) => Some(false),
                None => None,
            };

            match joins {
                Some(true) => {
                    let run = runs.last_mut().expect("just matched on it");
                    sealed |= run.when == entry.when;
                    run.entries.push(entry);
                }
                _ => {
                    sealed = false;
                    runs.push(Run {
                        when: entry.when,
                        entries: vec![entry],
                    });
                }
            }
        }

        // Inside a run the order is the user's, not the thread pool's: by game name.
        for run in &mut runs {
            run.entries.sort_by(|a, b| a.game.cmp(&b.game));
        }

        runs
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

/// One place decides the shape of the table, so a header can never drift from its rows.
fn row<'a>(cells: Vec<(crate::gui::widget::Element<'a>, u16)>) -> crate::gui::widget::Row<'a> {
    use crate::gui::widget::{Container, Row};
    use iced::{Alignment, Length};

    let mut row = Row::new()
        .spacing(design::space::MD)
        .align_y(Alignment::Center)
        .padding([design::space::SM, design::space::MD]);
    for (cell, portion) in cells {
        row = row.push(Container::new(cell).width(Length::FillPortion(portion)));
    }
    row
}

fn head<'a>(label: String, portion: u16) -> (crate::gui::widget::Element<'a>, u16) {
    (
        crate::gui::widget::text(label).size(design::text::CAPTION).into(),
        portion,
    )
}

fn cell<'a>(value: String, portion: u16) -> (crate::gui::widget::Element<'a>, u16) {
    (crate::gui::widget::text(value).size(design::text::BODY).into(), portion)
}

impl Logs {
    pub fn view(&self) -> crate::gui::widget::Element<'_> {
        use crate::gui::widget::{Container, text};
        use iced::Length;

        if !self.loaded || self.runs.is_empty() {
            return Container::new(text(TRANSLATOR.logs_empty()).size(design::text::BODY))
                .padding(design::space::XXL)
                .width(Length::Fill)
                .into();
        }

        match self
            .opened
            .and_then(|when| self.runs.iter().find(|run| run.when == when))
        {
            Some(run) => self.view_run(run),
            None => self.view_runs(),
        }
    }

    /// The list of executions: one row per run, newest first.
    fn view_runs(&self) -> crate::gui::widget::Element<'_> {
        use crate::gui::{
            common::Message,
            style,
            widget::{Button, Column, Container, Scrollable},
        };
        use iced::Length;

        let header = Container::new(row(vec![
            head(TRANSLATOR.logs_column_when(), 4),
            head(TRANSLATOR.total_games(), 2),
            head(TRANSLATOR.logs_column_change(), 6),
            head(TRANSLATOR.logs_column_files(), 2),
            head(TRANSLATOR.logs_column_size(), 2),
        ]))
        .class(style::Container::TableHeader);

        let mut body = Column::new();
        for run in &self.runs {
            // The whole row is the target, not a link at the end of it: the row is what the user is
            // pointing at, and a run has nowhere else to go.
            body = body.push(
                Button::new(row(vec![
                    cell(run.when.format("%Y-%m-%d %H:%M").to_string(), 4),
                    cell(run.entries.len().to_string(), 2),
                    cell(run.change(), 6),
                    cell(run.files().to_string(), 2),
                    cell(TRANSLATOR.adjusted_size(run.bytes()), 2),
                ]))
                .on_press(Message::OpenLogRun(Some(run.when)))
                .class(style::Button::Bare)
                .padding(0)
                .width(Length::Fill),
            );
        }

        Column::new()
            .width(Length::Fill)
            .push(header)
            .push(Scrollable::new(body).width(Length::Fill).height(Length::Fill))
            .into()
    }

    /// One execution, opened: everything it did, a row per game.
    fn view_run<'a>(&'a self, run: &'a Run) -> crate::gui::widget::Element<'a> {
        use crate::gui::{
            button,
            common::Message,
            font, style,
            widget::{Column, Container, Row, Scrollable, text},
        };
        use iced::{Alignment, Length};

        let back = Row::new()
            .width(Length::Fill)
            .spacing(design::space::MD)
            .align_y(Alignment::Center)
            .padding([design::space::SM, design::space::MD])
            .push(button::back(Message::OpenLogRun(None)))
            .push(
                text(TRANSLATOR.logs_run_detail(&run.when.format("%Y-%m-%d %H:%M").to_string()))
                    .font(font::TEXT_STRONG)
                    .size(design::text::SUBTITLE)
                    .line_height(design::leading::TITLE),
            )
            .push(iced::widget::space().width(Length::Fill))
            .push(
                text(format!(
                    "{} · {} · {}",
                    run.entries.len(),
                    run.change(),
                    TRANSLATOR.adjusted_size(run.bytes())
                ))
                .size(design::text::CAPTION)
                .class(style::Text::Muted),
            );

        let header = Container::new(row(vec![
            head(TRANSLATOR.logs_column_game(), 6),
            head(TRANSLATOR.logs_column_backup(), 2),
            head(TRANSLATOR.logs_column_change(), 6),
            head(TRANSLATOR.logs_column_files(), 2),
            head(TRANSLATOR.logs_column_size(), 2),
        ]))
        .class(style::Container::TableHeader);

        let mut body = Column::new();
        for entry in &run.entries {
            body = body.push(
                Container::new(row(vec![
                    cell(entry.game.clone(), 6),
                    cell(
                        if entry.full {
                            TRANSLATOR.logs_kind_full()
                        } else {
                            TRANSLATOR.logs_kind_differential()
                        },
                        2,
                    ),
                    cell(
                        match &entry.comment {
                            // O comentário é do usuário, e é a única parte do registro que ele
                            // escreveu: some junto com a mudança, nunca numa coluna que ele
                            // precise procurar.
                            Some(comment) => format!("{} — {}", entry.change(), comment),
                            None => entry.change(),
                        },
                        6,
                    ),
                    cell(entry.files.to_string(), 2),
                    cell(TRANSLATOR.adjusted_size(entry.bytes), 2),
                ]))
                .class(style::Container::TableRow),
            );
        }

        Column::new()
            .width(Length::Fill)
            .push(back)
            .push(header)
            .push(Scrollable::new(body).width(Length::Fill).height(Length::Fill))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> chrono::DateTime<chrono::Local> {
        use chrono::TimeZone;
        chrono::Local.timestamp_opt(1_700_000_000 + seconds, 0).unwrap()
    }

    fn entry(when: chrono::DateTime<chrono::Local>, game: &str) -> Entry {
        Entry {
            when,
            game: game.to_string(),
            full: true,
            added: 1,
            changed: 0,
            removed: 0,
            files: 3,
            bytes: 100,
            comment: None,
            emulator: None,
        }
    }

    /// The case that matters from this version on: one run stamps every game with one instant.
    #[test]
    fn gathers_one_run_from_a_shared_timestamp() {
        let runs = Logs::group_into_runs(vec![entry(at(0), "c"), entry(at(0), "a"), entry(at(0), "b")]);

        assert_eq!(1, runs.len());
        assert_eq!(3, runs[0].entries.len());
        // Inside a run the order is the user's, not the thread pool's.
        assert_eq!(
            vec!["a", "b", "c"],
            runs[0].entries.iter().map(|x| x.game.as_str()).collect::<Vec<_>>()
        );
    }

    /// Backups written before runs had a shared timestamp: seconds apart, still one execution.
    #[test]
    fn chains_older_backups_that_kept_going() {
        // Newest first, as `load` sorts them.
        let runs = Logs::group_into_runs(vec![
            entry(at(120), "d"),
            entry(at(80), "c"),
            entry(at(40), "b"),
            entry(at(0), "a"),
        ]);

        assert_eq!(1, runs.len(), "40s between each backup is one run that kept working");
        assert_eq!(4, runs[0].entries.len());
    }

    /// A real pause is a real boundary. This is what a fixed window would get wrong.
    #[test]
    fn splits_when_the_gap_is_a_pause() {
        let runs = Logs::group_into_runs(vec![entry(at(300), "b"), entry(at(0), "a")]);

        assert_eq!(2, runs.len());
        assert_eq!("b", runs[0].entries[0].game);
        assert_eq!("a", runs[1].entries[0].game);
    }

    /// The run is stamped with the newest backup in it, which is the one the list is sorted by.
    #[test]
    fn the_run_carries_the_instant_of_its_first_entry() {
        let runs = Logs::group_into_runs(vec![entry(at(40), "b"), entry(at(0), "a")]);

        assert_eq!(1, runs.len());
        assert_eq!(at(40), runs[0].when);
    }

    /// The seal: once a run is known to be exact, a nearby run is a different run.
    #[test]
    fn does_not_swallow_a_second_exact_run_that_happened_moments_later() {
        let runs = Logs::group_into_runs(vec![
            entry(at(60), "a"),
            entry(at(60), "b"),
            entry(at(0), "c"),
            entry(at(0), "d"),
        ]);

        assert_eq!(2, runs.len(), "60s apart, but both stamped their own games");
        assert_eq!(2, runs[0].entries.len());
        assert_eq!(2, runs[1].entries.len());
    }

    #[test]
    fn totals_are_the_sum_of_the_run() {
        let mut second = entry(at(0), "b");
        second.added = 2;
        second.changed = 5;
        second.bytes = 400;

        let runs = Logs::group_into_runs(vec![entry(at(0), "a"), second]);

        assert_eq!(1, runs.len());
        assert_eq!(3, runs[0].added());
        assert_eq!(5, runs[0].changed());
        assert_eq!(500, runs[0].bytes());
    }
}
