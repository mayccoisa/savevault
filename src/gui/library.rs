//! Biblioteca visual. A identidade do backup permanece separada do título e da capa.
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use iced::{Alignment, Length};
use serde::{Deserialize, Serialize};

use crate::{
    gui::{
        button,
        common::{Message, Screen, ScrollSubject},
        design, font, style, vault,
        widget::{Button, Column, Container, Element, Row, TextInput, Tooltip, pick_list, text},
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

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    View(View),
    Sort(Sort),
    Open(String),
    Back,
    Refresh,
    Operational,
    ChooseCover(String),
    CoverChosen(String, Option<PathBuf>),
    CoverReady(String, Result<Option<iced::widget::image::Handle>, String>),
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
    pub selected: Option<String>,
    pub versions: Vec<Backup>,
    pub operational: bool,
    pub covers: BTreeMap<String, iced::widget::image::Handle>,
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
        self.notice = None;
    }

    pub fn filtered<'a>(&self, games: &'a [vault::Game]) -> Vec<&'a vault::Game> {
        let query = self.query.trim().to_lowercase();
        let mut result: Vec<_> = games
            .iter()
            .filter(|g| {
                g.title.to_lowercase().contains(&query)
                    || g.name.to_lowercase().contains(&query)
                    || g.emulator.as_deref().unwrap_or("PC").to_lowercase().contains(&query)
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

    fn cover<'a>(&'a self, game: &'a vault::Game, width: f32, height: f32) -> Element<'a> {
        if let Some(handle) = self.covers.get(&game.name) {
            iced::widget::image(handle.clone())
                .width(width)
                .height(height)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(iced::border::Radius {
                    top_left: design::RADIUS,
                    top_right: design::RADIUS,
                    bottom_left: 0.0,
                    bottom_right: 0.0,
                })
                .into()
        } else {
            Container::new(
                Column::new()
                    .spacing(design::space::SM)
                    .align_x(Alignment::Center)
                    .push(
                        text(game.title.chars().next().unwrap_or('?').to_string())
                            .size(if width >= design::library::COVER_WIDTH {
                                design::text::SUBTITLE
                            } else {
                                design::text::BODY
                            })
                            .font(font::TEXT_STRONG),
                    )
                    .push(
                        (width >= design::library::COVER_WIDTH)
                            .then(|| text("Sem capa").size(design::text::CAPTION).class(style::Text::Muted)),
                    ),
            )
            .center_x(width)
            .center_y(height)
            .class(style::Container::Card)
            .into()
        }
    }

    fn card<'a>(&'a self, game: &'a vault::Game) -> Element<'a> {
        let inner = design::library::COVER_WIDTH - 2.0 * design::library::CARD_PAD;
        let title = ellipsize(&game.title, design::text_budget(inner, design::text::BODY, 2));
        let body = Column::new()
            .width(design::library::COVER_WIDTH)
            .spacing(design::space::SM)
            .push(self.cover(game, design::library::COVER_WIDTH, design::library::COVER_HEIGHT))
            .push(
                Container::new(
                    Column::new()
                        .spacing(design::space::XS)
                        .push(
                            Container::new(
                                text(title)
                                    .font(font::TEXT_STRONG)
                                    .size(design::text::BODY)
                                    .line_height(design::leading::TIGHT)
                                    .wrapping(iced::widget::text::Wrapping::Word),
                            )
                            .width(Length::Fill)
                            .height(design::library::TITLE_BLOCK)
                            .clip(true),
                        )
                        .push(
                            text(game.emulator.clone().unwrap_or_else(|| "PC".into()))
                                .size(design::text::CAPTION)
                                .class(style::Text::Muted),
                        )
                        .push(text(version_count(game.backups)).size(design::text::BODY))
                        .push(
                            text(format!("Último backup · {}", game.last_backup.format("%d/%m/%Y")))
                                .size(design::text::CAPTION)
                                .class(style::Text::Muted),
                        ),
                )
                .width(Length::Fill)
                .padding(design::library::CARD_PAD),
            );
        Tooltip::new(
            Button::new(body)
                .padding(0)
                .width(design::library::COVER_WIDTH)
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
                .push(self.cover(game, design::library::THUMB_WIDTH, design::library::THUMB_HEIGHT))
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
        let toolbar = Row::new()
            .spacing(design::space::SM)
            .align_y(Alignment::Center)
            .push(
                TextInput::new("Buscar jogo ou emulador", &self.query)
                    .on_input(|q| Message::Library(Event::Search(q)))
                    .size(design::text::BODY)
                    .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                        design::control::LINE,
                    )))
                    .padding([design::control::PAD_Y, design::control::PAD_X])
                    .width(Length::Fixed(design::control::SEARCH_WIDTH)),
            )
            .push(pick_list(
                [Sort::Recent, Sort::Name, Sort::Oldest],
                Some(self.sort),
                |s| Message::Library(Event::Sort(s)),
            ))
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
        let mut body = Column::new().spacing(design::space::LG).push(toolbar).push(
            text(format!(
                "{} de {} jogos · {} versões no cofre",
                filtered.len(),
                games.len(),
                games.iter().map(|g| g.backups).sum::<usize>()
            ))
            .size(design::text::CAPTION)
            .class(style::Text::Muted),
        );
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
        let mut info = Column::new()
            .spacing(design::space::MD)
            .width(Length::Fill)
            .push(
                text(game.title.clone())
                    .size(design::text::TITLE)
                    .font(font::TEXT_STRONG)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::Word),
            )
            .push(text(game.emulator.clone().unwrap_or_else(|| "PC".into())).class(style::Text::Muted))
            .push(text(format!("{} para restaurar", version_count(game.backups))))
            .push(text(format!(
                "Último backup em {}",
                game.last_backup.format("%d/%m/%Y às %H:%M")
            )))
            .push(
                text(format!(
                    "{} · {}",
                    if game.files == 1 {
                        "1 arquivo".into()
                    } else {
                        format!("{} arquivos", game.files)
                    },
                    crate::lang::TRANSLATOR.adjusted_size(game.bytes)
                ))
                .class(style::Text::Muted),
            )
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
            info = info.push(text(notice));
        }
        let mut body = Column::new()
            .spacing(design::space::XL)
            .push(button::secondary(
                "Voltar à biblioteca".into(),
                Some(Message::Library(Event::Back)),
                None,
            ))
            .push(
                Row::new()
                    .spacing(design::space::XL)
                    .push(self.cover(game, design::library::DETAIL_WIDTH, design::library::DETAIL_HEIGHT))
                    .push(info),
            )
            .push(
                text("Histórico de versões")
                    .size(design::text::SUBTITLE)
                    .font(font::TEXT_STRONG),
            )
            .push(
                text("Escolha uma versão para revisar os arquivos e o destino antes de restaurar.")
                    .class(style::Text::Muted),
            );
        for (index, backup) in self.versions.iter().enumerate() {
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
                        if index == 0 { " · Mais recente" } else { "" },
                        if backup.locked() { " · Protegido" } else { "" }
                    ))
                    .class(style::Text::Muted),
                );
            if let Some(comment) = backup.comment() {
                description = description.push(text(comment.clone()));
            }
            body = body.push(
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
                .class(style::Container::Card),
            );
        }
        if self.versions.is_empty() {
            body = body.push(text("As versões não estão mais disponíveis. Atualize a biblioteca."));
        }
        ScrollSubject::Restore
            .into_widget(body.padding([design::space::LG, design::space::XXL]))
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

fn persist(game: &str, bytes: &[u8], manual: bool) -> Result<iced::widget::image::Handle, String> {
    let image = decode(bytes)?.thumbnail(600, 900);
    let path = cover_path(game, manual);
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    image
        .save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let rgba = image.thumbnail(300, 450).to_rgba8();
    Ok(iced::widget::image::Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}

pub fn import_cover(game: &str, path: PathBuf) -> Result<Option<iced::widget::image::Handle>, String> {
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 12 * 1024 * 1024 {
        return Err("A capa deve ter no máximo 12 MB.".into());
    }
    persist(game, &std::fs::read(path).map_err(|e| e.to_string())?, true).map(Some)
}

pub async fn fetch_cover(game: String, steam: Option<u32>) -> Result<Option<iced::widget::image::Handle>, String> {
    for manual in [true, false] {
        if let Ok(bytes) = std::fs::read(cover_path(&game, manual))
            && let Ok(image) = decode(&bytes)
        {
            let rgba = image.thumbnail(300, 450).to_rgba8();
            return Ok(Some(iced::widget::image::Handle::from_rgba(
                rgba.width(),
                rgba.height(),
                rgba.into_raw(),
            )));
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
