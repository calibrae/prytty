use crate::token::TokenKind;

/// RGB color + text style for a token kind.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub fg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
}

impl Style {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            fg: (r, g, b),
            bold: false,
            italic: false,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// A color theme mapping token kinds to styles.
/// Default is a VS Code Dark+ inspired palette (looks good on #1e1e1e backgrounds).
#[derive(Debug, Clone)]
pub struct Theme {
    pub keyword: Style,
    pub type_: Style,
    pub function: Style,
    pub string: Style,
    pub number: Style,
    pub comment: Style,
    pub operator: Style,
    pub punctuation: Style,
    pub variable: Style,
    pub constant: Style,
    pub attribute: Style,
    pub builtin: Style,
    pub label: Style,
    pub key: Style,
    pub escape: Style,
    pub url: Style,
    pub path: Style,
    pub ip: Style,
    pub timestamp: Style,
    pub plain: Style,
}

impl Default for Theme {
    fn default() -> Self {
        // VS Code Dark+ inspired
        Self {
            keyword: Style::new(197, 134, 192).bold(),   // purple
            type_: Style::new(78, 201, 176),              // teal
            function: Style::new(220, 220, 170),          // light yellow
            string: Style::new(206, 145, 120),            // orange-brown
            number: Style::new(181, 206, 168),            // light green
            comment: Style::new(106, 153, 85).italic(),   // green, dim
            operator: Style::new(212, 212, 212),          // light gray
            punctuation: Style::new(150, 150, 150),       // gray
            variable: Style::new(156, 220, 254),          // light blue
            constant: Style::new(100, 150, 224),          // blue
            attribute: Style::new(156, 220, 254),         // light blue
            builtin: Style::new(78, 201, 176),            // teal
            label: Style::new(220, 220, 170).bold(),      // yellow, bold
            key: Style::new(156, 220, 254),               // light blue
            escape: Style::new(215, 186, 125),            // gold
            url: Style::new(100, 150, 224).italic(),      // blue, underline-ish
            path: Style::new(156, 220, 254),              // light blue
            ip: Style::new(181, 206, 168),                // light green
            timestamp: Style::new(106, 153, 85),          // dim green
            plain: Style::new(212, 212, 212),             // default fg
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_keyword_is_bold() {
        let theme = Theme::default();
        assert!(theme.keyword.bold, "keyword should be bold");
    }

    #[test]
    fn default_theme_comment_is_italic() {
        let theme = Theme::default();
        assert!(theme.comment.italic, "comment should be italic");
    }

    #[test]
    fn default_theme_all_fg_colors_nonzero() {
        let theme = Theme::default();
        // Every style should have a non-black foreground (i.e. intentional color)
        let styles = [
            theme.keyword, theme.type_, theme.function, theme.string,
            theme.number, theme.comment, theme.operator, theme.punctuation,
            theme.variable, theme.constant, theme.attribute, theme.builtin,
            theme.label, theme.key, theme.escape, theme.url, theme.path,
            theme.ip, theme.timestamp, theme.plain,
        ];
        for style in &styles {
            let (r, g, b) = style.fg;
            assert!(r > 0 || g > 0 || b > 0, "style has zero fg color: {style:?}");
        }
    }

    #[test]
    fn style_for_maps_all_token_kinds() {
        use crate::token::TokenKind;
        let theme = Theme::default();
        let kinds = [
            TokenKind::Keyword, TokenKind::Type, TokenKind::Function,
            TokenKind::String, TokenKind::Number, TokenKind::Comment,
            TokenKind::Operator, TokenKind::Punctuation, TokenKind::Variable,
            TokenKind::Constant, TokenKind::Attribute, TokenKind::Builtin,
            TokenKind::Label, TokenKind::Key, TokenKind::Escape,
            TokenKind::Url, TokenKind::Path, TokenKind::Ip,
            TokenKind::Timestamp, TokenKind::Plain,
        ];
        for kind in &kinds {
            // Just ensure it doesn't panic and returns some style
            let _style = theme.style_for(*kind);
        }
    }

    #[test]
    fn style_for_keyword_matches_theme_keyword() {
        use crate::token::TokenKind;
        let theme = Theme::default();
        let s = theme.style_for(TokenKind::Keyword);
        assert_eq!(s.fg, theme.keyword.fg);
        assert_eq!(s.bold, theme.keyword.bold);
    }

    #[test]
    fn style_for_string_matches_theme_string() {
        use crate::token::TokenKind;
        let theme = Theme::default();
        let s = theme.style_for(TokenKind::String);
        assert_eq!(s.fg, theme.string.fg);
    }

    #[test]
    fn style_new_defaults_bold_italic_false() {
        let s = Style::new(100, 150, 200);
        assert!(!s.bold);
        assert!(!s.italic);
        assert_eq!(s.fg, (100, 150, 200));
    }

    #[test]
    fn style_bold_sets_bold() {
        let s = Style::new(100, 150, 200).bold();
        assert!(s.bold);
        assert!(!s.italic);
    }

    #[test]
    fn style_italic_sets_italic() {
        let s = Style::new(100, 150, 200).italic();
        assert!(s.italic);
        assert!(!s.bold);
    }
}

impl Theme {
    /// Look up a theme by name. Returns default (dark+) for unknown names.
    pub fn by_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "solarized" | "solarized-dark" => Self::solarized_dark(),
            "monokai" => Self::monokai(),
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin_mocha(),
            "nord" => Self::nord(),
            "dracula" => Self::dracula(),
            _ => Self::default(),
        }
    }

