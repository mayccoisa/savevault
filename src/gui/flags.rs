//! Flag artwork for the language list.
//!
//! Windows does not draw flag emoji, so each language ships an SVG. The files under
//! `assets/flags` are the 4x3 set from flag-icons (MIT), except Esperanto, which that set does
//! not include. See `assets/flags/NOTICE`.

use std::borrow::Cow;

use iced::widget::svg::{self, Handle};

use crate::{gui::widget::Element, lang::Language};

pub fn bytes(language: Language) -> &'static [u8] {
    match language {
        Language::Arabic => include_bytes!("../../assets/flags/sa.svg"),
        Language::ChineseSimplified => include_bytes!("../../assets/flags/cn.svg"),
        Language::ChineseTraditional => include_bytes!("../../assets/flags/tw.svg"),
        Language::Czech => include_bytes!("../../assets/flags/cz.svg"),
        Language::Dutch => include_bytes!("../../assets/flags/nl.svg"),
        Language::English => include_bytes!("../../assets/flags/us.svg"),
        Language::Esperanto => include_bytes!("../../assets/flags/eo.svg"),
        Language::Filipino => include_bytes!("../../assets/flags/ph.svg"),
        Language::Finnish => include_bytes!("../../assets/flags/fi.svg"),
        Language::French => include_bytes!("../../assets/flags/fr.svg"),
        Language::German => include_bytes!("../../assets/flags/de.svg"),
        Language::Italian => include_bytes!("../../assets/flags/it.svg"),
        Language::Japanese => include_bytes!("../../assets/flags/jp.svg"),
        Language::Korean => include_bytes!("../../assets/flags/kr.svg"),
        Language::Norwegian => include_bytes!("../../assets/flags/no.svg"),
        Language::Polish => include_bytes!("../../assets/flags/pl.svg"),
        Language::PortugueseBrazilian => include_bytes!("../../assets/flags/br.svg"),
        Language::Russian => include_bytes!("../../assets/flags/ru.svg"),
        Language::Spanish => include_bytes!("../../assets/flags/es.svg"),
        Language::Swedish => include_bytes!("../../assets/flags/se.svg"),
        Language::Thai => include_bytes!("../../assets/flags/th.svg"),
        Language::Turkish => include_bytes!("../../assets/flags/tr.svg"),
        Language::Ukrainian => include_bytes!("../../assets/flags/ua.svg"),
        Language::Vietnamese => include_bytes!("../../assets/flags/vn.svg"),
    }
}

/// The flag beside a language name. Not tinted: the theme's SVG style leaves `color` empty.
pub fn flag(language: Language) -> Element<'static> {
    svg::Svg::new(Handle::from_memory(Cow::Borrowed(bytes(language))))
        .width(crate::gui::design::settings::FLAG_WIDTH)
        .height(crate::gui::design::settings::FLAG_HEIGHT)
        .content_fit(iced::ContentFit::Contain)
        .into()
}
