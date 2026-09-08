//! Save Vault's visual foundation, version 3.
//!
//! # Why this file exists
//!
//! Before it, no screen shared a scale with any other. Measured on the code it replaces:
//!
//! | Axis | Distinct values found | Now |
//! |---|---|---|
//! | Font size | 8 explicit (11, 12, 13, 14, 15, 16, 20, 25) plus iced's implicit 16 | 4 |
//! | Padding | 19 (1, 2, 3, 5, 6, 8, 10, 11, 12, 14, 15, 16, 18, 20, 24, 30, 35, 40, 0) | 6 |
//! | Spacing | 9 (0, 2, 4, 5, 8, 10, 12, 15, 20) | 6 |
//! | Corner radius | 7 (0, 2, 4, 5, 9, 10, 20) | 1, plus the pill |
//!
//! None of that was decided. It accumulated, one screen at a time, and the result is an interface
//! where nothing lines up with anything and the body text sits a step too large for a desktop tool
//! that spends its screen showing folder paths and file counts.
//!
//! # How to go back
//!
//! Version 2 was not a file: it was those literals, spread across the screens. So the way back is
//! the git revert of the commit that introduced this module, not a flag. What this module does
//! guarantee is that from here on the foundation moves in **one** place: no screen reads a bare
//! number any more, so changing `text::BODY` here changes the whole app, and nothing else has to be
//! hunted down and edited.
//!
//! # The rules this encodes
//!
//! - Emphasis is carried by **weight and colour**, never by size alone. That is why the type scale
//!   is short, and why [`font::TEXT_STRONG`](crate::gui::font::TEXT_STRONG) exists at all.
//! - Every space is a multiple of 4. Space **between** groups is at least twice the space
//!   **inside** them.
//! - One corner radius. Mixing square and round corners in one interface always looks worse.
//! - Elevation is chosen by "where does this sit on the z axis", never by "which shadow looks nice".

/// The type scale.
///
/// Four steps, on purpose. The house web ramp (12 · 14 · 16 · 18 · 20 · 24 · 30) keeps ~25% between
/// neighbours, which works when the base is 16 and the canvas is a page. A desktop tool has a
/// shorter useful range: the body has to stay small enough for a folder path to fit on one line,
/// and the largest thing on screen is a screen title, not a headline. So the ramp is tight, in the
/// shape Windows itself uses.
pub mod text {
    /// Metadata that is only read when looked for: timestamp, table column name, version string,
    /// badge. Never a sentence the user has to read to understand the screen.
    pub const CAPTION: f32 = 12.0;

    /// The default. Anything that does not say otherwise renders at this size, because
    /// `default_text_size` is set to it in `gui::run`.
    ///
    /// It used to be 16 (iced's own default), which on Windows at 125% scaling draws at 20 physical
    /// pixels. That is the size the complaint was about.
    pub const BODY: f32 = 14.0;

    /// The name of a card, a group heading, a section label. One step above the body, and paired
    /// with [`font::TEXT_STRONG`](crate::gui::font::TEXT_STRONG) instead of a bigger jump in size.
    pub const SUBTITLE: f32 = 16.0;

    /// The screen title in the top bar. The largest type in the app, and there is exactly one of it
    /// on screen at a time.
    pub const TITLE: f32 = 20.0;
}

/// Line height, as a multiplier of the font size.
///
/// Inversely proportional to the size: small text needs air between lines, large text needs less or
/// it drifts apart.
pub mod leading {
    /// Body copy, and anything that can wrap to a second line.
    pub const BODY: f32 = 1.45;

    /// A single-line label inside a control, where leading only makes the control taller.
    pub const TIGHT: f32 = 1.0;

    /// Titles, which are short and already have presence.
    pub const TITLE: f32 = 1.2;
}

/// The spacing scale. Every value is a multiple of 4, and every gap in the app is one of these.
///
/// The rule matters more than the numbers: the space **around** a group is at least twice the space
/// **inside** it. Equal spacing inside and between groups is what makes it impossible to tell which
/// label belongs to which field, and that is a functional defect, not a matter of taste.
pub mod space {
    /// Between an icon and its label, or between two lines of the same block.
    pub const XS: f32 = 4.0;
    /// Inside a control, and between items that read as one thing.
    pub const SM: f32 = 8.0;
    /// Between the rows of a form, between the fields of a card.
    pub const MD: f32 = 12.0;
    /// Inside a card, and between a heading and what it heads.
    pub const LG: f32 = 16.0;
    /// Between cards, and between the sections of a screen.
    pub const XL: f32 = 24.0;
    /// The outer margin of a screen.
    pub const XXL: f32 = 32.0;
}

