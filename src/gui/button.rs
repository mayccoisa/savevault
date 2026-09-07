use iced::{Length, alignment, keyboard};

use crate::{
    gui::{
        common::{
            BackupPhase, BrowseFileSubject, BrowseSubject, Message, Operation, RestorePhase, Screen, ValidatePhase,
        },
        icon::Icon,
        style,
        widget::{Button, Container, Element, Row, Text, Tooltip, text},
    },
    lang::TRANSLATOR,
    prelude::{EditAction, Finality, SyncDirection},
    resource::{config, manifest},
    scan::game_filter,
};

const WIDTH: u32 = 125;

fn template(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(5)
        .into()
}

fn template_bare(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(0)
        .into()
}

fn template_extended(
    content: Text,
    action: Option<Message>,
    style: Option<style::Button>,
    icon: Option<Icon>,
    tooltip: Option<String>,
) -> Element {
    let button = match icon {
        Some(icon) => template_complex(
            Container::new(
                Row::new()
                    .spacing(5)
                    .push(icon.text_narrow())
                    .push(content.width(Length::Shrink)),
            )
            .center_x(WIDTH),
            action,
            style,
        ),
        None => template(content, action, style),
    };

    match tooltip {
        Some(tooltip) => Tooltip::new(button, text(tooltip), iced::widget::tooltip::Position::Top)
            .class(style::Container::Tooltip)
            .into(),
        None => button,
    }
}

fn template_complex<'a>(
    content: impl Into<Element<'a>>,
    action: Option<Message>,
    style: Option<style::Button>,
) -> Element<'a> {
    Button::new(content)
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(5)
        .into()
}

pub fn primary<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Primary)
        .padding(5)
        .width(WIDTH)
        .into()
}

pub fn negative<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Negative)
        .width(WIDTH)
        .padding(5)
        .into()
}

pub fn add<'a>(action: impl Fn(EditAction) -> Message) -> Element<'a> {
    template(Icon::AddCircle.text(), Some(action(EditAction::Add)), None)
}

pub fn add_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize) -> Element<'a> {
    template(Icon::AddCircle.text(), Some(action(parent, EditAction::Add)), None)
}

