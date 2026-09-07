use std::collections::HashSet;

use iced::{Alignment, Length, keyboard, padding};

use crate::{
    cloud::{Remote, RemoteChoice},
    gui::{
        badge::Badge,
        button,
        common::{BrowseFileSubject, BrowseSubject, Message, Operation, ScrollSubject, UndoSubject},
        editor,
        game_list::GameList,
        icon::Icon,
        search::CustomGamesFilter,
        shortcuts::TextHistories,
        style,
        widget::{Button, Column, Container, Element, IcedParentExt, Row, checkbox, number_input, pick_list, text},
    },
    lang::{Language, TRANSLATOR},
    prelude::{AVAILABLE_PARALELLISM, STEAM_DECK, StrictPath},
    resource::{
        cache::Cache,
        config::{self, Accent, BackupFormat, CloudFilter, Config, Theme, ZipCompression},
        manifest::{Manifest, Store},
    },
    scan::{DuplicateDetector, Duplication, OperationStatus, ScanKind, game_filter},
};

const RCLONE_URL: &str = "https://rclone.org/downloads";
const RELEASE_URL: &str = "https://github.com/mtkennerly/ludusavi/releases";

fn template(content: Column) -> Element {
    // A coluna precisa declarar a largura: sem isto ela fica com a largura natural do que
    // ha dentro, e o que e Fill la dentro passa por cima da borda direita da janela.
    Container::new(content.width(Length::Fill).spacing(15).align_x(Alignment::Center))
        .height(Length::Fill)
        .center_x(Length::Fill)
        .padding(padding::all(5))
        .into()
}

/// A faixa de contexto: o resumo da varredura à esquerda, o destino recolhido à direita.
///
/// Ela substitui a faixa de status em corpo 25, que era o maior tipo da tela e competia com o
/// título logo acima. Nenhum número saiu: o que mudou foi o peso.
fn context_strip<'a>(
    summary: String,
    status: &OperationStatus,
    duplication: Duplication,
    target: Element<'a>,
) -> Element<'a> {
    Row::new()
        .height(38)
        .padding([0, 24])
        .spacing(10)
        .align_y(Alignment::Center)
        .push(text(summary).size(12).class(style::Text::Muted))
        .push_if(status.changed_games.new > 0, || {
            Badge::new_entry_with_count(status.changed_games.new).view()
        })
        .push_if(status.changed_games.different > 0, || {
            Badge::changed_entry_with_count(status.changed_games.different).view()
        })
        .push_if(!duplication.unique(), || {
            Badge::new(&TRANSLATOR.badge_duplicates())
                .faded(duplication.resolved())
                .view()
        })
        .push(iced::widget::space().width(Length::Fill))
        .push(target)
        .into()
}

/// O texto do resumo, que também é onde o motivo de a ação principal estar apagada aparece.
fn scan_summary(log: &GameList, operation: &Operation, status: &OperationStatus) -> String {
    if !operation.idle() {
        return TRANSLATOR.scanning_label();
    }
    if log.entries.is_empty() {
        return TRANSLATOR.not_scanned_yet_label();
    }

    let counted = format!(
        "{} · {}",
        TRANSLATOR.processed_games(status),
        TRANSLATOR.adjusted_size(status.total_bytes)
    );

    if status.changed_games.new + status.changed_games.different == 0 {
        format!("{} · {}", counted, TRANSLATOR.nothing_changed_label())
    } else {
        counted
    }
}

/// O destino recolhido: prefixo, nome da última pasta, e o caminho inteiro na dica.
fn target_chip<'a>(
    label: String,
    path: &StrictPath,
    scan_kind: ScanKind,
    operation: &Operation,
) -> Element<'a> {
    let rendered = path.render();
    let leaf = rendered
        .replace('\\', "/")
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(&rendered)
        .to_string();

    button::secondary(
        format!("{label} {leaf}"),
        operation
            .idle()
            .then_some(Message::ToggleTargetEditor { scan_kind }),
        Some(rendered),
    )
}

