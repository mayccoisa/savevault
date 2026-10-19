//! Biblioteca visual. A identidade do backup permanece separada do título e da capa.
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use iced::{Alignment, Length, alignment};
use serde::{Deserialize, Serialize};

use crate::{
    gui::{
        button,
        common::{Message, Screen, ScrollSubject},
        design, font,
        icon::Icon,
        style, vault,
        widget::{Button, Column, Container, Element, Row, Stack, TextInput, Tooltip, pick_list, text},
    },
    resource::config::Config,
    scan::layout::{Backup, BackupLayout},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    Gallery,
    List,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    Recent,
    Name,
    Oldest,
}
impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Recent => "Mais recentes",
            Self::Name => "Nome do jogo",
            Self::Oldest => "Mais antigos",
        })
    }
}

/// Which platform the catalogue is narrowed to. Built from the games actually in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlatformFilter {
    #[default]
    All,
    Pc,
    Emulator(String),
}

impl std::fmt::Display for PlatformFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::All => "Todas as plataformas",
            Self::Pc => "PC",
            Self::Emulator(name) => name,
        })
    }
}

/// How old the newest backup is. Recent is inside [`design::library::RECENT_DAYS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Freshness {
    #[default]
    Any,
    Recent,
    Older,
}

impl std::fmt::Display for Freshness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Any => "Qualquer status",
            Self::Recent => "Backup recente",
            Self::Older => "Backup antigo",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailTab {
    #[default]
    Overview,
    Versions,
    Records,
}

/// The portrait and the 16:9 frame made from it. The frame is what the card and the hero show.
#[derive(Clone, Debug)]
pub struct LoadedCover {
    pub portrait: iced::widget::image::Handle,
    pub banner: iced::widget::image::Handle,
}

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    View(View),
    Sort(Sort),
    Platform(PlatformFilter),
    Freshness(Freshness),
    Tab(DetailTab),
    Open(String),
    Back,
    Refresh,
    Operational,
    ChooseCover(String),
    CoverChosen(String, Option<PathBuf>),
    CoverReady(String, Result<Option<LoadedCover>, String>),
    Preview(String, Backup),
    Backup(String),
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    view: View,
    sort: Sort,
}

#[derive(Default)]
pub struct Library {
    pub query: String,
    pub view: View,
    pub sort: Sort,
    pub platform: PlatformFilter,
    pub freshness: Freshness,
    pub selected: Option<String>,
    pub versions: Vec<Backup>,
    pub tab: DetailTab,
    pub operational: bool,
    pub covers: BTreeMap<String, iced::widget::image::Handle>,
    pub banners: BTreeMap<String, iced::widget::image::Handle>,
    pub requested: HashSet<String>,
    pub notice: Option<String>,
}

pub fn directory() -> PathBuf {
    PathBuf::from(crate::prelude::app_dir().render()).join("library")
}

impl Library {
    pub fn load() -> Self {
        let prefs: Preferences = std::fs::read(directory().join("preferences.json"))
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        Self {
            view: prefs.view,
            sort: prefs.sort,
            ..Self::default()
        }
    }