pub fn remove<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn remove_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        Some(action(parent, EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn delete<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::Delete.text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn hide<'a>(action: Message) -> Element<'a> {
    template(Icon::VisibilityOff.text_small(), Some(action), None)
}

/// A bar button that is not the primary action: outlined, and only as wide as its label.
///
/// The fixed 125px of `primary` is what made the old row of actions read like a form. In a
/// command bar the width has to come from the word.
pub fn secondary<'a>(content: String, action: Option<Message>, tooltip: Option<String>) -> Element<'a> {
    let button: Element<'a> = Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Secondary)
        .padding([6, 14])
        .into();

    match tooltip {
        Some(tooltip) => Tooltip::new(button, text(tooltip), iced::widget::tooltip::Position::Bottom)
            .class(style::Container::Tooltip)
            .into(),
        None => button,
    }
}

/// The primary action of the bar: filled, and as wide as its label.
pub fn primary_bar<'a>(content: String, action: Option<Message>, tooltip: Option<String>) -> Element<'a> {
    let button: Element<'a> = Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Primary)
        .padding([6, 16])
        .into();

    match tooltip {
        Some(tooltip) => Tooltip::new(button, text(tooltip), iced::widget::tooltip::Position::Bottom)
            .class(style::Container::Tooltip)
            .into(),
        None => button,
    }
}

/// An icon button in the command bar.
pub fn bar_icon<'a>(icon: Icon, action: Option<Message>, active: bool, tooltip: Option<String>) -> Element<'a> {
    let button: Element<'a> = Button::new(icon.text_narrow())
        .on_press_maybe(action)
        .class(if active {
            style::Button::Negative
        } else {
            style::Button::Secondary
        })
        .padding([6, 12])
        .into();

    match tooltip {
        Some(tooltip) => Tooltip::new(button, text(tooltip), iced::widget::tooltip::Position::Bottom)
            .class(style::Container::Tooltip)
            .into(),
        None => button,
    }
}

/// Finding the games. It is the same message in every state; only the word and the weight change,
/// because before the first scan there is nothing to preview and "Preview" would be a lie.
pub fn scan<'a>(ongoing: &Operation, scan_kind: crate::scan::ScanKind, scanned: bool) -> Element<'a> {
    use crate::scan::ScanKind;

    let cancelling = matches!(
        ongoing,
        Operation::Backup {
            finality: Finality::Preview,
            cancelling: true,
            ..
        } | Operation::Restore {
            finality: Finality::Preview,
            cancelling: true,
            ..
        }
    );
    let scanning = matches!(
        ongoing,
        Operation::Backup {
            finality: Finality::Preview,
            cancelling: false,
            ..
        } | Operation::Restore {
            finality: Finality::Preview,
            cancelling: false,
            ..
        }
    );

    let label = if cancelling {
        TRANSLATOR.cancelling_button()
    } else if scanning {
        TRANSLATOR.cancel_button()
    } else if scanned {
        TRANSLATOR.rescan_button()
    } else {
        TRANSLATOR.scan_button()
    };

    let action = if scanning {
        Some(Message::CancelOperation)
    } else if ongoing.idle() {
        Some(match scan_kind {
            ScanKind::Backup => Message::Backup(BackupPhase::Start {
                preview: true,
                repair: false,
                jump: false,
                games: None,
            }),
            ScanKind::Restore => Message::Restore(RestorePhase::Start {
                preview: true,
                games: None,
            }),
        })
    } else {
        None
    };

    // Antes da primeira varredura ela É a ação principal, e por isso vem preenchida.
    if !scanned && !scanning && !cancelling {
        primary_bar(label, action, None)
    } else if scanning || cancelling {
        let button: Element<'a> = Button::new(text(label).align_x(alignment::Horizontal::Center))
            .on_press_maybe(action)
            .class(style::Button::Negative)
            .padding([6, 16])
            .into();
        button
    } else {
        secondary(label, action, None)
    }
}

/// The primary action of the backup bar.
///
/// Presence and availability are different questions, and collapsing them is what produced the
/// button that offered to back up nothing. It is absent before the first scan, because there is
/// no set to act on; it is present and disabled when the scan found nothing new, because there is
/// a set and no delta — and hiding it there would make the bar jump on every scan.
pub fn backup_main<'a>(ongoing: &Operation, filtered: bool, has_changes: bool) -> Element<'a> {
    let cancelling = matches!(
        ongoing,
        Operation::Backup {
            finality: Finality::Final,
            cancelling: true,
            ..
        }
    );
    let running = matches!(
        ongoing,
        Operation::Backup {
            finality: Finality::Final,
            cancelling: false,
            ..
        }
    );

    if cancelling {
        return primary_bar(TRANSLATOR.cancelling_button(), None, None);
    }
    if running {
        let button: Element<'a> = Button::new(text(TRANSLATOR.cancel_button()).align_x(alignment::Horizontal::Center))
            .on_press(Message::CancelOperation)
            .class(style::Button::Negative)
            .padding([6, 16])
            .into();
        return button;
    }

    let enabled = ongoing.idle() && has_changes;
    primary_bar(
        TRANSLATOR.backup_button(),
        enabled.then_some(Message::Backup(BackupPhase::Confirm { games: None })),
        if !has_changes {
            Some(TRANSLATOR.nothing_to_back_up_tooltip())
        } else if filtered {
            Some(TRANSLATOR.operation_will_only_include_listed_games())
        } else {
            None
        },
    )
}

/// The primary action of the restore bar.
///
/// It has no "nothing changed" state on purpose: restoring a backup that matches what is on disk
/// is a legitimate operation, and it is how a local overwrite gets undone.
pub fn restore_main<'a>(ongoing: &Operation, filtered: bool) -> Element<'a> {
    let cancelling = matches!(
        ongoing,
        Operation::Restore {
            finality: Finality::Final,
            cancelling: true,
            ..
        }
    );
    let running = matches!(
        ongoing,
        Operation::Restore {
            finality: Finality::Final,
            cancelling: false,
            ..
        }
    );

    if cancelling {
        return primary_bar(TRANSLATOR.cancelling_button(), None, None);
    }
    if running {
        let button: Element<'a> = Button::new(text(TRANSLATOR.cancel_button()).align_x(alignment::Horizontal::Center))
            .on_press(Message::CancelOperation)
            .class(style::Button::Negative)
            .padding([6, 16])
            .into();
        return button;
    }

    primary_bar(
        TRANSLATOR.restore_button(),
        ongoing
            .idle()
            .then_some(Message::Restore(RestorePhase::Confirm { games: None })),
        filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
    )
}

pub fn choose_folder<'a>(subject: BrowseSubject, modifiers: &keyboard::Modifiers) -> Element<'a> {
    if modifiers.shift() {
        template(Icon::OpenInNew.text(), Some(Message::OpenDirSubject(subject)), None)
    } else {
        template(Icon::FolderOpen.text(), Some(Message::BrowseDir(subject)), None)
    }
}

pub fn choose_file<'a>(subject: BrowseFileSubject, modifiers: &keyboard::Modifiers) -> Element<'a> {
    if modifiers.shift() {
        template(Icon::OpenInNew.text(), Some(Message::OpenFileSubject(subject)), None)
    } else {
        template(Icon::FolderOpen.text(), Some(Message::BrowseFile(subject)), None)
    }
}

pub fn filter<'a>(open: bool) -> Element<'a> {
    template(
        Icon::Filter.text(),
        Some(Message::Filter {
            event: game_filter::Event::Toggled,
        }),
        open.then_some(style::Button::Negative),
    )
}

pub fn reset_filter<'a>(dirty: bool) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        dirty.then_some(Message::Filter {
            event: game_filter::Event::Reset,
        }),
        Some(style::Button::Negative),
    )
}

