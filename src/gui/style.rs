use iced::{
    Background, Border, Color, Shadow,
    widget::{button, checkbox, container, pick_list, scrollable, text_editor, text_input},
};

use crate::{gui::design, resource::config, scan::ScanChange};

macro_rules! rgb8 {
    ($r:expr, $g:expr, $b:expr) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

trait ColorExt {
    fn alpha(self, alpha: f32) -> Color;
}

impl ColorExt for Color {
    fn alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}

pub struct Theme {
    source: config::Theme,
    background: Color,
    panel: Color,
    accent_ink: Color,
    field: Color,
    text: Color,
    text_inverted: Color,
    text_button: Color,
    text_skipped: Color,
    text_selection: Color,
    positive: Color,
    negative: Color,
    disabled: Color,
    navigation: Color,
    success: Color,
    failure: Color,
    skipped: Color,
    added: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(config::Theme::default(), config::Accent::default())
    }
}

impl From<config::Theme> for Theme {
    fn from(source: config::Theme) -> Self {
        Self::new(source, config::Accent::default())
    }
}

impl Theme {
    /// Colors come from the Save Vault v2 mockup (`save-vault-mockup.html`), which is the visual
    /// source of truth. The accent is picked by the user and applies on top of either theme.
    pub fn new(source: config::Theme, choice: config::Accent) -> Self {
        // Every accent carries its own ink: the text drawn on top of the accent itself. A single
        // ink for all of them would be unreadable on at least one, since they differ in lightness.
        let (accent, accent_ink) = match choice {
            config::Accent::Green => (rgb8!(0x38, 0xe0, 0x8a), rgb8!(0x04, 0x16, 0x0c)),
            config::Accent::Blue => (rgb8!(0x5b, 0x8d, 0xef), rgb8!(0x04, 0x0d, 0x1e)),
            config::Accent::Purple => (rgb8!(0xc7, 0x7d, 0xff), rgb8!(0x18, 0x06, 0x2a)),
            config::Accent::Orange => (rgb8!(0xf5, 0x85, 0x4a), rgb8!(0x25, 0x0d, 0x02)),
            config::Accent::Red => (rgb8!(0xf0, 0x55, 0x6a), rgb8!(0x2a, 0x05, 0x0c)),
        };

        match source {
            config::Theme::Light => Self {
                source,
                background: rgb8!(0xee, 0xf0, 0xf4),
                panel: Color::WHITE,
                accent_ink,
                field: rgb8!(0xf6, 0xf7, 0xfa),
                text: rgb8!(0x1a, 0x1d, 0x27),
                text_inverted: Color::WHITE,
                text_button: Color::WHITE,
                text_skipped: rgb8!(0x5a, 0x61, 0x73),
                text_selection: accent.alpha(0.35),
                positive: accent,
                negative: rgb8!(0xf0, 0x55, 0x6a),
                disabled: rgb8!(0x8b, 0x93, 0xa5),
                navigation: rgb8!(0xe2, 0xe5, 0xec),
                success: rgb8!(0x2f, 0x9e, 0x6a),
                failure: rgb8!(0xd9, 0x3f, 0x53),
                skipped: rgb8!(0xe2, 0xe5, 0xec),
                // Kept blue on purpose: "new" and "different" sit side by side in the file tree,
                // and tying "new" to the accent would collapse them whenever the accent is green.
                added: rgb8!(0x5b, 0x8d, 0xef),
            },
            config::Theme::Dark => Self {
                source,
                background: rgb8!(0x0e, 0x0f, 0x13),
                panel: rgb8!(0x15, 0x17, 0x1d),
                field: rgb8!(0x1a, 0x1d, 0x25),
                text: rgb8!(0xe8, 0xea, 0xf0),
                text_inverted: rgb8!(0x0e, 0x0f, 0x13),
                text_skipped: rgb8!(0x9a, 0xa0, 0xb2),
                disabled: rgb8!(0x6b, 0x72, 0x85),
                navigation: rgb8!(0x20, 0x24, 0x2e),
                success: rgb8!(0x27, 0x8c, 0x5c),
                failure: rgb8!(0xa8, 0x36, 0x47),
                skipped: rgb8!(0x1a, 0x1d, 0x25),
                ..Self::new(config::Theme::Light, choice)
            },
        }
    }
}

