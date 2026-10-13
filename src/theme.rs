use super::*;
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub(super) struct Theme {
    pub ink: Color32,
    pub muted: Color32,
    pub panel: Color32,
    pub canvas: Color32,
    pub stroke: Color32,
    pub accent: Color32,
    pub surface: Color32,
    pub hover: Color32,
    pub callouts: [([u8; 3], [u8; 3]); 5],
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            ink: Color32::from_gray(227),
            muted: Color32::from_gray(160),
            panel: Color32::from_gray(32),
            canvas: Color32::from_gray(25),
            stroke: Color32::from_gray(55),
            accent: Color32::from_rgb(205, 211, 215),
            surface: Color32::from_gray(37),
            hover: Color32::from_gray(43),
            callouts: [
                ([91, 155, 213], [27, 39, 51]),
                ([92, 184, 92], [27, 42, 31]),
                ([224, 160, 48], [46, 38, 22]),
                ([217, 83, 79], [47, 27, 27]),
                ([138, 138, 138], [38, 38, 42]),
            ],
        }
    }
}
pub(super) const ACCENTS: [(&str, [u8; 3]); 9] = [
    ("Neutral", [205, 211, 215]),
    ("Blue", [103, 170, 224]),
    ("Green", [112, 191, 145]),
    ("Amber", [221, 177, 95]),
    ("Red", [221, 123, 123]),
    ("Teal", [101, 191, 189]),
    ("Rose", [217, 149, 170]),
    ("Orange", [222, 155, 102]),
    ("Slate", [153, 173, 192]),
];
impl Theme {
    pub fn from_settings(settings: &crate::settings::Appearance) -> Self {
        let mut theme = Self::default();
        let color = ACCENTS[settings.accent.min(8)].1;
        theme.accent = Color32::from_rgb(color[0], color[1], color[2]);
        if settings.dim_page {
            theme.canvas = Color32::from_gray(29);
        }
        if settings.high_contrast {
            theme.ink = Color32::WHITE;
            theme.muted = Color32::from_gray(195);
            theme.stroke = Color32::from_gray(120);
        }
        theme
    }
}
thread_local! { static CURRENT: Cell<Theme> = Cell::new(Theme::default()); }
pub(super) fn current() -> Theme {
    CURRENT.with(Cell::get)
}
pub(super) fn apply(theme: Theme) {
    CURRENT.with(|value| value.set(theme));
}