    /// Solarized Dark — Ethan Schoonover's classic.
    /// Base03 (#002b36) background assumed.
    pub fn solarized_dark() -> Self {
        // Solarized accent colors
        let yellow  = (181, 137, 0);    // #b58900
        let orange  = (203, 75, 22);    // #cb4b16
        let red     = (220, 50, 47);    // #dc322f
        let magenta = (211, 54, 130);   // #d33682
        let violet  = (108, 113, 196);  // #6c71c4
        let blue    = (38, 139, 210);   // #268bd2
        let cyan    = (42, 161, 152);   // #2aa198
        let green   = (133, 153, 0);    // #859900

        // Solarized content tones
        let base0  = (131, 148, 150);   // #839496 — body text
        let base01 = (88, 110, 117);    // #586e75 — comments

        Self {
            keyword: Style::new(green.0, green.1, green.2).bold(),
            type_: Style::new(yellow.0, yellow.1, yellow.2),
            function: Style::new(blue.0, blue.1, blue.2),
            string: Style::new(cyan.0, cyan.1, cyan.2),
            number: Style::new(magenta.0, magenta.1, magenta.2),
            comment: Style::new(base01.0, base01.1, base01.2).italic(),
            operator: Style::new(base0.0, base0.1, base0.2),
            punctuation: Style::new(base01.0, base01.1, base01.2),
            variable: Style::new(blue.0, blue.1, blue.2),
            constant: Style::new(violet.0, violet.1, violet.2),
            attribute: Style::new(orange.0, orange.1, orange.2),
            builtin: Style::new(red.0, red.1, red.2),
            label: Style::new(orange.0, orange.1, orange.2).bold(),
            key: Style::new(blue.0, blue.1, blue.2),
            escape: Style::new(orange.0, orange.1, orange.2),
            url: Style::new(violet.0, violet.1, violet.2).italic(),
            path: Style::new(cyan.0, cyan.1, cyan.2),
            ip: Style::new(magenta.0, magenta.1, magenta.2),
            timestamp: Style::new(base01.0, base01.1, base01.2),
            plain: Style::new(base0.0, base0.1, base0.2),
        }
    }

    /// Monokai — the Sublime Text classic.
    pub fn monokai() -> Self {
        Self {
            keyword: Style::new(249, 38, 114).bold(),     // pink
            type_: Style::new(102, 217, 239),              // cyan
            function: Style::new(166, 226, 46),            // green
            string: Style::new(230, 219, 116),             // yellow
            number: Style::new(174, 129, 255),             // purple
            comment: Style::new(117, 113, 94).italic(),    // gray
            operator: Style::new(249, 38, 114),            // pink
            punctuation: Style::new(248, 248, 242),        // white
            variable: Style::new(248, 248, 242),           // white
            constant: Style::new(174, 129, 255),           // purple
            attribute: Style::new(166, 226, 46),           // green
            builtin: Style::new(102, 217, 239),            // cyan
            label: Style::new(230, 219, 116).bold(),       // yellow
            key: Style::new(249, 38, 114),                 // pink
            escape: Style::new(174, 129, 255),             // purple
            url: Style::new(102, 217, 239).italic(),       // cyan
            path: Style::new(166, 226, 46),                // green
            ip: Style::new(174, 129, 255),                 // purple
            timestamp: Style::new(117, 113, 94),           // gray
            plain: Style::new(248, 248, 242),              // white
        }
    }