/// A gaveta: o mesmo campo de sempre, com o mesmo histórico de desfazer.
///
/// Não existe passo de confirmação porque o campo já grava a cada tecla e já é reversível pelo
/// desfazer dele. Um "Salvar" aqui criaria um estado intermediário que o resto do app não tem.
fn target_drawer<'a>(field: Element<'a>, browse: Element<'a>, scan_kind: ScanKind) -> Element<'a> {
    Container::new(
        Row::new()
            .height(52)
            .padding([0, 24])
            .spacing(10)
            .align_y(Alignment::Center)
            .push(field)
            .push(browse)
            .push(button::secondary(
                TRANSLATOR.done_button(),
                Some(Message::ToggleTargetEditor { scan_kind }),
                None,
            )),
    )
    .class(style::Container::GameListEntry)
    .into()
}

#[derive(Default)]
pub struct Backup {
    pub log: GameList,
    pub previewed_games: HashSet<String>,
    pub duplicate_detector: DuplicateDetector,
    /// A gaveta do destino. Recolhida por padrão: o caminho do cofre se escolhe uma vez e se
    /// consulta de vez em quando, então ele não paga uma faixa da tela o tempo todo.
    pub target_editor_open: bool,
}

impl Backup {
    const SCAN_KIND: ScanKind = ScanKind::Backup;

    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(Self::SCAN_KIND, config, cache),
            ..Default::default()
        }
    }

    /// Os comandos da tela, montados na barra de título.
    ///
    /// Eles moram lá porque a barra já existe e já reserva 62px carregando uma palavra só. Uma
    /// faixa própria de ações abaixo dela reporia o empilhamento que esta rodada veio desfazer.
    pub fn commands<'a>(&'a self, config: &Config, manifest: &Manifest, operation: &Operation) -> Row<'a> {
        let duplicatees = self.log.duplicatees(&self.duplicate_detector);
        let status = self.log.compute_operation_status(
            config,
            Self::SCAN_KIND,
            manifest,
            &self.duplicate_detector,
            duplicatees.as_ref(),
        );
        let scanned = !self.log.entries.is_empty();
        let has_changes = status.changed_games.new + status.changed_games.different > 0;

        Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(button::scan(operation, Self::SCAN_KIND, scanned))
            // Sem lista varrida não há sobre o que filtrar, e um filtro que não filtra nada é
            // um controle que só ensina que clicar ali não faz nada.
            .push(button::bar_icon(
                Icon::VisibilityOff,
                (scanned && operation.idle())
                    .then(|| config::Event::ShowUnchangedGames(!config.scan.show_unchanged_games).into()),
                !config.scan.show_unchanged_games,
                Some(TRANSLATOR.only_changes_tooltip(config.scan.show_unchanged_games)),
            ))
            .push(button::bar_icon(
                Icon::Filter,
                (scanned && operation.idle()).then_some(Message::Filter {
                    event: game_filter::Event::Toggled,
                }),
                self.log.search.show,
                None,
            ))
            .push(iced::widget::space().width(16))
            // A ação principal só existe quando existe conjunto sobre o que agir. Antes da
            // primeira varredura a ação possível é encontrar os jogos, e ela já está aí à
            // esquerda; depois de varrer sem nada novo, o botão fica presente e apagado com o
            // motivo, porque sumir faria a barra dançar a cada varredura.
            .push_if(scanned, || {
                button::backup_main(operation, self.log.is_filtered(), has_changes)
            })
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element {

        let duplicatees = self.log.duplicatees(&self.duplicate_detector);

        let status = self.log.compute_operation_status(
            config,
            Self::SCAN_KIND,
            manifest,
            &self.duplicate_detector,
            duplicatees.as_ref(),
        );

        let content = Column::new()
            .push(context_strip(
                scan_summary(&self.log, operation, &status),
                &status,
                self.duplicate_detector.overall(),
                target_chip(
                    TRANSLATOR.backup_target_label(),
                    &config.backup.path,
                    Self::SCAN_KIND,
                    operation,
                ),
            ))
            .push_if(self.target_editor_open && operation.idle(), || {
                target_drawer(
                    histories.input(UndoSubject::BackupTarget),
                    button::choose_folder(BrowseSubject::BackupTarget, modifiers),
                    Self::SCAN_KIND,
                )
            })
            .push(self.log.view(
                Self::SCAN_KIND,
                config,
                manifest,
                &self.duplicate_detector,
                duplicatees.as_ref(),
                operation,
                histories,
                modifiers,
            ));

        template(content)
    }
}

