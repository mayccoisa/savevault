use std::collections::HashSet;

use iced::{Alignment, Length, keyboard, padding};

use crate::{
    cloud::{Remote, RemoteChoice},
    gui::{
        badge::Badge,
        button,
        common::{BrowseFileSubject, BrowseSubject, Message, Operation, ScrollSubject, UndoSubject},
        design, editor, emulator_art, font,
        game_list::GameList,
        icon::Icon,
        search::CustomGamesFilter,
        shortcuts::TextHistories,
        style, vault,
        widget::{
            Column, Container, Element, IcedParentExt, Row, checkbox, field_label, number_input, pick_list, text,
        },
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

/// A largura em que a linha de explicação para de crescer.
///
/// ~72 caracteres no corpo de 14px, que é o topo da faixa em que o olho ainda encontra o começo
/// da linha seguinte sem se perder. Sem isto a frase acompanha a janela: num monitor de 1920 ela
/// vira uma linha de quase 120 caracteres.
const EXPLANATION_WIDTH: f32 = 560.0;

const RCLONE_URL: &str = "https://rclone.org/downloads";
const RELEASE_URL: &str = "https://github.com/mtkennerly/ludusavi/releases";

fn template(content: Column) -> Element {
    // A coluna precisa declarar a largura: sem isto ela fica com a largura natural do que
    // ha dentro, e o que e Fill la dentro passa por cima da borda direita da janela.
    Container::new(
        content
            .width(Length::Fill)
            .spacing(design::space::MD)
            .align_x(Alignment::Center),
    )
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
        .padding([0.0, design::space::XL])
        .spacing(design::space::SM)
        .align_y(Alignment::Center)
        .push(text(summary).size(design::text::CAPTION).class(style::Text::Muted))
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
fn target_chip<'a>(label: String, path: &StrictPath, scan_kind: ScanKind, operation: &Operation) -> Element<'a> {
    let rendered = path.render();
    let leaf = rendered
        .replace('\\', "/")
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(&rendered)
        .to_string();

    button::secondary(
        format!("{label} {leaf}"),
        operation.idle().then_some(Message::ToggleTargetEditor { scan_kind }),
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
            .padding([0.0, design::space::XL])
            .spacing(design::space::SM)
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
            .spacing(design::space::SM)
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
    /// O que está guardado no cofre, do backup mais antigo para o mais novo.
    ///
    /// Lido do próprio cofre ao entrar na tela, e não do cache da última varredura. É o que faz a
    /// Restauração abrir respondendo "o que eu tenho e de quando é" em vez de uma lista de nomes
    /// sem dado nenhum esperando alguém apertar Escanear.
    pub inventory: Vec<vault::Game>,
}

impl Restore {
    const SCAN_KIND: ScanKind = ScanKind::Restore;

    /// A lista NÃO é mais semeada com o cache da última varredura.
    ///
    /// Aquilo punha nomes na tela sem dado nenhum atrás deles, e de quebra fazia a tela parecer
    /// varrida quando não estava. Ela nasce vazia agora, e quem ocupa o lugar é o acervo lido do
    /// cofre — que é a resposta certa para quem abre a Restauração.
    ///
    /// O acervo é lido AQUI e também ao entrar na tela. Só ao entrar não basta: quem abre o app
    /// direto na Restauração nunca passa pela troca de tela, e veria "nenhum jogo tem backup" com
    /// o cofre cheio — uma resposta errada, não uma tela vazia.
    pub fn new(config: &Config, _cache: &Cache) -> Self {
        Self {
            inventory: vault::games(config),
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
            .spacing(design::space::SM)
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
            // Antes de varrer, a tela mostra o ACERVO. Varrer troca para a lista operacional, que é
            // a que sabe restaurar; o acervo só responde o que existe guardado, e é ele que faz a
            // tela abrir com conteúdo em vez de esperar alguém apertar Escanear.
            .push_if(self.log.entries.is_empty(), || self.view_inventory())
            .push_if(!self.log.entries.is_empty(), || {
                self.log.view(
                    Self::SCAN_KIND,
                    config,
                    manifest,
                    &self.duplicate_detector,
                    duplicatees.as_ref(),
                    operation,
                    histories,
                    modifiers,
                )
            });

        template(content)
    }

    /// O acervo: um card por jogo guardado, do backup mais antigo para o mais novo.
    fn view_inventory(&self) -> Element<'_> {
        if self.inventory.is_empty() {
            return Container::new(text(TRANSLATOR.vault_empty()).size(design::text::BODY))
                .padding(design::space::XXL)
                .width(Length::Fill)
                .into();
        }

        let mut content = Column::new()
            .width(Length::Fill)
            .spacing(design::space::MD)
            .padding([0.0, design::space::XL])
            .push(
                text(TRANSLATOR.vault_stored_games(self.inventory.len()))
                    .size(design::text::CAPTION)
                    .class(style::Text::Muted),
            );

        for game in &self.inventory {
            content = content.push(
                Container::new(
                    Row::new()
                        .spacing(design::space::MD)
                        .align_y(Alignment::Center)
                        .push(emulator_art::game_tile(game))
                        .push(
                            Column::new()
                                .spacing(design::space::XS)
                                .width(Length::Fill)
                                .push(
                                    text(game.name.clone())
                                        .font(font::TEXT_STRONG)
                                        .size(design::text::SUBTITLE)
                                        .line_height(design::leading::TITLE),
                                )
                                .push(game.emulator.clone().map(|emulator| {
                                    text(emulator).size(design::text::CAPTION).class(style::Text::Muted)
                                })),
                        )
                        .push(
                            Column::new()
                                .spacing(design::space::XS)
                                .align_x(Alignment::End)
                                .push(text(game.last_backup.format("%Y-%m-%d %H:%M").to_string()))
                                .push(
                                    text(format!("{} · {}", game.files, TRANSLATOR.adjusted_size(game.bytes)))
                                        .size(design::text::CAPTION)
                                        .class(style::Text::Muted),
                                ),
                        ),
                )
                .padding(design::space::MD)
                .width(Length::Fill)
                .class(style::Container::Card),
            );
        }

        ScrollSubject::Restore.into_widget(content).into()
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

    /// O estado de um emulador, em uma palavra.
    ///
    /// Existe para a frase inteira deixar de ser a única informação do card. Oito frases de uma
    /// linha, todas no mesmo corpo e no mesmo cinza, é o que fazia a tela não ter onde o olho
    /// pousar: para saber quantos emuladores o computador tem, era preciso ler as oito.
    fn state_of(&self, app: crate::scan::emulator::App) -> EmulatorState {
        let Some(found) = self
            .diagnosis
            .as_ref()
            .and_then(|d| d.emulators.iter().find(|x| x.name == app.name()))
        else {
            return EmulatorState::Unchecked;
        };

        match &found.data_root {
            // "Usando <pasta> (0 arquivos de save)" é a frase que enganou na prática: ela soa como
            // sucesso. Pasta encontrada e vazia é caso próprio, com instrução.
            Some(_) if found.games.is_empty() => EmulatorState::NoSaves,
            Some(_) => EmulatorState::Ready,
            None => {
                let matching = found.candidates.iter().filter(|x| x.matches_signature).count();
                if matching > 1 {
                    EmulatorState::Ambiguous
                } else {
                    EmulatorState::Missing
                }
            }
        }
    }

    /// A frase que explica o estado, quando ela diz mais do que a palavra do chip já disse.
    ///
    /// `None` não é omissão: "DuckStation não foi encontrado neste computador" ao lado de um chip
    /// escrito "Não encontrado" é a mesma informação duas vezes, e era ela que ocupava metade da
    /// tela. Some quando o chip basta, fica quando carrega a pasta, a contagem ou a instrução.
    fn detail_of(&self, app: crate::scan::emulator::App) -> Option<String> {
        let found = self
            .diagnosis
            .as_ref()
            .and_then(|d| d.emulators.iter().find(|x| x.name == app.name()))?;

        match &found.data_root {
            Some(root) if found.games.is_empty() => Some(TRANSLATOR.emulator_using_folder_without_saves(root)),
            Some(root) => Some(TRANSLATOR.emulator_using_folder(root, found.games.len())),
            None => {
                let matching = found.candidates.iter().filter(|x| x.matches_signature).count();
                (matching > 1).then(|| TRANSLATOR.emulator_ambiguous(app.name(), matching))
            }
        }
    }

    /// Traduz o veredito do motor sobre uma pasta. O motor devolve fato; o texto é daqui.
    fn verdict_for<'a>(&self, app: crate::scan::emulator::App, path: &crate::prelude::StrictPath) -> Element<'a> {
        use crate::scan::emulator::FolderVerdict;

        let (label, ok) = match app.inspect_folder(path) {
            FolderVerdict::Empty => (TRANSLATOR.emulator_folder_empty(), false),
            FolderVerdict::Missing => (TRANSLATOR.emulator_folder_missing(), false),
            FolderVerdict::NotThisEmulator { missing } => {
                (TRANSLATOR.emulator_folder_wrong(app.name(), &missing.join(", ")), false)
            }
            FolderVerdict::NoSaves { areas } => (TRANSLATOR.emulator_folder_without_saves(&areas.join(", ")), false),
            FolderVerdict::Ready { saves } => (TRANSLATOR.emulator_folder_ready(saves), true),
        };

        text(label)
            .size(design::text::CAPTION)
            .line_height(design::leading::BODY)
            .class(if ok { style::Text::Default } else { style::Text::Muted })
            .into()
    }

    /// Um card de emulador.
    ///
    /// A hierarquia é nome (16 semibold) → estado (chip) → console (12 apagado) → detalhe, e é
    /// deliberado que o tamanho não faça o trabalho sozinho: antes o nome era corpo 20, o mesmo do
    /// título da tela, e por isso oito cards gritavam no mesmo tom que o cabeçalho.
    fn card<'a>(
        &'a self,
        app: crate::scan::emulator::App,
        config: &'a Config,
        histories: &'a TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        let state = self.state_of(app);

        let mut body = Column::new().spacing(design::space::MD).push(
            Row::new()
                .spacing(design::space::MD)
                .align_y(Alignment::Center)
                .push(emulator_art::tile(app))
                .push(
                    Column::new()
                        .spacing(design::space::XS)
                        .width(Length::Fill)
                        .push(
                            text(app.name())
                                .font(font::TEXT_STRONG)
                                .size(design::text::SUBTITLE)
                                .line_height(design::leading::TITLE),
                        )
                        // O console é o que faz a lista ser legível: quem procura o save sabe que
                        // jogou um jogo de PS2, não que o PCSX2 é o que roda PS2.
                        .push(
                            text(emulator_art::console(app))
                                .size(design::text::CAPTION)
                                .class(style::Text::Muted),
                        ),
                )
                .push(chip(state.label(), state.is_good())),
        );

        if let Some(detail) = self.detail_of(app) {
            body = body.push(
                text(detail)
                    .size(design::text::BODY)
                    .line_height(design::leading::BODY)
                    .class(style::Text::Muted),
            );
        }

        // As raízes deste emulador especificamente. É isto que faz "todas as configurações ficarem
        // dentro do emulador": o usuário não escolhe loja num menu solto.
        for (index, root) in config.roots.iter().enumerate().filter(
            |(_, root)| matches!(root, crate::resource::config::Root::Emulator(emulator) if emulator.app == Some(app)),
        ) {
            body = body.push(
                Column::new()
                    .spacing(design::space::XS)
                    .push(
                        Row::new()
                            .spacing(design::space::SM)
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
                    // enquanto a instalação de verdade está noutro disco) parecia estar certa, e o
                    // backup vinha vazio sem explicação.
                    .push(self.verdict_for(app, root.path())),
            );
        }

        Container::new(body.push(button::add_emulator_root(app)))
            .padding(design::space::LG)
            .width(Length::Fill)
            .class(style::Container::Card)
            .into()
    }

    /// The command of the screen, mounted in the title bar with the others.
    pub fn commands<'a>(&'a self) -> Row<'a> {
        Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(button::refresh_emulators())
    }

    pub fn view<'a>(
        &'a self,
        config: &'a Config,
        histories: &'a TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        use crate::scan::emulator::App;

        let found = App::ALL.iter().filter(|app| self.state_of(**app).is_good()).count();

        // A ação foi para a barra de título, junto com as das outras telas. Só o resumo fica aqui.
        let header = Column::new()
            .width(Length::Fill)
            .spacing(design::space::XS)
            .push(
                text(TRANSLATOR.emulators_summary(found, App::ALL.len()))
                    .font(font::TEXT_STRONG)
                    .size(design::text::SUBTITLE)
                    .line_height(design::leading::TITLE),
            )
            // Largura travada: a linha ocupava a janela inteira, o que num monitor de 1920 dá quase
            // 120 caracteres, muito acima dos 75 em que o olho ainda acha o começo da linha
            // seguinte.
            .push(
                text(TRANSLATOR.emulators_explanation())
                    .size(design::text::BODY)
                    .line_height(design::leading::BODY)
                    .class(style::Text::Muted)
                    .width(Length::Fixed(EXPLANATION_WIDTH)),
            );

        // Duas colunas, fixas. A janela mínima do app tem 1036 de largura, e a máxima que cabe num
        // monitor comum passa pouco de 1600: entre as duas, dois cards é sempre o que a largura
        // comporta, então medir a janela em tempo de layout não decidiria nada e custaria um
        // widget a mais para dar errado.
        //
        // O que os oito cards NÃO podem voltar a ser é uma coluna só, que era o desenho anterior:
        // desperdiçava dois terços de um monitor, e como cada bloco tinha a largura do próprio
        // texto, nenhum deles alinhava com o de cima.
        const COLUMNS: usize = 2;

        let mut content = Column::new()
            .width(Length::Fill)
            .spacing(design::space::LG)
            .push(header);

        for chunk in App::ALL.chunks(COLUMNS) {
            let mut row = Row::new()
                .width(Length::Fill)
                .spacing(design::space::LG)
                .align_y(Alignment::Start);
            for app in chunk {
                row = row.push(self.card(*app, config, histories, modifiers));
            }
            // A fileira incompleta ganha vãos vazios, senão os cards dela esticam para ocupar a
            // linha e param de ter a mesma largura dos de cima.
            for _ in chunk.len()..COLUMNS {
                row = row.push(iced::widget::space().width(Length::Fill));
            }
            content = content.push(row);
        }

        for planned in App::PLANNED {
            content = content.push(
                Container::new(
                    Row::new()
                        .spacing(design::space::MD)
                        .align_y(Alignment::Center)
                        .push(text(*planned).font(font::TEXT_STRONG).size(design::text::SUBTITLE))
                        .push(Badge::new(&TRANSLATOR.emulator_coming_soon()).view()),
                )
                .padding(design::space::LG)
                .width(Length::Fill)
                .class(style::Container::Card),
            );
        }

        template(Column::new().push(ScrollSubject::Emulators.into_widget(content.padding([0.0, design::space::XL]))))
    }
}

/// O estado de um emulador nesta máquina, resumido em uma palavra para o chip do card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmulatorState {
    /// Pasta de dados encontrada, com save dentro.
    Ready,
    /// Pasta encontrada, e vazia. Não é sucesso: o backup sairia sem nada.
    NoSaves,
    /// Mais de uma pasta de dados casou, então o destino da restauração fica indefinido.
    Ambiguous,
    /// Nenhuma pasta de dados deste emulador neste computador.
    Missing,
    /// Ainda não foi olhado o disco.
    Unchecked,
}

impl EmulatorState {
    fn label(self) -> String {
        match self {
            Self::Ready => TRANSLATOR.emulator_state_ready(),
            Self::NoSaves => TRANSLATOR.emulator_state_no_saves(),
            Self::Ambiguous => TRANSLATOR.emulator_state_ambiguous(),
            Self::Missing => TRANSLATOR.emulator_state_missing(),
            Self::Unchecked => TRANSLATOR.emulator_state_unchecked(),
        }
    }

    /// Só `Ready` conta como bom, e é o que a contagem do cabeçalho usa.
    ///
    /// `NoSaves` fica de fora de propósito: pasta reconhecida e vazia é exatamente o caso que
    /// parecia sucesso e produzia backup vazio sem explicação.
    fn is_good(self) -> bool {
        self == Self::Ready
    }
}

/// Uma etiqueta de estado: uma palavra, e a cor como reforço, nunca como o único sinal.
fn chip<'a>(label: String, positive: bool) -> Element<'a> {
    Container::new(
        text(label)
            .size(design::text::CAPTION)
            .line_height(design::leading::TIGHT),
    )
    .padding([design::space::XS, design::space::SM])
    .class(style::Container::Chip { positive })
    .into()
}

#[derive(Default)]
pub struct CustomGames {
    pub filter: CustomGamesFilter,
}

impl CustomGames {
    /// The commands of the screen, mounted in the title bar.
    ///
    /// They lived in a strip of their own below the title, as four filled accent buttons side by
    /// side — four primary actions, when only one of them (adding a game) leads the screen. The bar
    /// above already reserved the space and was carrying a single word, which is the same reason
    /// Backup and Restore put theirs there.
    pub fn commands<'a>(&'a self, config: &Config) -> Row<'a> {
        Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(button::add_game())
            .push(button::toggle_all_custom_games(
                self.all_visible_game_selected(config),
                self.is_filtered(),
            ))
            .push(button::sort(config::Event::SortCustomGames))
            .push(button::filter(self.filter.enabled))
    }

    pub fn view<'a>(
        &'a self,
        config: &Config,
        manifest: &Manifest,
        operating: bool,
        histories: &'a TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        let content = Column::new()
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