pub fn sort<'a>(message: impl Into<Message>) -> Element<'a> {
    template(text(TRANSLATOR.sort_button()).width(WIDTH), Some(message.into()), None)
}

pub fn refresh<'a>(action: Message, ongoing: bool) -> Element<'a> {
    template(Icon::Refresh.text(), (!ongoing).then_some(action), None)
}

pub fn refresh_custom_game<'a>(action: Message, ongoing: bool, enabled: bool) -> Element<'a> {
    template(Icon::Refresh.text(), (!ongoing && enabled).then_some(action), None)
}

pub fn search<'a>(action: Message) -> Element<'a> {
    template(Icon::Search.text(), Some(action), None)
}

pub fn move_up<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (index > 0).then(|| action(EditAction::move_up(index))),
        None,
    )
}

pub fn move_up_maybe<'a>(action: impl Fn(EditAction) -> Message, index: usize, enabled: bool) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (enabled && index > 0).then(|| action(EditAction::move_up(index))),
        None,
    )
}

pub fn move_up_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (index > 0).then(|| action(parent, EditAction::move_up(index))),
        None,
    )
}

pub fn move_down<'a>(action: impl Fn(EditAction) -> Message, index: usize, max: usize) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (index < max - 1).then(|| action(EditAction::move_down(index))),
        None,
    )
}

pub fn move_down_maybe<'a>(
    action: impl Fn(EditAction) -> Message,
    index: usize,
    max: usize,
    enabled: bool,
) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (enabled && index < max - 1).then(|| action(EditAction::move_down(index))),
        None,
    )
}

pub fn move_down_nested<'a>(
    action: impl Fn(usize, EditAction) -> Message,
    parent: usize,
    index: usize,
    max: usize,
) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (index < max - 1).then(|| action(parent, EditAction::move_down(index))),
        None,
    )
}

pub fn next_page<'a>(action: impl Fn(usize) -> Message, page: usize, pages: usize) -> Element<'a> {
    template(
        Icon::ArrowForward.text(),
        (page < pages).then(|| action(page + 1)),
        None,
    )
}

pub fn previous_page<'a>(action: impl Fn(usize) -> Message, page: usize) -> Element<'a> {
    template(Icon::ArrowBack.text(), (page > 0).then(|| action(page - 1)), None)
}