#[derive(Default)]
pub struct Restore {
    pub log: GameList,
    pub duplicate_detector: DuplicateDetector,
    /// Ver o campo irmão em [`Backup`].
    pub target_editor_open: bool,
}

impl Restore {
    const SCAN_KIND: ScanKind = ScanKind::Restore;

    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(Self::SCAN_KIND, config, cache),
            ..Default::default()
        }
    }

    /// Os comandos da tela, montados na barra de título.
    ///
    /// Eles moram lá porque a barra já existe e já reserva 62px carregando uma palavra só. Uma
    /// faixa própria de ações abaixo dela reporia o empilhamento que esta rodada veio desfazer.
    pub fn commands<'a>(&'a self, config: &Config, _manifest: &Manifest, operation: &Operation) -> Row<'a> {
        let scanned = !self.log.entries.is_empty();

        Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(button::scan(operation, Self::SCAN_KIND, scanned))
            // Sem lista varrida não há sobre o que filtrar, e um filtro que não filtra nada é
            // um controle que só ensina que clicar ali não faz nada.
            .push(button::bar_icon(
                Icon::VisibilityOff,
                (scanned && operation.idle())
                    .then(|| config::Event::ShowUnchangedGames(!config.scan.show_unchanged_games).into()),
                !config.scan.show_unchanged_games,
                Some(TRANSLATOR.only_changes_tooltip(config.scan.show_unchanged_games)),
            ))
            .push(button::bar_icon(
                Icon::Filter,
                (scanned && operation.idle()).then_some(Message::Filter {
                    event: game_filter::Event::Toggled,
                }),
                self.log.search.show,
                None,
            ))
            .push_if(scanned && operation.idle(), || button::validate_backups(operation))
            .push(iced::widget::space().width(16))
            .push_if(scanned, || button::restore_main(operation, self.log.is_filtered()))
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element {

        let duplicatees = self.log.duplicatees(&self.duplicate_detector);

        let status = self.log.compute_operation_status(
            config,
            Self::SCAN_KIND,
            manifest,
            &self.duplicate_detector,
            duplicatees.as_ref(),
        );

        let content = Column::new()
            .push(context_strip(
                scan_summary(&self.log, operation, &status),
                &status,
                self.duplicate_detector.overall(),
                target_chip(
                    TRANSLATOR.restore_source_label(),
                    &config.restore.path,
                    Self::SCAN_KIND,
                    operation,
                ),
            ))
            .push_if(self.target_editor_open && operation.idle(), || {
                target_drawer(
                    histories.input(UndoSubject::RestoreSource),
                    button::choose_folder(BrowseSubject::RestoreSource, modifiers),
                    Self::SCAN_KIND,
                )
            })
            .push(self.log.view(
                Self::SCAN_KIND,
                config,
                manifest,
                &self.duplicate_detector,
                duplicatees.as_ref(),
                operation,
                histories,
                modifiers,
            ));

        template(content)
    }
}

/// A tela dos emuladores.
///
/// Guarda o diagnóstico em vez de recalculá-lo no `view`, porque ele lista pastas em disco.
#[derive(Default)]
pub struct Emulators {
    pub diagnosis: Option<crate::scan::emulator::Diagnosis>,
}

impl Emulators {
    pub fn refresh(&mut self, config: &Config) {
        self.diagnosis = Some(crate::scan::emulator::diagnose(&config.roots));
    }

    /// Traduz o veredito do motor sobre uma pasta. O motor devolve fato; o texto é daqui.
    fn verdict_for<'a>(&self, app: crate::scan::emulator::App, path: &crate::prelude::StrictPath) -> Element<'a> {
        use crate::scan::emulator::FolderVerdict;

        let (label, ok) = match app.inspect_folder(path) {
            FolderVerdict::Empty => (TRANSLATOR.emulator_folder_empty(), false),
            FolderVerdict::Missing => (TRANSLATOR.emulator_folder_missing(), false),
            FolderVerdict::NotThisEmulator { missing } => (
                TRANSLATOR.emulator_folder_wrong(app.name(), &missing.join(", ")),
                false,
            ),
            FolderVerdict::NoSaves { areas } => {
                (TRANSLATOR.emulator_folder_without_saves(&areas.join(", ")), false)
            }
            FolderVerdict::Ready { saves } => (TRANSLATOR.emulator_folder_ready(saves), true),
        };

        Container::new(text(label).size(14))
            .padding([0, 5])
            .class(if ok {
                style::Container::Notification
            } else {
                style::Container::Badge
            })
            .into()
    }