impl iced::theme::Base for Theme {
    fn default(_preference: iced::theme::Mode) -> Self {
        <Theme as Default>::default()
    }

    fn mode(&self) -> iced::theme::Mode {
        match self.source {
            config::Theme::Light => iced::theme::Mode::Light,
            config::Theme::Dark => iced::theme::Mode::Dark,
        }
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: self.background,
            text_color: self.text,
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }

    fn name(&self) -> &str {
        match self.source {
            config::Theme::Light => "light",
            config::Theme::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Text {
    #[default]
    Default,
    Failure,
    /// Secondary text, for the context strip under the command bar.
    Muted,
}
impl iced::widget::text::Catalog for Theme {
    type Class<'a> = Text;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, item: &Self::Class<'_>) -> iced::widget::text::Style {
        match item {
            Text::Default => iced::widget::text::Style { color: None },
            Text::Muted => iced::widget::text::Style {
                color: Some(self.text_skipped),
            },
            Text::Failure => iced::widget::text::Style {
                color: Some(self.negative),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Menu;
impl iced::widget::overlay::menu::Catalog for Theme {
    type Class<'a> = Menu;

    fn default<'a>() -> <Self as iced::overlay::menu::Catalog>::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &<Self as iced::overlay::menu::Catalog>::Class<'_>) -> iced::overlay::menu::Style {
        iced::overlay::menu::Style {
            background: self.field.into(),
            border: Border {
                color: self.text.alpha(0.5),
                width: 1.0,
                radius: design::RADIUS.into(),
            },
            text_color: self.text,
            selected_background: self.positive.into(),
            selected_text_color: self.accent_ink,
            shadow: Shadow::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Button {
    #[default]
    Primary,
    Negative,
    GameActionPrimary,
    GameListEntryTitle,
    GameListEntryTitleFailed,
    GameListEntryTitleDisabled,
    GameListEntryTitleUnscanned,
    NavButtonActive,
    NavButtonInactive,
    SideNavActive,
    SideNavInactive,
    /// Outlined button, for what shares a bar with the primary action without competing with it.
    Secondary,
    Badge,
    Bare,
}
impl button::Catalog for Theme {
    type Class<'a> = Button;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        let active = button::Style {
            background: match class {
                Button::Primary | Button::GameActionPrimary => Some(self.positive.into()),
                Button::GameListEntryTitle => Some(self.success.into()),
                Button::GameListEntryTitleFailed => Some(self.failure.into()),
                Button::GameListEntryTitleDisabled => Some(self.skipped.into()),
                Button::GameListEntryTitleUnscanned => None,
                Button::Negative => Some(self.negative.into()),
                Button::NavButtonActive | Button::SideNavActive => Some(self.navigation.alpha(0.9).into()),
                Button::NavButtonInactive | Button::SideNavInactive | Button::Secondary => None,
                Button::Badge => None,
                Button::Bare => None,
            },
            border: Border {
                color: match class {
                    Button::NavButtonActive | Button::NavButtonInactive => self.navigation,
                    Button::Secondary => self.field,
                    _ => Color::TRANSPARENT,
                },
                width: match class {
                    Button::NavButtonActive | Button::NavButtonInactive | Button::Secondary => 1.0,
                    _ => 0.0,
                },
                // One radius for every button. This block used to hand out 9px to the pill-shaped
                // ones and 4px to the rest, which is how two kinds of corner ended up side by
                // side in the same toolbar.
                radius: design::RADIUS.into(),
            },
            text_color: match class {
                Button::GameListEntryTitleDisabled => self.text_skipped.alpha(0.8),
                Button::GameListEntryTitleUnscanned => self.text.alpha(0.8),
                Button::NavButtonActive | Button::NavButtonInactive | Button::Bare | Button::SideNavActive => self.text,
                Button::Secondary => self.text,
                Button::SideNavInactive => self.text_skipped,
                Button::Primary | Button::GameActionPrimary => self.accent_ink,
                _ => self.text_button.alpha(0.8),
            },
            // A shadow with an offset and no blur is a hard black step, not a shadow. It is what
            // made every button look pasted onto the screen.
            shadow: match class {
                Button::NavButtonActive
                | Button::NavButtonInactive
                | Button::SideNavActive
                | Button::SideNavInactive
                | Button::Secondary
                | Button::Bare
                | Button::Badge => design::elevation::FLAT,
                _ => design::elevation::RESTING,
            },
            snap: true,
        };

        match status {
            button::Status::Active => active,
            button::Status::Hovered => button::Style {
                background: match class {
                    Button::NavButtonActive | Button::SideNavActive => Some(self.navigation.alpha(0.95).into()),
                    Button::NavButtonInactive | Button::SideNavInactive => Some(self.navigation.alpha(0.5).into()),
                    Button::Secondary => Some(self.field.into()),
                    _ => active.background,
                },
                border: Border {
                    color: match class {
                        Button::NavButtonActive | Button::NavButtonInactive => self.navigation,
                        _ => active.border.color,
                    },
                    width: match class {
                        Button::NavButtonActive | Button::NavButtonInactive => 1.0,
                        _ => active.border.width,
                    },
                    // Hover does not change the shape of a control. It used to grow the nav button
                    // from 9px to 10px, which reads as a wobble, not as feedback.
                    radius: active.border.radius,
                },
                text_color: match class {
                    Button::GameListEntryTitleDisabled => self.text_skipped,
                    Button::GameListEntryTitleUnscanned
                    | Button::NavButtonActive
                    | Button::NavButtonInactive
                    | Button::SideNavActive
                    | Button::SideNavInactive => self.text,
                    Button::Bare => self.text.alpha(0.9),
                    Button::Secondary => self.text,
                    Button::Primary | Button::GameActionPrimary => self.accent_ink,
                    _ => self.text_button,
                },
                shadow: match class {
                    Button::NavButtonActive
                    | Button::NavButtonInactive
                    | Button::SideNavActive
                    | Button::SideNavInactive
                    | Button::Secondary
                    | Button::Bare
                    | Button::Badge => design::elevation::FLAT,
                    _ => design::elevation::RAISED,
                },
                snap: true,
            },
            // Pressed sits back down on the surface, which is the whole feedback.
            button::Status::Pressed => button::Style {
                shadow: design::elevation::FLAT,
                ..active
            },
            button::Status::Disabled => button::Style {
                shadow: design::elevation::FLAT,
                background: active.background.map(|background| match background {
                    Background::Color(color) => Background::Color(Color {
                        a: color.a * 0.5,
                        ..color
                    }),
                    Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(0.5)),
                }),
                text_color: Color {
                    a: active.text_color.a * 0.5,
                    ..active.text_color
                },
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Container {
    #[default]
    Wrapper,
    Primary,
    /// The navigation column on the left.
    Sidebar,
    /// The strip above the content, carrying the title of the screen.
    Topbar,
    /// The row of column names at the top of a table.
    TableHeader,
    /// One line of a table.
    TableRow,
    ModalForeground,
    ModalBackground,
    GameListEntry,
    Badge,
    BadgeActivated,
    BadgeFaded,
    ChangeBadge {
        change: ScanChange,
        faded: bool,
    },
    DisabledBackup,
    Notification,
    Tooltip,
    /// A card: a self-contained block of content with its own surface.
    ///
    /// It carries a surface **or** a border, never both, because a block that has a different
    /// background and an outline around it reads as two nested boxes.
    Card,
    /// The square a logo or an avatar sits in.
    ///
    /// Artwork supplied by someone else arrives at whatever aspect ratio and whatever background
    /// its author chose. A fixed tile with its own surface is what keeps a row of them aligned.
    LogoTile,
    /// A status chip: one short phrase naming the state of the thing beside it.
    Chip {
        positive: bool,
    },
}
impl container::Catalog for Theme {
    type Class<'a> = Container;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        container::Style {
            background: Some(match class {
                Container::Wrapper => Color::TRANSPARENT.into(),
                Container::GameListEntry => self.field.alpha(design::alpha::HALF).into(),
                Container::ModalBackground => self.field.alpha(0.75).into(),
                Container::Notification => self.field.alpha(0.5).into(),
                Container::TableHeader => self.field.into(),
                Container::TableRow => Color::TRANSPARENT.into(),
                Container::Sidebar => self.panel.into(),
                Container::Topbar => self.background.into(),
                Container::ModalForeground => self.panel.into(),
                Container::Tooltip => self.field.into(),
                Container::DisabledBackup => self.disabled.into(),
                Container::Card => self.panel.into(),
                Container::LogoTile => self.field.into(),
                // The positive chip is filled with the accent and written in the accent's own ink.
                // A tinted-background-plus-accent-text chip would have been quieter, but the accent
                // is a light green: as text it lands near 1.7:1 on the light theme, well under the
                // 4.5:1 that body-sized text has to reach. The ink pairing is the one this theme
                // already guarantees on every accent.
                Container::Chip { positive } => {
                    if *positive {
                        self.positive.into()
                    } else {
                        self.field.alpha(design::alpha::STRONG).into()
                    }
                }
                Container::BadgeActivated => self.negative.into(),
                _ => self.background.into(),
            }),
            border: Border {
                color: match class {
                    Container::Wrapper => Color::TRANSPARENT,
                    Container::GameListEntry | Container::Notification => self.field,
                    Container::Sidebar | Container::Topbar | Container::TableRow => self.field,
                    Container::ChangeBadge { change, faded } => {
                        if *faded {
                            self.disabled
                        } else {
                            match change {
                                ScanChange::New => self.added,
                                ScanChange::Different => self.positive,
                                ScanChange::Removed => self.negative,
                                ScanChange::Same | ScanChange::Unknown => self.disabled,
                            }
                        }
                    }
                    Container::BadgeActivated => self.negative,
                    // The card is a surface, so it takes no border at all. Both would read as two
                    // nested boxes.
                    Container::Card | Container::LogoTile => Color::TRANSPARENT,
                    Container::Chip { .. } => Color::TRANSPARENT,
                    Container::ModalForeground | Container::BadgeFaded => self.disabled,
                    _ => self.text,
                },
                // A block gets a surface or an outline, never both. The sidebar, the top bar and
                // the list entry each used to carry a 1px border on top of a background that
                // already set them apart, which is how the screen ended up looking
                // compartmented into boxes inside boxes.
                width: match class {
                    Container::ModalForeground
                    | Container::Badge
                    | Container::BadgeActivated
                    | Container::BadgeFaded
                    | Container::ChangeBadge { .. } => design::stroke::HAIRLINE,
                    _ => 0.0,
                },
                radius: match class {
                    // A badge is a tag, so it is a pill. Everything else that has a corner at
                    // all has the same one.
                    Container::Badge
                    | Container::BadgeActivated
                    | Container::BadgeFaded
                    | Container::ChangeBadge { .. }
                    | Container::Chip { .. } => design::RADIUS_PILL.into(),
                    Container::ModalForeground
                    | Container::GameListEntry
                    | Container::DisabledBackup
                    | Container::Notification
                    | Container::Tooltip
                    | Container::Card
                    | Container::LogoTile => design::RADIUS.into(),
                    _ => 0.0.into(),
                },
            },
            text_color: match class {
                Container::Wrapper => None,
                Container::DisabledBackup => Some(self.text_inverted),
                Container::ChangeBadge { change, faded } => {
                    if *faded {
                        Some(self.disabled)
                    } else {
                        match change {
                            ScanChange::New => Some(self.added),
                            ScanChange::Different => Some(self.positive),
                            ScanChange::Removed => Some(self.negative),
                            ScanChange::Same | ScanChange::Unknown => Some(self.disabled),
                        }
                    }
                }
                Container::TableHeader => Some(self.text_skipped),
                Container::BadgeActivated => Some(self.text_button),
                Container::Chip { positive } => {
                    if *positive {
                        Some(self.accent_ink)
                    } else {
                        Some(self.text_skipped)
                    }
                }
                Container::Card | Container::LogoTile => None,
                Container::BadgeFaded => Some(self.disabled),
                _ => Some(self.text),
            },
            shadow: match class {
                Container::ModalForeground => design::elevation::MODAL,
                Container::Tooltip => design::elevation::OVERLAY,
                _ => design::elevation::FLAT,
            },
            snap: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Scrollable;
impl scrollable::Catalog for Theme {
    type Class<'a> = Scrollable;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        let active = scrollable::Style {
            auto_scroll: scrollable::AutoScroll {
                background: self.background.into(),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: self.text,
            },
            container: container::Style::default(),
            vertical_rail: scrollable::Rail {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: design::RADIUS_PILL.into(),
                },
                scroller: scrollable::Scroller {
                    // The thumb used to sit at 0.7, which on the dark theme is a near-white bar
                    // running down the side of every screen, louder than the content beside it.
                    background: self.text.alpha(design::alpha::HALF).into(),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: design::RADIUS_PILL.into(),
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: design::RADIUS_PILL.into(),
                },
                scroller: scrollable::Scroller {
                    // The thumb used to sit at 0.7, which on the dark theme is a near-white bar
                    // running down the side of every screen, louder than the content beside it.
                    background: self.text.alpha(design::alpha::HALF).into(),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: design::RADIUS_PILL.into(),
                    },
                },
            },
            gap: None,
        };

        match status {
            scrollable::Status::Active { .. } => active,
            scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
                ..
            } => {
                if !is_horizontal_scrollbar_hovered && !is_vertical_scrollbar_hovered {
                    return active;
                }

                scrollable::Style {
                    vertical_rail: scrollable::Rail {
                        background: Some(self.text.alpha(0.4).into()),
                        border: Border {
                            color: self.text.alpha(0.8),
                            ..active.vertical_rail.border
                        },
                        ..active.vertical_rail
                    },
                    horizontal_rail: scrollable::Rail {
                        background: Some(self.text.alpha(0.4).into()),
                        border: Border {
                            color: self.text.alpha(0.8),
                            ..active.horizontal_rail.border
                        },
                        ..active.horizontal_rail
                    },
                    ..active
                }
            }
            scrollable::Status::Dragged { .. } => self.style(
                _class,
                scrollable::Status::Hovered {
                    is_horizontal_scrollbar_hovered: true,
                    is_vertical_scrollbar_hovered: true,
                    is_horizontal_scrollbar_disabled: false,
                    is_vertical_scrollbar_disabled: false,
                },
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum PickList {
    #[default]
    Primary,
    Backup,
    Popup,
}
impl pick_list::Catalog for Theme {
    type Class<'a> = PickList;

    fn default<'a>() -> <Self as pick_list::Catalog>::Class<'a> {
        Default::default()
    }

    fn style(&self, class: &<Self as pick_list::Catalog>::Class<'_>, status: pick_list::Status) -> pick_list::Style {
        let active = pick_list::Style {
            border: Border {
                color: self.text.alpha(0.7),
                width: 1.0,
                radius: match class {
                    PickList::Primary => design::RADIUS.into(),
                    PickList::Backup | PickList::Popup => design::RADIUS.into(),
                },
            },
            background: self.field.alpha(0.6).into(),
            text_color: self.text,
            placeholder_color: iced::Color::BLACK,
            handle_color: self.text,
        };

        match status {
            pick_list::Status::Active => active,
            pick_list::Status::Hovered => pick_list::Style {
                background: self.field.into(),
                ..active
            },
            pick_list::Status::Opened { .. } => active,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Checkbox;
impl checkbox::Catalog for Theme {
    type Class<'a> = Checkbox;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: checkbox::Status) -> checkbox::Style {
        let active = checkbox::Style {
            background: self.field.alpha(0.6).into(),
            icon_color: self.text,
            border: Border {
                color: self.text.alpha(0.6),
                width: 1.0,
                radius: design::RADIUS.into(),
            },
            text_color: Some(self.text),
        };

        match status {
            checkbox::Status::Active { .. } => active,
            checkbox::Status::Hovered { .. } => checkbox::Style {
                background: self.field.into(),
                ..active
            },
            checkbox::Status::Disabled { .. } => checkbox::Style {
                background: match active.background {
                    Background::Color(color) => Background::Color(Color {
                        a: color.a * 0.5,
                        ..color
                    }),
                    Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(0.5)),
                },
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextInput;
impl text_input::Catalog for Theme {
    type Class<'a> = TextInput;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        let active = text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: Border {
                color: self.text.alpha(0.8),
                width: 1.0,
                radius: design::RADIUS.into(),
            },
            icon: self.negative,
            placeholder: self.text.alpha(0.5),
            value: self.text,
            selection: self.text_selection,
        };

        match status {
            text_input::Status::Active => active,
            text_input::Status::Hovered | text_input::Status::Focused { .. } => text_input::Style {
                border: Border {
                    color: self.text,
                    ..active.border
                },
                ..active
            },
            text_input::Status::Disabled => text_input::Style {
                background: self.disabled.into(),
                value: self.text.alpha(0.5),
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressBar;
impl iced::widget::progress_bar::Catalog for Theme {
    type Class<'a> = ProgressBar;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>) -> iced::widget::progress_bar::Style {
        iced::widget::progress_bar::Style {
            background: self.disabled.into(),
            bar: self.added.into(),
            border: Border {
                radius: design::RADIUS.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextEditor;
impl text_editor::Catalog for Theme {
    type Class<'a> = TextEditor;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: text_editor::Status) -> text_editor::Style {
        let active = text_editor::Style {
            background: self.field.alpha(0.3).into(),
            border: Border {
                radius: design::RADIUS.into(),
                width: 1.0,
                color: self.field,
            },
            placeholder: self.text_skipped,
            value: self.text,
            selection: self.text_selection,
        };

        match status {
            text_editor::Status::Active => active,
            text_editor::Status::Hovered => text_editor::Style {
                border: Border {
                    color: self.text,
                    ..active.border
                },
                ..active
            },
            text_editor::Status::Focused { .. } => text_editor::Style {
                border: Border {
                    color: self.text,
                    ..active.border
                },
                ..active
            },
            text_editor::Status::Disabled => text_editor::Style {
                background: Background::Color(self.disabled),
                value: active.placeholder,
                ..active
            },
        }
    }
}

/// The emulator logos are the only SVGs in the app, and they are other people's artwork.
///
/// So this catalog deliberately does nothing: `color: None` keeps every logo in the colours its
/// project drew it in. Tinting them to the theme would turn seven recognisable marks into seven
/// identical silhouettes, which is the opposite of what they are on the screen for.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svg;
impl iced::widget::svg::Catalog for Theme {
    type Class<'a> = Svg;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, _status: iced::widget::svg::Status) -> iced::widget::svg::Style {
        iced::widget::svg::Style { color: None }
    }
}