    pub fn save_preferences(&mut self) {
        let result = (|| -> Result<(), String> {
            std::fs::create_dir_all(directory()).map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec(&Preferences {
                view: self.view,
                sort: self.sort,
            })
            .map_err(|e| e.to_string())?;
            std::fs::write(directory().join("preferences.json"), bytes).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            self.notice = Some("Não foi possível salvar a preferência de visualização.".into());
        }
    }

    pub fn open(&mut self, name: String, config: &Config) {
        self.versions = BackupLayout::new(config.restore.path.clone())
            .try_game_layout(&name)
            .map(|layout| layout.restorable_backups_flattened())
            .unwrap_or_default();
        self.versions.sort_by_key(|backup| std::cmp::Reverse(*backup.when()));
        self.selected = Some(name);
        self.tab = DetailTab::Overview;
        self.notice = None;
    }

    pub fn filtered<'a>(&self, games: &'a [vault::Game]) -> Vec<&'a vault::Game> {
        let query = self.query.trim().to_lowercase();
        let cutoff = chrono::Local::now() - chrono::Duration::days(design::library::RECENT_DAYS);
        let mut result: Vec<_> = games
            .iter()
            .filter(|g| {
                let platform_ok = match &self.platform {
                    PlatformFilter::All => true,
                    PlatformFilter::Pc => g.emulator.is_none(),
                    PlatformFilter::Emulator(name) => g.emulator.as_deref() == Some(name),
                };
                let freshness_ok = match self.freshness {
                    Freshness::Any => true,
                    Freshness::Recent => g.last_backup >= cutoff,
                    Freshness::Older => g.last_backup < cutoff,
                };
                let query_ok = query.is_empty()
                    || g.title.to_lowercase().contains(&query)
                    || g.name.to_lowercase().contains(&query)
                    || g.emulator.as_deref().unwrap_or("PC").to_lowercase().contains(&query);
                platform_ok && freshness_ok && query_ok
            })
            .collect();
        result.sort_by(|a, b| {
            match self.sort {
                Sort::Recent => b.last_backup.cmp(&a.last_backup),
                Sort::Oldest => a.last_backup.cmp(&b.last_backup),
                Sort::Name => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            }
            .then_with(|| a.name.cmp(&b.name))
        });
        result
    }

    fn portrait<'a>(&'a self, game: &'a vault::Game, width: f32, height: f32, round_bottom: bool) -> Element<'a> {
        let radius = if round_bottom { design::RADIUS } else { 0.0 };
        if let Some(handle) = self.covers.get(&game.name) {
            iced::widget::image(handle.clone())
                .width(width)
                .height(height)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(iced::border::Radius {
                    top_left: design::RADIUS,
                    top_right: design::RADIUS,
                    bottom_left: radius,
                    bottom_right: radius,
                })
                .into()
        } else {
            Container::new(
                text(game.title.chars().next().unwrap_or('?').to_string())
                    .size(if height >= design::library::HERO_THUMB_HEIGHT {
                        design::text::SUBTITLE
                    } else {
                        design::text::BODY
                    })
                    .font(font::TEXT_STRONG),
            )
            .center_x(width)
            .center_y(height)
            .class(style::Container::Card)
            .into()
        }
    }

    /// The 16:9 frame. A portrait cover is not cropped: the frame is a blurred, darkened copy of
    /// the same image with the sharp cover standing in the middle. Built when the cover loads.
    fn banner<'a>(&'a self, game: &'a vault::Game, width: f32, height: f32) -> Element<'a> {
        let radius = iced::border::Radius {
            top_left: design::RADIUS,
            top_right: design::RADIUS,
            bottom_left: 0.0,
            bottom_right: 0.0,
        };
        let picture: Element<'a> = if let Some(handle) = self.banners.get(&game.name) {
            iced::widget::image(handle.clone())
                .width(width)
                .height(height)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(radius)
                .into()
        } else {
            Container::new(
                Column::new()
                    .spacing(design::space::XS)
                    .align_x(Alignment::Center)
                    .push(
                        text(game.title.chars().next().unwrap_or('?').to_string())
                            .size(design::text::SUBTITLE)
                            .font(font::TEXT_STRONG),
                    )
                    .push(text("Sem capa").size(design::text::CAPTION).class(style::Text::Muted)),
            )
            .center_x(width)
            .center_y(height)
            .class(style::Container::Card)
            .into()
        };
        Stack::new()
            .width(width)
            .height(height)
            .push(picture)
            .push(
                Container::new(chip(platform_name(game)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::End)
                    .align_y(alignment::Vertical::Top)
                    .padding(design::space::SM),
            )
            .into()
    }

    fn card<'a>(&'a self, game: &'a vault::Game) -> Element<'a> {
        let inner = design::library::CARD_WIDTH - 2.0 * design::library::CARD_PAD;
        let title = ellipsize(&game.title, design::text_budget(inner, design::text::SUBTITLE, 1));
        let body = Column::new()
            .width(design::library::CARD_WIDTH)
            .push(self.banner(game, design::library::CARD_WIDTH, design::library::BANNER_HEIGHT))
            .push(
                Container::new(
                    Column::new()
                        .spacing(design::space::XS)
                        .push(
                            text(platform_name(game).to_uppercase())
                                .size(design::text::CAPTION)
                                .class(style::Text::Muted),
                        )
                        .push(
                            Container::new(
                                text(title)
                                    .font(font::TEXT_STRONG)
                                    .size(design::text::SUBTITLE)
                                    .line_height(design::leading::TIGHT)
                                    .wrapping(iced::widget::text::Wrapping::None),
                            )
                            .width(Length::Fill)
                            .height(design::library::TITLE_BLOCK)
                            .clip(true),
                        )
                        .push(chip(version_count(game.backups)))
                        .push(
                            Row::new()
                                .width(Length::Fill)
                                .align_y(Alignment::Center)
                                .push(
                                    text("Último backup")
                                        .size(design::text::CAPTION)
                                        .class(style::Text::Muted),
                                )
                                .push(iced::widget::space().width(Length::Fill))
                                .push(
                                    text(game.last_backup.format("%d/%m/%Y").to_string())
                                        .size(design::text::CAPTION)
                                        .class(style::Text::Muted),
                                ),
                        ),
                )
                .width(Length::Fill)
                .padding(design::library::CARD_PAD),
            );
        Tooltip::new(
            Button::new(body)
                .padding(0)
                .width(design::library::CARD_WIDTH)
                .class(style::Button::LibraryCard)
                .on_press(Message::Library(Event::Open(game.name.clone()))),
            text(game.title.clone()),
            iced::widget::tooltip::Position::Top,
        )
        .into()
    }

    fn list_row<'a>(&'a self, game: &'a vault::Game) -> Element<'a> {
        Button::new(
            Row::new()
                .spacing(design::space::MD)
                .align_y(Alignment::Center)
                .push(self.portrait(game, design::library::THUMB_WIDTH, design::library::THUMB_HEIGHT, true))
                .push(
                    Column::new()
                        .spacing(design::space::XS)
                        .width(Length::Fill)
                        .push(
                            text(ellipsize(&game.title, 72))
                                .font(font::TEXT_STRONG)
                                .wrapping(iced::widget::text::Wrapping::Word),
                        )
                        .push(text(game.emulator.clone().unwrap_or_else(|| "PC".into())).class(style::Text::Muted)),
                )
                .push(text(version_count(game.backups)))
                .push(text(game.last_backup.format("%d/%m/%Y %H:%M").to_string()).class(style::Text::Muted)),
        )
        .padding(design::space::SM)
        .width(Length::Fill)
        .class(style::Button::LibraryCard)
        .on_press(Message::Library(Event::Open(game.name.clone())))
        .into()
    }

    pub fn view<'a>(&'a self, games: &'a [vault::Game], idle: bool) -> Element<'a> {
        if let Some(game) = self
            .selected
            .as_ref()
            .and_then(|name| games.iter().find(|g| &g.name == name))
        {
            return self.details(game, idle);
        }
        if games.is_empty() {
            return Container::new(
                Column::new()
                    .spacing(design::space::MD)
                    .push(
                        text("Seu progresso começa aqui")
                            .size(design::text::TITLE)
                            .font(font::TEXT_STRONG),
                    )
                    .push(text("Os jogos aparecem nesta biblioteca depois do primeiro backup."))
                    .push(button::primary(
                        "Fazer backup".into(),
                        idle.then_some(Message::SwitchScreen(Screen::Backup)),
                    )),
            )
            .padding(design::space::XXL)
            .into();
        }
        let filtered = self.filtered(games);
        let mut platforms = vec![PlatformFilter::All, PlatformFilter::Pc];
        let mut names: Vec<String> = games.iter().filter_map(|game| game.emulator.clone()).collect();
        names.sort();
        names.dedup();
        for name in names {
            platforms.push(PlatformFilter::Emulator(name));
        }
        let selected_platform = platforms
            .iter()
            .find(|item| *item == &self.platform)
            .cloned()
            .unwrap_or_default();
        let heading = Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(text("Biblioteca").size(design::text::SUBTITLE).font(font::TEXT_STRONG))
            .push(count_badge(games.len()))
            .push(iced::widget::space().width(Length::Fill))
            .push(pick_list(
                [Sort::Recent, Sort::Name, Sort::Oldest],
                Some(self.sort),
                |s| Message::Library(Event::Sort(s)),
            ));
        let filters = Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(
                TextInput::new("Buscar jogo ou emulador", &self.query)
                    .on_input(|q| Message::Library(Event::Search(q)))
                    .icon(search_icon())
                    .size(design::text::BODY)
                    .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                        design::control::LINE,
                    )))
                    .padding([design::control::PAD_Y, design::control::PAD_X])
                    .width(Length::Fixed(design::control::SEARCH_WIDTH)),
            )
            .push(
                pick_list(platforms, Some(selected_platform), |value| {
                    Message::Library(Event::Platform(value))
                })
                .width(Length::Fixed(200.0)),
            )
            .push(
                pick_list(
                    [Freshness::Any, Freshness::Recent, Freshness::Older],
                    Some(self.freshness),
                    |value| Message::Library(Event::Freshness(value)),
                )
                .width(Length::Fixed(180.0)),
            )
            .push(iced::widget::space().width(Length::Fill))
            .push(button::segment(
                "Galeria".into(),
                self.view == View::Gallery,
                Message::Library(Event::View(View::Gallery)),
            ))
            .push(button::segment(
                "Lista".into(),
                self.view == View::List,
                Message::Library(Event::View(View::List)),
            ));
        let mut body = Column::new()
            .spacing(design::space::MD)
            .push(heading)
            .push(filters)
            .push((filtered.len() != games.len()).then(|| {
                text(format!("{} de {} jogos", filtered.len(), games.len()))
                    .size(design::text::CAPTION)
                    .class(style::Text::Muted)
            }));
        if let Some(notice) = &self.notice {
            body = body.push(text(notice));
        }
        if filtered.is_empty() {
            body = body.push(text("Nenhum jogo encontrado. Tente outro nome ou emulador."));
        } else if self.view == View::Gallery {
            let mut grid = Row::new().spacing(design::library::GAP);
            for game in filtered {
                grid = grid.push(self.card(game));
            }
            body = body.push(grid.wrap().vertical_spacing(design::library::GAP));
        } else {
            for game in filtered {
                body = body.push(self.list_row(game));
            }
        }
        ScrollSubject::Restore
            .into_widget(body.padding([design::space::LG, design::space::XXL]))
            .into()
    }

    fn details<'a>(&'a self, game: &'a vault::Game, idle: bool) -> Element<'a> {
        let newest = self.versions.first().cloned();
        let restore = button::primary_bar(
            "Restaurar última versão".into(),
            newest
                .as_ref()
                .and_then(|backup| idle.then(|| Message::Library(Event::Preview(game.name.clone(), backup.clone())))),
            Some("Abre a revisão do destino antes de escrever.".into()),
        );
        let mut actions = Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(restore)
            .push(button::secondary(
                "Fazer novo backup".into(),
                idle.then(|| Message::Library(Event::Backup(game.name.clone()))),
                None,
            ))
            .push(button::secondary(
                "Escolher capa".into(),
                Some(Message::Library(Event::ChooseCover(game.name.clone()))),
                None,
            ));
        if let Some(notice) = &self.notice {
            actions = actions.push(text(notice.clone()).class(style::Text::OnDarkMuted));
        }
        let copy = Column::new()
            .spacing(design::space::SM)
            .width(Length::Fill)
            .push(
                text(platform_name(game).to_uppercase())
                    .size(design::text::CAPTION)
                    .class(style::Text::OnDarkMuted),
            )
            .push(
                text(game.title.clone())
                    .size(design::text::DISPLAY)
                    .font(font::TEXT_STRONG)
                    .class(style::Text::OnDark)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::Word),
            )
            .push(
                text(format!(
                    "{} para restaurar · último backup em {}",
                    version_count(game.backups),
                    game.last_backup.format("%d/%m/%Y às %H:%M")
                ))
                .class(style::Text::OnDarkMuted),
            )
            .push(
                Row::new()
                    .spacing(design::space::SM)
                    .push(chip(version_count(game.backups)))
                    .push(chip(file_count(game.files)))
                    .push(chip(crate::lang::TRANSLATOR.adjusted_size(game.bytes))),
            )
            .push(actions);
        let hero_body = Row::new()
            .spacing(design::space::XL)
            .align_y(Alignment::Center)
            .push(self.portrait(
                game,
                design::library::HERO_THUMB_WIDTH,
                design::library::HERO_THUMB_HEIGHT,
                true,
            ))
            .push(copy);
        let hero_art: Element<'a> = if let Some(handle) = self.banners.get(&game.name) {
            iced::widget::image(handle.clone())
                .width(Length::Fill)
                .height(design::library::HERO_HEIGHT)
                .content_fit(iced::ContentFit::Cover)
                .into()
        } else {
            Container::new(iced::widget::space())
                .width(Length::Fill)
                .height(design::library::HERO_HEIGHT)
                .class(style::Container::Card)
                .into()
        };
        let hero = Stack::new()
            .width(Length::Fill)
            .height(design::library::HERO_HEIGHT)
            .push(hero_art)
            .push(
                Container::new(hero_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([design::space::LG, design::space::XL])
                    .align_y(alignment::Vertical::Center),
            );
        let tabs = Row::new()
            .spacing(design::space::SM)
            .push(button::segment(
                "Visão geral".into(),
                self.tab == DetailTab::Overview,
                Message::Library(Event::Tab(DetailTab::Overview)),
            ))
            .push(button::segment(
                "Versões".into(),
                self.tab == DetailTab::Versions,
                Message::Library(Event::Tab(DetailTab::Versions)),
            ))
            .push(button::segment(
                "Registros".into(),
                self.tab == DetailTab::Records,
                Message::Library(Event::Tab(DetailTab::Records)),
            ));
        let main = match self.tab {
            DetailTab::Overview => self.overview(game),
            DetailTab::Versions => self.version_list(game, idle),
            DetailTab::Records => self.record_list(),
        };
        let page = Column::new()
            .spacing(design::space::LG)
            .width(Length::Fill)
            .push(button::secondary(
                "Voltar à biblioteca".into(),
                Some(Message::Library(Event::Back)),
                None,
            ))
            .push(hero)
            .push(tabs)
            .push(
                Row::new()
                    .spacing(design::space::XL)
                    .align_y(Alignment::Start)
                    .push(main)
                    .push(self.side_facts(game)),
            );
        ScrollSubject::Restore
            .into_widget(page.padding([design::space::LG, design::space::XXL]))
            .into()
    }

    fn overview<'a>(&'a self, game: &'a vault::Game) -> Element<'a> {
        let newest = self.versions.first();
        let mut body = Column::new()
            .spacing(design::space::SM)
            .width(Length::Fill)
            .push(text("O que está guardado").size(design::text::SUBTITLE).font(font::TEXT_STRONG))
            .push(
                text(format!(
                    "Os saves de {} estão no cofre. Restaurar abre a revisão dos arquivos e do destino antes de escrever qualquer coisa.",
                    game.title
                ))
                .class(style::Text::Muted),
            );
        if let Some(backup) = newest {
            let kind = if matches!(backup, Backup::Full(_)) {
                "completa"
            } else {
                "diferencial"
            };
            body = body.push(text(format!(
                "A versão mais recente é {kind}, de {}.",
                backup.when_local().format("%d/%m/%Y às %H:%M")
            )));
            if let Some(comment) = backup.comment() {
                body = body.push(text(comment.clone()).class(style::Text::Muted));
            }
        } else {
            body = body.push(text("As versões não estão mais disponíveis. Atualize a biblioteca."));
        }
        Container::new(body)
            .padding(design::space::LG)
            .width(Length::Fill)
            .class(style::Container::Card)
            .into()
    }

    fn version_list<'a>(&'a self, game: &'a vault::Game, idle: bool) -> Element<'a> {
        let mut column = Column::new().spacing(design::space::MD).width(Length::Fill).push(
            text("Escolha uma versão para revisar os arquivos e o destino antes de restaurar.")
                .class(style::Text::Muted),
        );
        for (index, backup) in self.versions.iter().enumerate() {
            column = column.push(version_card(game, backup, index == 0, idle));
        }
        if self.versions.is_empty() {
            column = column.push(text("As versões não estão mais disponíveis. Atualize a biblioteca."));
        }
        column.into()
    }

    fn record_list<'a>(&'a self) -> Element<'a> {
        let mut column = Column::new()
            .spacing(design::space::MD)
            .width(Length::Fill)
            .push(text("Anotações gravadas com cada versão.").class(style::Text::Muted));
        let mut any = false;
        for backup in &self.versions {
            let Some(comment) = backup.comment() else {
                continue;
            };
            if comment.trim().is_empty() {
                continue;
            }
            any = true;
            column = column.push(
                Container::new(
                    Column::new()
                        .spacing(design::space::XS)
                        .push(text(backup.when_local().format("%d/%m/%Y às %H:%M").to_string()).font(font::TEXT_STRONG))
                        .push(text(comment.clone())),
                )
                .padding(design::space::MD)
                .width(Length::Fill)
                .class(style::Container::Card),
            );
        }
        if !any {
            column = column.push(
                Container::new(text("Nenhuma versão tem anotação.").class(style::Text::Muted))
                    .padding(design::space::LG)
                    .width(Length::Fill)
                    .class(style::Container::Card),
            );
        }
        column.into()
    }

    fn side_facts<'a>(&'a self, game: &'a vault::Game) -> Element<'a> {
        Container::new(
            Column::new()
                .spacing(design::space::SM)
                .push(text("No cofre").size(design::text::SUBTITLE).font(font::TEXT_STRONG))
                .push(text(platform_name(game)).class(style::Text::Muted))
                .push(text(version_count(game.backups)))
                .push(text(file_count(game.files)).class(style::Text::Muted))
                .push(text(crate::lang::TRANSLATOR.adjusted_size(game.bytes)).class(style::Text::Muted))
                .push(
                    text(format!("Último backup em {}", game.last_backup.format("%d/%m/%Y"))).class(style::Text::Muted),
                )
                .push(
                    text("A restauração não escreve direto. Ela abre a revisão do destino.")
                        .size(design::text::CAPTION)
                        .class(style::Text::Muted),
                ),
        )
        .padding(design::space::LG)
        .width(design::library::SIDE_WIDTH)
        .class(style::Container::Card)
        .into()
    }
}