/// The one corner radius.
pub const RADIUS: f32 = 6.0;

/// The pill, for badges and chips.
///
/// This is not a second radius: a pill is a *shape*. A badge drawn at [`RADIUS`] reads as a small
/// rectangle rather than a tag. Anything that is not a badge or a chip uses [`RADIUS`].
pub const RADIUS_PILL: f32 = 999.0;

/// Border widths.
pub mod stroke {
    /// A hairline that only separates content. It does not have to reach 3:1 contrast.
    pub const HAIRLINE: f32 = 1.0;

    /// The border that is the only thing identifying a control. It has to reach 3:1 contrast, and
    /// at 1px a colour soft enough to go unnoticed disappears entirely, so it goes to 2px instead
    /// of getting louder.
    #[allow(unused)]
    pub const CONTROL: f32 = 2.0;
}

/// The five steps of elevation. Pick by where the thing sits on the z axis.
pub mod elevation {
    use iced::{Color, Shadow, Vector};

    const fn shadow(y: f32, blur: f32, alpha: f32) -> Shadow {
        Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
            offset: Vector::new(0.0, y),
            blur_radius: blur,
        }
    }

    /// Flat on the surface: a card, a table row.
    pub const FLAT: Shadow = shadow(0.0, 0.0, 0.0);
    /// Barely lifted: a button.
    pub const RESTING: Shadow = shadow(1.0, 3.0, 0.20);
    /// A dropdown, a pick list menu.
    pub const RAISED: Shadow = shadow(4.0, 6.0, 0.24);
    /// A popover, a tooltip, a notification.
    pub const OVERLAY: Shadow = shadow(10.0, 24.0, 0.28);
    /// A modal, which has to read as detached from everything behind it.
    pub const MODAL: Shadow = shadow(15.0, 35.0, 0.32);
}

/// Fixed opacities, decided once, so no screen invents a fourth "slightly transparent".
///
/// The whole ladder stays declared even while only some rungs are in use. That is the point of
/// fixing them: the next screen that needs a translucent surface picks a rung, instead of typing
/// 0.15 because it looked about right that afternoon.
#[allow(unused)]
pub mod alpha {
    pub const FAINT: f32 = 0.05;
    pub const SUBTLE: f32 = 0.1;
    pub const SOFT: f32 = 0.2;
    pub const HALF: f32 = 0.4;
    pub const STRONG: f32 = 0.6;
    pub const HEAVY: f32 = 0.8;
}

/// Control geometry.
pub mod control {
    /// One height for every control: button, text field, pick list, tab.
    ///
    /// A size variant changes the width and the type, never the height. Three button heights in one
    /// app is the most visible symptom of an interface assembled by accumulation, and this app had
    /// them.
    pub const HEIGHT: f32 = 32.0;

    /// Horizontal padding inside a control that carries a label.
    pub const PAD_X: f32 = 12.0;

    /// Horizontal padding inside a control that carries only an icon, so it comes out square.
    pub const PAD_X_ICON: f32 = 8.0;
}

/// Icon sizes.
///
/// The Material Icons face is drawn for 16 to 24 pixels. Blowing one up past that is what makes an
/// icon look crude, so there is no size above [`LG`] here.
pub mod icon {
    /// Inline with body text.
    pub const SM: f32 = 16.0;
    /// The default: inside a button, in a toolbar.
    pub const MD: f32 = 20.0;
    /// Leading an item in a list or a card header.
    pub const LG: f32 = 24.0;
}

/// The navigation column on the left.
///
/// Fixed, never a percentage: a percentage column is wasteful on a monitor and truncates on a
/// laptop. The content beside it is what flexes.
pub const SIDEBAR_WIDTH: f32 = 232.0;

/// The strip above the content that carries the screen title.
pub const TOPBAR_HEIGHT: f32 = 56.0;

/// The emulator card on the Emulators screen.
pub mod emulator_card {
    /// The tile the emulator's logo sits in.
    ///
    /// The logos are the projects' own artwork, at very different aspect ratios, and some are
    /// light-on-transparent while others are dark-on-transparent. A tile of a fixed size with its
    /// own surface behind it is what stops them from wrecking the grid and from vanishing into the
    /// theme.
    pub const TILE: f32 = 44.0;

    /// The logo inside the tile. Smaller than the tile, so every logo gets the same optical weight
    /// no matter how much padding its own artwork already carries.
    pub const LOGO: f32 = 28.0;
}