    pub fn view<'a>(
        &'a self,
        config: &'a Config,
        histories: &'a TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        use crate::scan::emulator::App;

        let mut content = Column::new()
            .spacing(15)
            .padding([0, 20])
            .push(
                Row::new()
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(text(TRANSLATOR.emulators_explanation()).width(Length::Fill))
                    .push(button::refresh_emulators()),
            );

        for app in App::ALL {
            let diagnosis = self
                .diagnosis
                .as_ref()
                .and_then(|d| d.emulators.iter().find(|x| x.name == app.name()));

            // O texto vem daqui, e não do `problem` do motor, porque aquele é mensagem de
            // diagnóstico de linha de comando, escrita em inglês. Na interface o usuário lê no
            // idioma dele.
            let status = match diagnosis {
                Some(found) => match &found.data_root {
                    // "Usando <pasta> (0 arquivos de save)" é a frase que enganou na prática: ela
                    // soa como sucesso. Pasta encontrada e vazia é caso próprio, com instrução.
                    Some(root) if found.games.is_empty() => {
                        text(TRANSLATOR.emulator_using_folder_without_saves(root))
                    }
                    Some(root) => text(TRANSLATOR.emulator_using_folder(root, found.games.len())),
                    None => {
                        let matching = found.candidates.iter().filter(|x| x.matches_signature).count();
                        if matching > 1 {
                            text(TRANSLATOR.emulator_ambiguous(app.name(), matching))
                        } else {
                            text(TRANSLATOR.emulator_not_found(app.name()))
                        }
                    }
                },
                None => text(TRANSLATOR.emulator_not_checked_yet()),
            };

            // As raízes deste emulador especificamente. É isto que faz "todas as configurações
            // ficarem dentro do emulador": o usuário não escolhe loja num menu solto.
            let mut block = Column::new()
                .spacing(10)
                .padding(10)
                .push(
                    Row::new()
                        .spacing(15)
                        .align_y(Alignment::Center)
                        .push(text(app.name()).size(20)),
                )
                .push(status);

            for (index, root) in config.roots.iter().enumerate().filter(|(_, root)| {
                matches!(root, crate::resource::config::Root::Emulator(emulator) if emulator.app == Some(*app))
            }) {
                block = block
                    .push(
                        Row::new()
                            .spacing(20)
                            .align_y(Alignment::Center)
                            .push(histories.input(UndoSubject::RootPath(index)))
                            .push(button::choose_folder(BrowseSubject::Root(index), modifiers))
                            .push(button::remove(
                                Message::config(crate::resource::config::Event::Root),
                                index,
                            )),
                    )
                    // O veredito da pasta que o usuário escolheu, e não só o status do emulador.
                    // Sem isto, uma pasta reconhecida porém vazia (a pasta padrão do sistema,
                    // enquanto a instalação de verdade está noutro disco) parecia estar certa, e
                    // o backup vinha vazio sem explicação.
                    .push(self.verdict_for(*app, root.path()));
            }

            block = block.push(button::add_emulator_root(*app));

            content = content.push(Container::new(block).class(style::Container::GameListEntry));
        }

        for planned in App::PLANNED {
            content = content.push(
                Container::new(
                    Row::new()
                        .spacing(15)
                        .padding(10)
                        .align_y(Alignment::Center)
                        .push(text(*planned).size(20))
                        .push(Badge::new(&TRANSLATOR.emulator_coming_soon()).view()),
                )
                .class(style::Container::GameListEntry),
            );
        }

        template(Column::new().push(ScrollSubject::Emulators.into_widget(content)))
    }
}

#[derive(Default)]
pub struct CustomGames {
    pub filter: CustomGamesFilter,
}

impl CustomGames {
    pub fn view<'a>(
        &'a self,
        config: &Config,
        manifest: &Manifest,
        operating: bool,
        histories: &'a TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        let content = Column::new()
            .push(
                Row::new()
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(button::add_game())
                    .push(button::toggle_all_custom_games(
                        self.all_visible_game_selected(config),
                        self.is_filtered(),
                    ))
                    .push(button::sort(config::Event::SortCustomGames))
                    .push(button::filter(self.filter.enabled)),
            )
            .push(self.filter.view(histories))
            .push(editor::custom_games(
                config,
                manifest,
                operating,
                histories,
                modifiers,
                &self.filter,
            ));

