use iced::{Font, font};

pub const TEXT_DATA: &[u8] = include_bytes!("../../assets/NotoSans-Regular.ttf");
pub const TEXT: Font = Font {
    family: font::Family::Name("Noto Sans"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

pub const ICONS_DATA: &[u8] = include_bytes!("../../assets/MaterialIcons-Regular.ttf");
pub const ICONS: Font = Font {
    family: font::Family::Name("Material Icons"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

/// The semibold face, for emphasis.
///
/// Hierarchy carried by size alone produces a primary that is too big and a secondary that is too
/// small, which is exactly what the old screens did. With a second weight available, the type scale
/// can stay short: a card title is one step up in size and one step up in weight, not three steps
/// up in size.
///
/// Same family and same licence (OFL 1.1) as [`TEXT`], from the Noto project.
pub const TEXT_STRONG_DATA: &[u8] = include_bytes!("../../assets/NotoSans-SemiBold.ttf");
pub const TEXT_STRONG: Font = Font {
    family: font::Family::Name("Noto Sans"),
    weight: font::Weight::Semibold,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};