    /// Catppuccin Mocha — the pastel dark theme.
    pub fn catppuccin_mocha() -> Self {
        Self {
            keyword: Style::new(203, 166, 247).bold(),    // mauve
            type_: Style::new(250, 179, 135),              // peach
            function: Style::new(137, 180, 250),           // blue
            string: Style::new(166, 227, 161),             // green
            number: Style::new(250, 179, 135),             // peach
            comment: Style::new(108, 112, 134).italic(),   // overlay0
            operator: Style::new(205, 214, 244),           // text
            punctuation: Style::new(147, 153, 178),        // overlay2
            variable: Style::new(205, 214, 244),           // text
            constant: Style::new(250, 179, 135),           // peach
            attribute: Style::new(249, 226, 175),          // yellow
            builtin: Style::new(245, 194, 231),            // pink
            label: Style::new(249, 226, 175).bold(),       // yellow
            key: Style::new(137, 180, 250),                // blue
            escape: Style::new(245, 194, 231),             // pink
            url: Style::new(116, 199, 236).italic(),       // sapphire
            path: Style::new(148, 226, 213),               // teal
            ip: Style::new(166, 227, 161),                 // green
            timestamp: Style::new(108, 112, 134),          // overlay0
            plain: Style::new(205, 214, 244),              // text
        }
    }

    /// Nord — the Arctic color palette.
    pub fn nord() -> Self {
        Self {
            keyword: Style::new(129, 161, 193).bold(),    // nord9
            type_: Style::new(143, 188, 187),              // nord7
            function: Style::new(136, 192, 208),           // nord8
            string: Style::new(163, 190, 140),             // nord14
            number: Style::new(180, 142, 173),             // nord15
            comment: Style::new(76, 86, 106).italic(),     // nord3
            operator: Style::new(216, 222, 233),           // nord4
            punctuation: Style::new(76, 86, 106),          // nord3
            variable: Style::new(216, 222, 233),           // nord4
            constant: Style::new(180, 142, 173),           // nord15
            attribute: Style::new(235, 203, 139),          // nord13
            builtin: Style::new(143, 188, 187),            // nord7
            label: Style::new(235, 203, 139).bold(),       // nord13
            key: Style::new(129, 161, 193),                // nord9
            escape: Style::new(208, 135, 112),             // nord12
            url: Style::new(136, 192, 208).italic(),       // nord8
            path: Style::new(163, 190, 140),               // nord14
            ip: Style::new(143, 188, 187),                 // nord7
            timestamp: Style::new(76, 86, 106),            // nord3
            plain: Style::new(216, 222, 233),              // nord4
        }
    }

    /// Dracula — the dark theme for vampires.
    pub fn dracula() -> Self {
        Self {
            keyword: Style::new(255, 121, 198).bold(),    // pink
            type_: Style::new(139, 233, 253),              // cyan
            function: Style::new(80, 250, 123),            // green
            string: Style::new(241, 250, 140),             // yellow
            number: Style::new(189, 147, 249),             // purple
            comment: Style::new(98, 114, 164).italic(),    // comment
            operator: Style::new(255, 121, 198),           // pink
            punctuation: Style::new(248, 248, 242),        // foreground
            variable: Style::new(248, 248, 242),           // foreground
            constant: Style::new(189, 147, 249),           // purple
            attribute: Style::new(80, 250, 123),           // green
            builtin: Style::new(139, 233, 253),            // cyan
            label: Style::new(241, 250, 140).bold(),       // yellow
            key: Style::new(139, 233, 253),                // cyan
            escape: Style::new(255, 184, 108),             // orange
            url: Style::new(139, 233, 253).italic(),       // cyan
            path: Style::new(80, 250, 123),                // green
            ip: Style::new(189, 147, 249),                 // purple
            timestamp: Style::new(98, 114, 164),           // comment
            plain: Style::new(248, 248, 242),              // foreground
        }
    }

    pub fn style_for(&self, kind: TokenKind) -> Style {
        match kind {
            TokenKind::Keyword => self.keyword,
            TokenKind::Type => self.type_,
            TokenKind::Function => self.function,
            TokenKind::String => self.string,
            TokenKind::Number => self.number,
            TokenKind::Comment => self.comment,
            TokenKind::Operator => self.operator,
            TokenKind::Punctuation => self.punctuation,
            TokenKind::Variable => self.variable,
            TokenKind::Constant => self.constant,
            TokenKind::Attribute => self.attribute,
            TokenKind::Builtin => self.builtin,
            TokenKind::Label => self.label,
            TokenKind::Key => self.key,
            TokenKind::Escape => self.escape,
            TokenKind::Url => self.url,
            TokenKind::Path => self.path,
            TokenKind::Ip => self.ip,
            TokenKind::Timestamp => self.timestamp,
            TokenKind::Plain => self.plain,
        }
    }
}