fn cover_path(game: &str, manual: bool) -> PathBuf {
    directory().join("covers").join(format!(
        "{}{}.png",
        crate::prelude::sha1(game.to_owned()),
        if manual { "-manual" } else { "" }
    ))
}

fn decode(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("A capa deve ter no máximo 12 MB.".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| "Não foi possível ler a imagem. Escolha um arquivo PNG ou JPEG válido.".into())
}

fn loaded_from_image(image: &image::DynamicImage) -> LoadedCover {
    let portrait = image.thumbnail(300, 450).to_rgba8();
    let banner = landscape_frame(image);
    LoadedCover {
        portrait: iced::widget::image::Handle::from_rgba(portrait.width(), portrait.height(), portrait.into_raw()),
        banner: iced::widget::image::Handle::from_rgba(banner.width(), banner.height(), banner.into_raw()),
    }
}

fn persist(game: &str, bytes: &[u8], manual: bool) -> Result<LoadedCover, String> {
    let image = decode(bytes)?;
    let path = cover_path(game, manual);
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    image
        .thumbnail(600, 900)
        .save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(loaded_from_image(&image))
}

pub fn import_cover(game: &str, path: PathBuf) -> Result<Option<LoadedCover>, String> {
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 12 * 1024 * 1024 {
        return Err("A capa deve ter no máximo 12 MB.".into());
    }
    persist(game, &std::fs::read(path).map_err(|e| e.to_string())?, true).map(Some)
}

