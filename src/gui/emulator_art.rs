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
/// The tile is always the same size and always the same light surface, in both themes. Both parts
/// matter: the artwork arrives at seven different aspect ratios, so without a fixed frame no two
/// cards in the grid would line up; and several of these logos are drawn for a light background,
/// so on the dark theme they would lose their outline into the card behind them.
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