pub fn toggle_all_custom_games<'a>(all_enabled: bool, filtered: bool) -> Element<'a> {
    if all_enabled {
        template_extended(
            text(TRANSLATOR.disable_all_button()).width(WIDTH),
            Some(Message::DeselectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    } else {
        template_extended(
            text(TRANSLATOR.enable_all_button()).width(WIDTH),
            Some(Message::SelectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    }
}

pub fn add_game<'a>() -> Element<'a> {
    template(
        text(TRANSLATOR.add_game_button()).width(WIDTH),
        Some(config::Event::CustomGame(EditAction::Add).into()),
        None,
    )
}

pub fn add_emulator_root<'a>(app: crate::scan::emulator::App) -> Element<'a> {
    template(
        text(TRANSLATOR.add_emulator_folder_button()).width(WIDTH),
        Some(config::Event::AddEmulatorRoot(app).into()),
        None,
    )
}

/// Verifica e instala a atualização num clique.
///
/// Fica desabilitado enquanto a atualização corre, para o usuário não disparar dois downloads
/// que iriam mexer no mesmo arquivo ao mesmo tempo.
pub fn check_for_update<'a>(updating: &bool) -> Element<'a> {
    template(
        text(TRANSLATOR.check_for_update_button()).size(14),
        (!*updating).then_some(Message::UpdateApp),
        None,
    )
}

pub fn refresh_emulators<'a>() -> Element<'a> {
    template(Icon::Refresh.text(), Some(Message::RefreshEmulators), None)
}

pub fn open_url<'a>(label: String, url: String) -> Element<'a> {
    template(text(label).width(WIDTH), Some(Message::OpenUrl(url)), None)
}

pub fn open_url_icon<'a>(url: String) -> Element<'a> {
    template(Icon::OpenInBrowser.text(), Some(Message::OpenUrl(url)), None)
}

pub fn side_nav<'a>(screen: Screen, current_screen: Screen) -> Button<'a> {
    let label = match screen {
        Screen::Backup => TRANSLATOR.nav_backup_button(),
        Screen::Restore => TRANSLATOR.nav_restore_button(),
        Screen::CustomGames => TRANSLATOR.nav_custom_games_button(),
        Screen::Emulators => TRANSLATOR.nav_emulators_button(),
        Screen::Logs => TRANSLATOR.nav_logs_button(),
        Screen::Other => TRANSLATOR.nav_other_button(),
    };

    Button::new(text(label).size(14).align_x(alignment::Horizontal::Left))
        .on_press(Message::SwitchScreen(screen))
        .width(Length::Fill)
        .padding([10, 11])
        .class(if current_screen == screen {
            style::Button::SideNavActive
        } else {
            style::Button::SideNavInactive
        })
}

pub fn upload<'a>(operation: &Operation) -> Element<'a> {
    template(
        Icon::Upload.text(),
        match operation {
            Operation::Idle => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Upload,
            }),
            Operation::Cloud {
                direction: SyncDirection::Upload,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Operation::Cloud {
                direction: SyncDirection::Upload,
                ..
            } => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn download<'a>(operation: &Operation) -> Element<'a> {
    template(
        Icon::Download.text(),
        match operation {
            Operation::Idle => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Download,
            }),
            Operation::Cloud {
                direction: SyncDirection::Download,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Operation::Cloud {
                direction: SyncDirection::Download,
                ..
            } => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn validate_backups<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        text(match ongoing {
            Operation::ValidateBackups { cancelling: false, .. } => TRANSLATOR.cancel_button(),
            Operation::ValidateBackups { cancelling: true, .. } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.validate_button(),
        })
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::ValidateBackups(ValidatePhase::Start)),
            Operation::ValidateBackups { cancelling: false, .. } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(ongoing, Operation::ValidateBackups { .. }).then_some(style::Button::Negative),
    )
}

pub fn show_game_notes<'a>(game: String, notes: Vec<manifest::Note>) -> Element<'a> {
    template_bare(
        Icon::Info.text_narrow(),
        Some(Message::ShowGameNotes { game, notes }),
        Some(style::Button::Bare),
    )
}

pub fn expand<'a>(expanded: bool, on_press: Message) -> Element<'a> {
    Button::new(
        (if expanded {
            Icon::KeyboardArrowDown
        } else {
            Icon::KeyboardArrowRight
        })
        .text_small(),
    )
    .on_press(on_press)
    .class(style::Button::Primary)
    .padding(5)
    .height(25)
    .width(25)
    .into()
}
