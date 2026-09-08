//! The artwork that identifies each emulator on the Emulators screen.
//!
//! # Where the logos come from
//!
//! Each one is the project's own icon, taken from its own repository. `assets/emulators/SOURCES.md`
//! records the exact URL and licence for every file; nothing here was drawn by us and nothing was
//! taken from a search result.
//!
//! # Why the console name lives here and not in `scan::emulator`
//!
//! The scan layer states facts about folders on disk. Which console an emulator emulates is true,
//! but it is only ever used to label a card, and putting it in `Profile` would mean editing eight
//! consts in a file that gets merged from upstream. This is the presentation layer, so the label
//! belongs here — the same reason the folder verdicts are translated in `screen.rs` instead of in
//! the engine.

use iced::Length;

use crate::{
    gui::{
        design, font,
        icon::Icon,
        style,
        widget::{Container, Element, text},
    },
    scan::emulator::App,
};

/// A logo file, in the format its project publishes it in.
///
/// Both formats are here because the projects do not agree: DuckStation, PCSX2 and Xenia ship a
/// raster icon and no vector one, while Eden, PPSSPP, RPCS3 and shadPS4 ship SVG. Converting
/// either way would mean shipping artwork the project did not publish.
enum Logo {
    Png(&'static [u8]),
    Svg(&'static [u8]),
}

/// The logo for an emulator, when its project publishes one we can ship.
///
/// `None` is not an oversight. Sudachi's repository was taken down, so there is no upstream
/// artwork left to point at, and inventing a logo for someone else's project is exactly the kind
/// of thing this codebase does not do. Those fall back to a generic glyph.
fn logo(app: App) -> Option<Logo> {
    match app {
        App::DuckStation => Some(Logo::Png(include_bytes!("../../assets/emulators/duckstation.png"))),
        App::Pcsx2 => Some(Logo::Png(include_bytes!("../../assets/emulators/pcsx2.png"))),
        App::Xenia => Some(Logo::Png(include_bytes!("../../assets/emulators/xenia.png"))),
        App::Eden => Some(Logo::Svg(include_bytes!("../../assets/emulators/eden.svg"))),
        App::Ppsspp => Some(Logo::Svg(include_bytes!("../../assets/emulators/ppsspp.svg"))),
        App::Rpcs3 => Some(Logo::Svg(include_bytes!("../../assets/emulators/rpcs3.svg"))),
        App::ShadPs4 => Some(Logo::Svg(include_bytes!("../../assets/emulators/shadps4.svg"))),
        App::Sudachi => None,
    }
}

/// The console the emulator runs games from.
///
/// This is what turns a list of eight unfamiliar words into something a person can scan: most
/// people looking for their saves know they played a PS2 game, not that PCSX2 is the thing that
/// runs it.
pub fn console(app: App) -> &'static str {
    match app {
        App::DuckStation => "PlayStation",
        App::Pcsx2 => "PlayStation 2",
        App::Rpcs3 => "PlayStation 3",
        App::ShadPs4 => "PlayStation 4",
        App::Ppsspp => "PSP",
        App::Eden | App::Sudachi => "Nintendo Switch",
        App::Xenia => "Xbox 360",
    }
}

/// The logo, in the tile it sits in.
///
/// The tile is always the same size, and it sits on the theme's field surface — the same one a text
/// input uses, so it reads as a frame in either theme rather than as a light patch pasted on a dark
/// card. The fixed size is the part that is not negotiable: the artwork arrives at seven different
/// aspect ratios, and without a frame of its own no two cards in the grid would line up.
///
/// It works on the dark theme because every one of these logos is a coloured mark, not a
/// single-colour silhouette. A logo that were white-on-transparent would disappear here, and the
/// answer then would be a light tile, not a tinted logo.
pub fn tile<'a>(app: App) -> Element<'a> {
    let art: Element<'a> = match logo(app) {
        Some(Logo::Png(bytes)) => iced::widget::image(iced::widget::image::Handle::from_bytes(bytes))
            .width(design::emulator_card::LOGO)
            .height(design::emulator_card::LOGO)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        Some(Logo::Svg(bytes)) => iced::widget::svg(iced::widget::svg::Handle::from_memory(bytes))
            .width(design::emulator_card::LOGO)
            .height(design::emulator_card::LOGO)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        None => text(Icon::VideogameAsset.as_char().to_string())
            .font(font::ICONS)
            .size(design::icon::LG)
            .line_height(design::leading::TIGHT)
            .class(style::Text::Muted)
            .into(),
    };

    // `center_x` and `center_y` take the size as their argument and overwrite whatever `width` and
    // `height` set before them, so the tile has to be sized through them. Written the other way
    // round it silently becomes a Fill × Fill container, and a tile that grows without limit takes
    // the whole card layout down with it.
    Container::new(art)
        .center_x(Length::Fixed(design::emulator_card::TILE))
        .center_y(Length::Fixed(design::emulator_card::TILE))
        .class(style::Container::LogoTile)
        .into()
}

/// The tile for one game in the vault inventory.
///
/// An emulator game gets the emulator's own logo, which is already bundled and needs no network.
/// Everything else gets its initial, which is a placeholder and says so: it is the shape a cover
/// will take once there is a cover to put there.
pub fn game_tile<'a>(game: &crate::gui::vault::Game) -> Element<'a> {
    let app = game
        .emulator
        .as_ref()
        .and_then(|name| App::ALL.iter().copied().find(|app| app.name() == name));

    match app {
        Some(app) => tile(app),
        None => Container::new(
            text(
                game.name
                    .chars()
                    .find(|c| c.is_alphanumeric())
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_else(|| "?".to_string()),
            )
            .font(font::TEXT_STRONG)
            .size(design::text::TITLE)
            .line_height(design::leading::TIGHT)
            .class(style::Text::Muted),
        )
        .center_x(Length::Fixed(design::emulator_card::TILE))
        .center_y(Length::Fixed(design::emulator_card::TILE))
        .class(style::Container::LogoTile)
        .into(),
    }
}