/// The width the settings form stops growing at.
///
/// A form is read down a single column, and a control that starts 900px away from its label has
/// stopped being labelled. The old screen had no limit at all, so on a wide monitor the picker for
/// the language sat alone in the top-left corner of a very large empty area.
const FORM_WIDTH: f32 = 760.0;

/// The width a picker gets in the settings form.
///
/// Common to all of them, so the column of controls has a straight right edge. Left to size
/// themselves, "Escuro" and "Português brasileiro (100%)" end in very different places and the
/// form looks ragged.
const FIELD_CONTROL_WIDTH: f32 = 280.0;

/// The label column of a settings row.
///
/// Fixed, so every control in the form starts at the same x. Ragged labels each pushing their own
/// control to a different place is what makes a form read as a dump of fields rather than a list of
/// choices.
const FIELD_LABEL_WIDTH: f32 = 176.0;

/// One block of settings: a card, its heading, and the rows inside it.
///
/// This replaces the old shape, which was a line of body text ending in a colon followed by a
/// bordered box. The heading was the same size and weight as the labels underneath it, so the
/// screen had no levels: nine groups and nine headings that did not look like headings.
fn settings_section<'a>(title: String, body: Column<'a>) -> Element<'a> {
    // A heading does not point at anything either, and several of these titles come from the same
    // colon-bearing strings the fields use.
    let title = field_label(title);

    Container::new(
        Column::new()
            .spacing(design::space::LG)
            .push(
                text(title)
                    .font(font::TEXT_STRONG)
                    .size(design::text::SUBTITLE)
                    .line_height(design::leading::TITLE),
            )
            .push(body.spacing(design::space::MD).width(Length::Fill)),
    )
    .padding(design::space::LG)
    .width(Length::Fill)
    .class(style::Container::Card)
    .into()
}

