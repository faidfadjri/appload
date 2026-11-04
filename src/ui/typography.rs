use super::theme::*;
use eframe::egui::{FontFamily, FontId, RichText};

/// Standardized SaaS typography tokens with strict weight mapping and integer pixel sizes.
/// Exactly six base sizes: 22, 16, 14, 13, 12, 11 (plus optional 20 for large technical version headers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typo {
    /// 22px semibold — Page title (one per page)
    H1,
    /// 16px semibold — Section title, modal title
    H2,
    /// 14px semibold — Card title, environment name, table emphasis
    H3,
    /// 13px regular — Paragraphs, input text, table cells
    Body,
    /// 13px medium — Buttons, form labels, tabs, nav items
    Label,
    /// 12px regular — Secondary text, helper text, metadata
    Small,
    /// 12px medium — Table header cells, small buttons
    SmallMedium,
    /// 11px medium — Badges and pills, sidebar section overlines
    Caption,
    /// 12px mono regular — Versions, hashes, paths, commands, keys
    Mono,
    /// 11px mono medium — Version badges in the sidebar
    MonoSm,
    /// 13px mono regular — Code/command input fields, terminal
    Code,
    /// 20px mono medium — Large version display in environment cards
    MonoLg,
}

impl Typo {
    #[inline]
    pub fn size(self) -> f32 {
        match self {
            Typo::H1 => 22.0,
            Typo::H2 => 16.0,
            Typo::H3 => 14.0,
            Typo::Body | Typo::Label | Typo::Code => 13.0,
            Typo::Small | Typo::SmallMedium | Typo::Mono => 12.0,
            Typo::Caption | Typo::MonoSm => 11.0,
            Typo::MonoLg => 20.0,
        }
    }

    #[inline]
    pub fn family(self) -> FontFamily {
        match self {
            Typo::H1 | Typo::H2 | Typo::H3 => FontFamily::Name("semibold".into()),
            Typo::Body | Typo::Small => FontFamily::Proportional,
            Typo::Label | Typo::SmallMedium | Typo::Caption => FontFamily::Name("medium".into()),
            Typo::Mono | Typo::Code => FontFamily::Monospace,
            Typo::MonoSm | Typo::MonoLg => FontFamily::Name("mono-medium".into()),
        }
    }

    #[inline]
    pub fn font(self) -> FontId {
        FontId::new(self.size(), self.family())
    }

    pub fn rich(self, text: impl Into<String>) -> RichText {
        let t = text.into();
        let color = match self {
            Typo::H1 | Typo::H2 | Typo::H3 | Typo::Body | Typo::Label => TEXT_PRIMARY,
            Typo::Small | Typo::SmallMedium => TEXT_SECONDARY,
            Typo::Caption => TEXT_MUTED,
            Typo::Mono | Typo::MonoSm | Typo::Code | Typo::MonoLg => TEXT_PRIMARY,
        };
        RichText::new(t).font(self.font()).color(color)
    }
}

pub mod typo {
    use super::*;

    #[inline]
    pub fn h1(text: impl Into<String>) -> RichText {
        Typo::H1.rich(text)
    }

    #[inline]
    pub fn h2(text: impl Into<String>) -> RichText {
        Typo::H2.rich(text)
    }

    #[inline]
    pub fn h3(text: impl Into<String>) -> RichText {
        Typo::H3.rich(text)
    }

    #[inline]
    pub fn body(text: impl Into<String>) -> RichText {
        Typo::Body.rich(text)
    }

    #[inline]
    pub fn label(text: impl Into<String>) -> RichText {
        Typo::Label.rich(text)
    }

    #[inline]
    pub fn small(text: impl Into<String>) -> RichText {
        Typo::Small.rich(text)
    }

    #[inline]
    pub fn small_medium(text: impl Into<String>) -> RichText {
        Typo::SmallMedium.rich(text)
    }

    #[inline]
    pub fn caption(text: impl Into<String>) -> RichText {
        Typo::Caption.rich(text)
    }

    #[inline]
    pub fn mono(text: impl Into<String>) -> RichText {
        Typo::Mono.rich(text)
    }

    #[inline]
    pub fn mono_sm(text: impl Into<String>) -> RichText {
        Typo::MonoSm.rich(text)
    }

    #[inline]
    pub fn code(text: impl Into<String>) -> RichText {
        Typo::Code.rich(text)
    }

    #[inline]
    pub fn mono_lg(text: impl Into<String>) -> RichText {
        Typo::MonoLg.rich(text)
    }
}

pub trait RichTextExt {
    fn typo(self, t: Typo) -> RichText;
}

impl RichTextExt for RichText {
    #[inline]
    fn typo(self, t: Typo) -> RichText {
        self.font(t.font())
    }
}