        template(content)
    }

    fn is_filtered(&self) -> bool {
        self.filter.enabled
    }

    pub fn visible_games(&self, config: &Config) -> Vec<usize> {
        config
            .custom_games
            .iter()
            .enumerate()
            .filter_map(|(i, game)| self.filter.qualifies(game).then_some(i))
            .collect()
    }

    fn all_visible_game_selected(&self, config: &Config) -> bool {
        config
            .custom_games
            .iter()
            .filter(|game| self.filter.qualifies(game))
            .all(|x| !x.ignore)
    }
}

pub fn other<'a>(
    updating_manifest: bool,
    config: &'a Config,
    cache: &'a Cache,
    operation: &Operation,
    histories: &'a TextHistories,
    modifiers: &keyboard::Modifiers,
) -> Element<'a> {
    let is_rclone_valid = config.apps.rclone.is_valid();
    let is_cloud_configured = config.cloud.remote.is_some();
    let is_cloud_path_valid = crate::cloud::validate_cloud_path(&config.cloud.path).is_ok();

    let content = Column::new()
        .push_if(*STEAM_DECK, || {
            Row::new()
                .padding([0, 20])
                .spacing(20)
                .align_y(iced::Alignment::Center)
                .push(
                    Button::new(text(TRANSLATOR.exit_button()).align_x(iced::alignment::Horizontal::Center))
                        .on_press(Message::Exit { user: true })
                        .width(125)
                        .class(style::Button::Negative)
                        .padding(5),
                )
        })
        .push({
            let content = Column::new()
                .spacing(20)
                .padding(padding::top(0).bottom(5).left(15).right(15))
                .width(Length::Fill)
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(text(TRANSLATOR.field_language()))
                        .push(
                            pick_list(
                                Language::ALL,
                                Some(config.language),
                                Message::config(config::Event::Language),
                            )
                            .class(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(text(TRANSLATOR.field_theme()))
                        .push(
                            pick_list(Theme::ALL, Some(config.theme), Message::config(config::Event::Theme))
                                .class(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(text(TRANSLATOR.field_accent()))
                        .push(
                            pick_list(Accent::ALL, Some(config.accent), Message::config(config::Event::Accent))
                                .class(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(checkbox(
                            TRANSLATOR.new_version_check(),
                            config.release.check,
                            Message::config(config::Event::CheckRelease),
                        ))
                        .push(button::open_url_icon(RELEASE_URL.to_string())),
                )
                .push(
                    Column::new().spacing(5).push(text(TRANSLATOR.scan_field())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(10)
                                .push({
                                    AVAILABLE_PARALELLISM.map(|max_threads| {
                                        Column::new()
                                            .spacing(5)
                                            .push(checkbox(
                                                TRANSLATOR.override_max_threads(),
                                                config.runtime.threads.is_some(),
                                                Message::config(config::Event::OverrideMaxThreads),
                                            ))
                                            .push({
                                                config.runtime.threads.map(|threads| {
                                                    Container::new(number_input(
                                                        threads.get() as i32,
                                                        TRANSLATOR.threads_label(),
                                                        1..=(max_threads.get() as i32),
                                                        Message::config(|x| config::Event::MaxThreads(x as usize)),
                                                    ))
                                                    .padding(padding::left(35))
                                                })
                                            })
                                    })
                                })
                                .push(
                                    checkbox(
                                        TRANSLATOR.explanation_for_exclude_store_screenshots(),
                                        config.backup.filter.exclude_store_screenshots,
                                        Message::config(config::Event::ExcludeStoreScreenshots),
                                    )
                                    .class(style::Checkbox),
                                )
                                .push(checkbox(
                                    TRANSLATOR.show_disabled_games(),
                                    config.scan.show_deselected_games,
                                    Message::config(config::Event::ShowDeselectedGames),
                                ))
                                .push(checkbox(
                                    TRANSLATOR.show_unchanged_games(),
                                    config.scan.show_unchanged_games,
                                    Message::config(config::Event::ShowUnchangedGames),
                                ))
                                .push(checkbox(
                                    TRANSLATOR.show_unscanned_games(),
                                    config.scan.show_unscanned_games,
                                    Message::config(config::Event::ShowUnscannedGames),
                                ))
                                .push(checkbox(
                                    TRANSLATOR.field(&TRANSLATOR.explanation_for_exclude_cloud_games()),
                                    config.backup.filter.cloud.exclude,
                                    Message::config(move |exclude| {
                                        config::Event::CloudFilter(CloudFilter {
                                            exclude,
                                            ..config.backup.filter.cloud
                                        })
                                    }),
                                ))
                                .push(
                                    Row::new()
                                        .padding(padding::left(35))
                                        .spacing(10)
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Epic),
                                                config.backup.filter.cloud.epic,
                                                Message::config(move |epic| {
                                                    config::Event::CloudFilter(CloudFilter {
                                                        epic,
                                                        ..config.backup.filter.cloud
                                                    })
                                                }),
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Gog),
                                                config.backup.filter.cloud.gog,
                                                Message::config(move |gog| {
                                                    config::Event::CloudFilter(CloudFilter {
                                                        gog,
                                                        ..config.backup.filter.cloud
                                                    })
                                                }),
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                format!(
                                                    "{} / {}",
                                                    TRANSLATOR.store(&Store::Origin),
                                                    TRANSLATOR.store(&Store::Ea)
                                                ),
                                                config.backup.filter.cloud.origin,
                                                Message::config(move |origin| {
                                                    config::Event::CloudFilter(CloudFilter {
                                                        origin,
                                                        ..config.backup.filter.cloud
                                                    })
                                                }),
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Steam),
                                                config.backup.filter.cloud.steam,
                                                Message::config(move |steam| {
                                                    config::Event::CloudFilter(CloudFilter {
                                                        steam,
                                                        ..config.backup.filter.cloud
                                                    })
                                                }),
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Uplay),
                                                config.backup.filter.cloud.uplay,
                                                Message::config(move |uplay| {
                                                    config::Event::CloudFilter(CloudFilter {
                                                        uplay,
                                                        ..config.backup.filter.cloud
                                                    })
                                                }),
                                            )
                                            .class(style::Checkbox),
                                        ),
                                ),
                        )
                        .class(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new().spacing(5).push(text(TRANSLATOR.backup_field())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(10)
                                .push(
                                    Row::new()
                                        .spacing(20)
                                        .height(30)
                                        .align_y(Alignment::Center)
                                        .push({
                                            number_input(
                                                config.backup.retention.full as i32,
                                                TRANSLATOR.full_retention(),
                                                1..=255,
                                                Message::config(|x| config::Event::FullRetention(x as u8)),
                                            )
                                        })
                                        .push({
                                            number_input(
                                                config.backup.retention.differential as i32,
                                                TRANSLATOR.differential_retention(),
                                                0..=255,
                                                Message::config(|x| config::Event::DiffRetention(x as u8)),
                                            )
                                        }),
                                )
                                .push(
                                    Row::new()
                                        .spacing(20)
                                        .align_y(Alignment::Center)
                                        .push(
                                            Row::new()
                                                .spacing(5)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.backup_format_field()))
                                                .push(
                                                    pick_list(
                                                        BackupFormat::ALL,
                                                        Some(config.backup.format.chosen),
                                                        Message::config(config::Event::BackupFormat),
                                                    )
                                                    .class(style::PickList::Primary),
                                                ),
                                        )
                                        .push_if(config.backup.format.chosen == BackupFormat::Zip, || {
                                            Row::new()
                                                .spacing(5)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.backup_compression_field()))
                                                .push(
                                                    pick_list(
                                                        ZipCompression::ALL,
                                                        Some(config.backup.format.zip.compression),
                                                        Message::config(config::Event::BackupCompression),
                                                    )
                                                    .class(style::PickList::Primary),
                                                )
                                        })
                                        .push(match (config.backup.format.level(), config.backup.format.range()) {
                                            (Some(level), Some(range)) => Some(number_input(
                                                level,
                                                TRANSLATOR.backup_compression_level_field(),
                                                range,
                                                Message::config(config::Event::CompressionLevel),
                                            )),
                                            _ => None,
                                        }),
                                )
                                .push(Row::new().spacing(5).align_y(Alignment::Center).push(checkbox(
                                    TRANSLATOR.skip_unconstructive_backups(),
                                    config.backup.only_constructive,
                                    Message::config(config::Event::OnlyConstructiveBackups),
                                ))),
                        )
                        .class(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_y(iced::Alignment::Center)
                                .push(text(TRANSLATOR.manifest_label()).width(100))
                                .push(button::refresh(
                                    Message::UpdateManifest { force: true },
                                    updating_manifest,
                                )),
                        )
                        .push(editor::manifest(config, cache, histories, modifiers).padding(padding::top(10))),
                )
                .push(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_y(iced::Alignment::Center)
                                .push(text(TRANSLATOR.cloud_field()).width(100)),
                        )
                        .push(
                            Container::new({
                                let mut column = Column::new().spacing(5).push(
                                    Row::new()
                                        .spacing(20)
                                        .align_y(Alignment::Center)
                                        .push(text(TRANSLATOR.rclone_label()).width(70))
                                        .push(histories.input(UndoSubject::RcloneExecutable))
                                        .push_if(!is_rclone_valid, || {
                                            Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                                        })
                                        .push(button::choose_file(BrowseFileSubject::RcloneExecutable, modifiers))
                                        .push(histories.input(UndoSubject::RcloneArguments)),
                                );

                                if is_rclone_valid {
                                    let choice: RemoteChoice = config.cloud.remote.as_ref().into();
                                    column = column
                                        .push({
                                            let mut row = Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.remote_label()).width(70))
                                                .push_if(!operation.idle(), || {
                                                    text(choice.to_string())
                                                        .height(30)
                                                        .align_y(iced::alignment::Vertical::Center)
                                                })
                                                .push_if(operation.idle(), || {
                                                    pick_list(
                                                        RemoteChoice::ALL,
                                                        Some(choice),
                                                        Message::EditedCloudRemote,
                                                    )
                                                });

                                            if let Some(Remote::Custom { .. }) = &config.cloud.remote {
                                                row = row
                                                    .push(text(TRANSLATOR.remote_name_label()))
                                                    .push(histories.input(UndoSubject::CloudRemoteId));
                                            }

                                            if let Some(description) =
                                                config.cloud.remote.as_ref().and_then(|x| x.description())
                                            {
                                                row = row.push(text(description));
                                            }

                                            row
                                        })
                                        .push_if(choice != RemoteChoice::None, || {
                                            Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.folder_label()).width(70))
                                                .push(histories.input(UndoSubject::CloudPath))
                                                .push_if(!is_cloud_path_valid, || {
                                                    Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                                                })
                                        })
                                        .push_if(is_cloud_configured && is_cloud_path_valid, || {
                                            Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(button::upload(operation))
                                                .push(button::download(operation))
                                                .push(checkbox(
                                                    TRANSLATOR.synchronize_automatically(),
                                                    config.cloud.synchronize,
                                                    Message::config(|_| config::Event::ToggleCloudSynchronize),
                                                ))
                                        })
                                        .push_if(!is_cloud_configured, || text(TRANSLATOR.cloud_not_configured()))
                                        .push_if(!is_cloud_path_valid, || {
                                            text(TRANSLATOR.prefix_warning(&TRANSLATOR.cloud_path_invalid()))
                                                .class(style::Text::Failure)
                                        });
                                } else {
                                    column = column
                                        .push(
                                            text(TRANSLATOR.prefix_warning(&TRANSLATOR.rclone_unavailable()))
                                                .class(style::Text::Failure),
                                        )
                                        .push(button::open_url(TRANSLATOR.get_rclone_button(), RCLONE_URL.to_string()));
                                }

                                column
                            })
                            .padding(5)
                            .class(style::Container::GameListEntry),
                        ),
                )
                .push(
                    Column::new().spacing(5).push(text(TRANSLATOR.roots_label())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(4)
                                .push(editor::root(config, histories, modifiers)),
                        )
                        .class(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new()
                        .push(text(TRANSLATOR.ignored_items_label()))
                        .push(editor::ignored_items(config, histories, modifiers).padding(padding::top(10))),
                )
                .push(
                    Column::new()
                        .push(text(TRANSLATOR.redirects_label()))
                        .push(editor::redirect(config, histories, modifiers).padding(padding::top(10))),
                );
            ScrollSubject::Other.into_widget(content)
        });

    template(content)
}