pub async fn fetch_cover(game: String, steam: Option<u32>) -> Result<Option<LoadedCover>, String> {
    for manual in [true, false] {
        if let Ok(bytes) = std::fs::read(cover_path(&game, manual))
            && let Ok(image) = decode(&bytes)
        {
            return Ok(Some(loaded_from_image(&image)));
        }
    }
    let Some(id) = steam else {
        return Ok(None);
    };

    static DOWNLOADS: std::sync::LazyLock<tokio::sync::Semaphore> =
        std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(4));
    let _permit = DOWNLOADS.acquire().await.map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(format!(
            "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{id}/library_600x900_2x.jpg"
        ))
        .send()
        .await
        .map_err(|_| "Sem conexão para buscar a capa. Você pode escolher uma imagem.".to_string())?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > 12 * 1024 * 1024 {
            return Err("A capa recebida é grande demais.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    // Uma escolha manual feita enquanto a rede respondia sempre tem precedência.
    if cover_path(&game, true).exists() {
        return import_cover(&game, cover_path(&game, true));
    }
    persist(&game, &bytes, false).map(Some)
}

/// A 16:9 frame for a cover that is usually a portrait.
///
/// There is no separate landscape art. The background is the same cover, scaled to fill, blurred
/// and darkened. The sharp cover is fitted inside that frame and centered, so a 2:3 box is not
/// cropped down to a face or a logo.
fn landscape_frame(source: &image::DynamicImage) -> image::RgbaImage {
    const FRAME_W: u32 = 960;
    const FRAME_H: u32 = 540;
    let backdrop = cover_crop(source, 320, 180);
    let backdrop = image::imageops::blur(&backdrop, 8.0);
    let mut frame = image::DynamicImage::ImageRgba8(backdrop)
        .resize_exact(FRAME_W, FRAME_H, image::imageops::FilterType::Triangle)
        .to_rgba8();
    for pixel in frame.pixels_mut() {
        pixel[0] = (f32::from(pixel[0]) * 0.38) as u8;
        pixel[1] = (f32::from(pixel[1]) * 0.38) as u8;
        pixel[2] = (f32::from(pixel[2]) * 0.38) as u8;
    }
    let sharp = source
        .resize(
            FRAME_W * 2 / 5,
            FRAME_H.saturating_sub(48),
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    let x = FRAME_W.saturating_sub(sharp.width()) / 2;
    let y = FRAME_H.saturating_sub(sharp.height()) / 2;
    image::imageops::overlay(&mut frame, &sharp, i64::from(x), i64::from(y));
    frame
}

fn cover_crop(source: &image::DynamicImage, width: u32, height: u32) -> image::RgbaImage {
    let scale = (width as f32 / source.width().max(1) as f32).max(height as f32 / source.height().max(1) as f32);
    let scaled_w = ((source.width() as f32) * scale).ceil().max(width as f32) as u32;
    let scaled_h = ((source.height() as f32) * scale).ceil().max(height as f32) as u32;
    let scaled = source.resize_exact(scaled_w, scaled_h, image::imageops::FilterType::Triangle);
    let x = scaled_w.saturating_sub(width) / 2;
    let y = scaled_h.saturating_sub(height) / 2;
    scaled.crop_imm(x, y, width, height).to_rgba8()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(name: &str, title: &str, day: u32) -> vault::Game {
        vault::Game {
            name: name.into(),
            title: title.into(),
            last_backup: chrono::DateTime::parse_from_rfc3339(&format!("2026-10-{day:02}T10:00:00Z"))
                .unwrap()
                .with_timezone(&chrono::Local),
            backups: 2,
            files: 1,
            bytes: 1,
            emulator: Some("DuckStation".into()),
        }
    }
    #[test]
    fn pesquisa_titulo_e_serial_sem_trocar_identidade() {
        let games = vec![
            game("SLUS-00594", "Final Fantasy VII", 1),
            game("SLUS-00067", "Castlevania", 2),
        ];
        let mut library = Library {
            query: "final fantasy".into(),
            ..Library::default()
        };
        assert_eq!(library.filtered(&games)[0].name, "SLUS-00594");
        library.query = "slus-00067".into();
        assert_eq!(library.filtered(&games)[0].title, "Castlevania");
    }
    #[test]
    fn ordena_por_data_e_nome() {
        let games = vec![game("a", "Zelda", 1), game("b", "Castlevania", 2)];
        let mut library = Library::default();
        assert_eq!(library.filtered(&games)[0].name, "b");
        library.sort = Sort::Oldest;
        assert_eq!(library.filtered(&games)[0].name, "a");
        library.sort = Sort::Name;
        assert_eq!(library.filtered(&games)[0].name, "b");
    }
    #[test]
    fn imagem_invalida_nao_e_aceita() {
        assert!(decode(b"<html>erro</html>").is_err());
    }
    #[test]
    fn filtra_por_plataforma_sem_trocar_identidade() {
        let games = vec![
            game("SLUS-00594", "Final Fantasy VII", 1),
            vault::Game {
                emulator: None,
                ..game("hk", "Hollow Knight", 2)
            },
        ];
        let mut library = Library {
            platform: PlatformFilter::Pc,
            ..Library::default()
        };
        assert_eq!(library.filtered(&games)[0].name, "hk");
        library.platform = PlatformFilter::Emulator("DuckStation".into());
        assert_eq!(library.filtered(&games)[0].name, "SLUS-00594");
    }

    #[test]
    fn quadro_horizontal_nao_corta_a_capa() {
        let image = image::RgbaImage::from_pixel(20, 30, image::Rgba([220, 40, 40, 255]));
        let frame = landscape_frame(&image::DynamicImage::ImageRgba8(image));
        assert_eq!((frame.width(), frame.height()), (960, 540));
        let center = frame.get_pixel(480, 270);
        let corner = frame.get_pixel(4, 4);
        assert!(
            u16::from(center[0]) > u16::from(corner[0]) + 40,
            "a capa nítida fica no centro e o fundo desfocado fica escurecido"
        );
    }

    #[test]
    fn capa_usa_chave_opaca_sem_caminho_do_jogo() {
        let path = cover_path("../../jogo", true);
        assert_eq!(path.parent().unwrap(), directory().join("covers"));
        assert!(path.file_name().unwrap().to_string_lossy().ends_with("-manual.png"));
    }
}

fn version_count(count: usize) -> String {
    if count == 1 {
        "1 versão".into()
    } else {
        format!("{count} versões")
    }
}

fn file_count(count: usize) -> String {
    if count == 1 {
        "1 arquivo".into()
    } else {
        format!("{count} arquivos")
    }
}

fn platform_name(game: &vault::Game) -> String {
    game.emulator.clone().unwrap_or_else(|| "PC".into())
}

fn chip<'a>(label: impl Into<String>) -> Element<'a> {
    Container::new(text(label.into()).size(design::text::CAPTION))
        .padding([2.0, design::space::SM])
        .class(style::Container::Chip { positive: false })
        .into()
}

fn count_badge<'a>(count: usize) -> Element<'a> {
    Container::new(
        text(count.to_string())
            .size(design::text::CAPTION)
            .font(font::TEXT_STRONG),
    )
    .padding([2.0, design::space::SM])
    .class(style::Container::Chip { positive: true })
    .into()
}

fn search_icon() -> iced::widget::text_input::Icon<iced::Font> {
    iced::widget::text_input::Icon {
        font: font::ICONS,
        code_point: Icon::Search.as_char(),
        size: Some(iced::Pixels(design::icon::SM)),
        spacing: design::space::SM,
        side: iced::widget::text_input::Side::Left,
    }
}

fn version_card<'a>(game: &'a vault::Game, backup: &'a Backup, newest: bool, idle: bool) -> Element<'a> {
    let kind = if matches!(backup, Backup::Full(_)) {
        "Completo"
    } else {
        "Diferencial"
    };
    let mut description = Column::new()
        .spacing(design::space::XS)
        .width(Length::Fill)
        .push(text(backup.when_local().format("%d/%m/%Y às %H:%M").to_string()).font(font::TEXT_STRONG))
        .push(
            text(format!(
                "{kind}{}{}",
                if newest { " · Mais recente" } else { "" },
                if backup.locked() { " · Protegido" } else { "" }
            ))
            .class(style::Text::Muted),
        );
    if let Some(comment) = backup.comment() {
        description = description.push(text(comment.clone()));
    }
    Container::new(
        Row::new()
            .spacing(design::space::MD)
            .align_y(Alignment::Center)
            .push(description)
            .push(button::secondary(
                "Revisar restauração".into(),
                idle.then(|| Message::Library(Event::Preview(game.name.clone(), backup.clone()))),
                None,
            )),
    )
    .padding(design::space::MD)
    .width(Length::Fill)
    .class(style::Container::Card)
    .into()
}

/// Cuts a title so it stays inside the card. The full name stays on the tooltip.
fn ellipsize(title: &str, max_chars: usize) -> String {
    if title.chars().count() <= max_chars {
        return title.to_string();
    }
    let keep = max_chars.saturating_sub(1).max(1);
    let mut end = title.len();
    for (index, (byte, _)) in title.char_indices().enumerate() {
        if index == keep {
            end = byte;
            break;
        }
    }
    if let Some(word) = title[..end].rfind(' ')
        && word >= end / 2
    {
        end = word;
    }
    format!("{}…", title[..end].trim_end())
}

#[cfg(test)]
mod ellipsize_tests {
    use super::ellipsize;

    #[test]
    fn titulo_curto_permanece() {
        assert_eq!(ellipsize("Hades", 24), "Hades");
    }

    #[test]
    fn titulo_longo_ganha_reticencias_e_nao_passa_do_limite() {
        let title = "Jogo de teste com um título muito longo para verificar o comportamento da biblioteca";
        let cut = ellipsize(title, 24);
        assert!(cut.ends_with('…'));
        assert!(cut.chars().count() <= 24);
        assert!(title.starts_with(cut.trim_end_matches('…').trim_end()));
    }
}