/// A labelled row inside a section.
fn settings_field<'a>(label: String, control: impl Into<Element<'a>>) -> Row<'a> {
    let label = field_label(label);

    Row::new()
        .spacing(design::space::LG)
        .align_y(Alignment::Center)
        .push(Container::new(text(label)).width(Length::Fixed(FIELD_LABEL_WIDTH)))
        .push(control)
}

/// A row of settings that is a sentence with a checkbox, not a label and a control.
fn settings_toggle<'a>(control: impl Into<Element<'a>>) -> Row<'a> {
    Row::new()
        .spacing(design::space::SM)
        .align_y(Alignment::Center)
        .push(control)
}

/// Options that only apply while the checkbox above them is on.
///
/// Indented, because the relationship is the whole point: without it, "Epic / GOG / Steam" reads as
/// five more settings rather than as the detail of the one above.
fn settings_nested<'a>(content: impl Into<Element<'a>>) -> Container<'a> {
    Container::new(content).padding(padding::left(design::space::XL))
}

pub fn other<'a>(
    updating_manifest: bool,
    config: &'a Config,
    cache: &'a Cache,
    operation: &'a Operation,
    histories: &'a TextHistories,
    modifiers: &keyboard::Modifiers,
) -> Element<'a> {
    let is_rclone_valid = config.apps.rclone.is_valid();
    let is_cloud_configured = config.cloud.remote.is_some();
    let is_cloud_path_valid = crate::cloud::validate_cloud_path(&config.cloud.path).is_ok();

    let appearance = settings_section(
        TRANSLATOR.settings_appearance(),
        Column::new()
            .push(settings_field(
                TRANSLATOR.language_label(),
                pick_list(
                    Language::ALL,
                    Some(config.language),
                    Message::config(config::Event::Language),
                )
                .width(Length::Fixed(FIELD_CONTROL_WIDTH))
                .class(style::PickList::Primary),
            ))
            .push(settings_field(
                TRANSLATOR.theme_label(),
                pick_list(Theme::ALL, Some(config.theme), Message::config(config::Event::Theme))
                    .width(Length::Fixed(FIELD_CONTROL_WIDTH))
                    .class(style::PickList::Primary),
            ))
            .push(settings_field(
                TRANSLATOR.accent_label(),
                pick_list(Accent::ALL, Some(config.accent), Message::config(config::Event::Accent))
                    .width(Length::Fixed(FIELD_CONTROL_WIDTH))
                    .class(style::PickList::Primary),
            )),
    );

    let updates = settings_section(
        TRANSLATOR.settings_updates(),
        Column::new().push(
            settings_toggle(checkbox(
                TRANSLATOR.new_version_check(),
                config.release.check,
                Message::config(config::Event::CheckRelease),
            ))
            .push(iced::widget::space().width(Length::Fill))
            .push(button::open_url_icon(RELEASE_URL.to_string())),
        ),
    );

    let scanning = settings_section(
        TRANSLATOR.scan_label(),
        Column::new()
            .push_if(AVAILABLE_PARALELLISM.is_some(), || {
                Column::new()
                    .spacing(design::space::SM)
                    .push(settings_toggle(checkbox(
                        TRANSLATOR.override_max_threads(),
                        config.runtime.threads.is_some(),
                        Message::config(config::Event::OverrideMaxThreads),
                    )))
                    .push(
                        config
                            .runtime
                            .threads
                            .zip(*AVAILABLE_PARALELLISM)
                            .map(|(threads, max_threads)| {
                                settings_nested(number_input(
                                    threads.get() as i32,
                                    TRANSLATOR.threads_label(),
                                    1..=(max_threads.get() as i32),
                                    Message::config(|x| config::Event::MaxThreads(x as usize)),
                                ))
                            }),
                    )
            })
            .push(settings_toggle(
                checkbox(
                    TRANSLATOR.explanation_for_exclude_store_screenshots(),
                    config.backup.filter.exclude_store_screenshots,
                    Message::config(config::Event::ExcludeStoreScreenshots),
                )
                .class(style::Checkbox),
            ))
            .push(settings_toggle(checkbox(
                TRANSLATOR.show_disabled_games(),
                config.scan.show_deselected_games,
                Message::config(config::Event::ShowDeselectedGames),
            )))
            .push(settings_toggle(checkbox(
                TRANSLATOR.show_unchanged_games(),
                config.scan.show_unchanged_games,
                Message::config(config::Event::ShowUnchangedGames),
            )))
            .push(settings_toggle(checkbox(
                TRANSLATOR.show_unscanned_games(),
                config.scan.show_unscanned_games,
                Message::config(config::Event::ShowUnscannedGames),
            )))
            .push(settings_toggle(checkbox(
                TRANSLATOR.field(&TRANSLATOR.explanation_for_exclude_cloud_games()),
                config.backup.filter.cloud.exclude,
                Message::config(move |exclude| {
                    config::Event::CloudFilter(CloudFilter {
                        exclude,
                        ..config.backup.filter.cloud
                    })
                }),
            )))
            .push(settings_nested(
                Row::new()
                    .spacing(design::space::MD)
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
            )),
    );

    let backup = settings_section(
        TRANSLATOR.backup_label(),
        Column::new()
            .push(settings_field(
                TRANSLATOR.full_retention(),
                number_input(
                    config.backup.retention.full as i32,
                    String::new(),
                    1..=255,
                    Message::config(|x| config::Event::FullRetention(x as u8)),
                ),
            ))
            .push(settings_field(
                TRANSLATOR.differential_retention(),
                number_input(
                    config.backup.retention.differential as i32,
                    String::new(),
                    0..=255,
                    Message::config(|x| config::Event::DiffRetention(x as u8)),
                ),
            ))
            .push(settings_field(
                TRANSLATOR.backup_format_field(),
                pick_list(
                    BackupFormat::ALL,
                    Some(config.backup.format.chosen),
                    Message::config(config::Event::BackupFormat),
                )
                .class(style::PickList::Primary),
            ))
            .push_if(config.backup.format.chosen == BackupFormat::Zip, || {
                settings_field(
                    TRANSLATOR.backup_compression_field(),
                    pick_list(
                        ZipCompression::ALL,
                        Some(config.backup.format.zip.compression),
                        Message::config(config::Event::BackupCompression),
                    )
                    .class(style::PickList::Primary),
                )
            })
            .push(match (config.backup.format.level(), config.backup.format.range()) {
                (Some(level), Some(range)) => Some(settings_field(
                    TRANSLATOR.backup_compression_level_field(),
                    number_input(
                        level,
                        String::new(),
                        range,
                        Message::config(config::Event::CompressionLevel),
                    ),
                )),
                _ => None,
            })
            .push(settings_toggle(checkbox(
                TRANSLATOR.skip_unconstructive_backups(),
                config.backup.only_constructive,
                Message::config(config::Event::OnlyConstructiveBackups),
            ))),
    );

    let manifest = settings_section(
        TRANSLATOR.manifest_label_bare(),
        Column::new()
            .push(
                Row::new()
                    .align_y(Alignment::Center)
                    .push(iced::widget::space().width(Length::Fill))
                    .push(button::refresh(
                        Message::UpdateManifest { force: true },
                        updating_manifest,
                    )),
            )
            .push(editor::manifest(config, cache, histories, modifiers)),
    );

    let cloud = settings_section(TRANSLATOR.cloud_label(), {
        // The executable and its arguments are two rows, not one. Together they were a label, a
        // path field, an error icon, a browse button and a second field on a single line, and the
        // last one ran off the edge of the card.
        let mut column = Column::new()
            .push(
                settings_field(
                    TRANSLATOR.rclone_label(),
                    histories.input(UndoSubject::RcloneExecutable),
                )
                .push_if(!is_rclone_valid, || {
                    Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                })
                .push(button::choose_file(BrowseFileSubject::RcloneExecutable, modifiers)),
            )
            .push(settings_field(
                TRANSLATOR.arguments_label(),
                histories.input(UndoSubject::RcloneArguments),
            ));

        if is_rclone_valid {
            let choice: RemoteChoice = config.cloud.remote.as_ref().into();
            column = column
                .push({
                    let mut row = settings_field(
                        TRANSLATOR.remote_label(),
                        Container::new(
                            Row::new()
                                .align_y(Alignment::Center)
                                .push_if(!operation.idle(), || text(choice.to_string()))
                                .push_if(operation.idle(), || {
                                    pick_list(RemoteChoice::ALL, Some(choice), Message::EditedCloudRemote)
                                }),
                        )
                        .height(design::control::HEIGHT)
                        .center_y(design::control::HEIGHT),
                    );

                    if let Some(Remote::Custom { .. }) = &config.cloud.remote {
                        row = row
                            .push(text(TRANSLATOR.remote_name_label()))
                            .push(histories.input(UndoSubject::CloudRemoteId));
                    }

                    if let Some(description) = config.cloud.remote.as_ref().and_then(|x| x.description()) {
                        row = row.push(text(description).class(style::Text::Muted));
                    }

                    row
                })
                .push_if(choice != RemoteChoice::None, || {
                    settings_field(TRANSLATOR.folder_label(), histories.input(UndoSubject::CloudPath))
                        .push_if(!is_cloud_path_valid, || {
                            Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                        })
                })
                .push_if(is_cloud_configured && is_cloud_path_valid, || {
                    Row::new()
                        .spacing(design::space::SM)
                        .align_y(Alignment::Center)
                        .push(button::upload(operation))
                        .push(button::download(operation))
                        .push(checkbox(
                            TRANSLATOR.synchronize_automatically(),
                            config.cloud.synchronize,
                            Message::config(|_| config::Event::ToggleCloudSynchronize),
                        ))
                })
                .push_if(!is_cloud_configured, || {
                    text(TRANSLATOR.cloud_not_configured()).class(style::Text::Muted)
                })
                .push_if(!is_cloud_path_valid, || {
                    text(TRANSLATOR.prefix_warning(&TRANSLATOR.cloud_path_invalid())).class(style::Text::Failure)
                });
        } else {
            column = column
                .push(text(TRANSLATOR.prefix_warning(&TRANSLATOR.rclone_unavailable())).class(style::Text::Failure))
                .push(Row::new().push(button::open_url(TRANSLATOR.get_rclone_button(), RCLONE_URL.to_string())));
        }

        column
    });

    let roots = settings_section(
        TRANSLATOR.roots_label(),
        Column::new().push(editor::root(config, histories, modifiers)),
    );

    let exclusions = settings_section(
        TRANSLATOR.ignored_items_label(),
        Column::new().push(editor::ignored_items(config, histories, modifiers)),
    );

    let redirects = settings_section(
        TRANSLATOR.redirects_label(),
        Column::new().push(editor::redirect(config, histories, modifiers)),
    );

    let form = Column::new()
        .width(Length::Fixed(FORM_WIDTH))
        .spacing(design::space::LG)
        .padding([0.0, design::space::XL])
        .push_if(*STEAM_DECK, || {
            Row::new()
                .width(Length::Fill)
                .push(iced::widget::space().width(Length::Fill))
                .push(button::negative(
                    TRANSLATOR.exit_button(),
                    Some(Message::Exit { user: true }),
                ))
        })
        .push(appearance)
        .push(updates)
        .push(scanning)
        .push(backup)
        .push(manifest)
        .push(cloud)
        .push(roots)
        .push(exclusions)
        .push(redirects);

    template(Column::new().push(ScrollSubject::Other.into_widget(form)))
}
