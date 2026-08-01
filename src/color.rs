//! Terminal color support.
//!
//! Colors are emitted only when stdout is a terminal (not a pipe/file) and
//! `NO_COLOR` is unset/empty and `TERM` is not `dumb` — the conventional
//! opt-out signals. When disabled, all helpers return their input unchanged.

use std::io::IsTerminal;

/// How colour output is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorMode {
    /// Emit colour only when stdout is a terminal and `NO_COLOR` is unset.
    Auto,
    /// Always emit colour, even when piping to a file or another command.
    Always,
    /// Never emit colour.
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    const fn hex(hex: u32) -> Self {
        Rgb {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// A darker shade of this colour, used as the background when a language
    /// needs a two-tone treatment. GitHub's colour map gives one colour per
    /// language, so we derive the background by scaling toward black.
    pub fn darker(self) -> Self {
        Rgb {
            r: (self.r as u16 * 2 / 3) as u8,
            g: (self.g as u16 * 2 / 3) as u8,
            b: (self.b as u16 * 2 / 3) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    enabled: bool,
}

impl Colors {
    pub fn detect() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
        let enabled = std::io::stdout().is_terminal() && !no_color && !dumb;
        Colors { enabled }
    }

    pub fn from_mode(mode: ColorMode) -> Self {
        let enabled = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
                let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
                std::io::stdout().is_terminal() && !no_color && !dumb
            }
        };
        Colors { enabled }
    }

    #[cfg(test)]
    pub fn force(enabled: bool) -> Self {
        Colors { enabled }
    }

    fn wrap(&self, s: &str, code: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    /// True-colour foreground from an RGB triple.
    pub fn fg(&self, s: &str, c: Rgb) -> String {
        self.wrap(s, &format!("38;2;{};{};{}", c.r, c.g, c.b))
    }

    /// Foreground over a dark background (two-colour logo).
    pub fn on(&self, s: &str, fg: Rgb, bg: Rgb) -> String {
        self.wrap(s, &format!("38;2;{};{};{};48;2;{};{};{}", fg.r, fg.g, fg.b, bg.r, bg.g, bg.b))
    }

    /// Standard 8-colour dim/gray foreground.
    pub fn gray(&self, s: &str) -> String {
        self.wrap(s, "90")
    }

    /// Bright foreground by palette index (1=red..7=white). Emits
    /// `\x1b[9{n}m` so text stays readable on dark backgrounds.
    pub fn ansi(&self, s: &str, n: u8) -> String {
        let code = 90 + n.min(7);
        self.wrap(s, &format!("{code}"))
    }

    /// Explicit SGR background colour code (e.g. 40=black, 100=dark gray).
    /// Only dark backgrounds are used — light backgrounds are never emitted.
    pub fn bg_code(&self, s: &str, code: u8) -> String {
        self.wrap(s, &format!("{code}"))
    }

    /// Bright foreground (1=red..7=white) over a dark SGR background code.
    /// e.g. `on_bg(s, 4, 100)` = bright blue text on dark gray.
    pub fn on_bg(&self, s: &str, fg: u8, bg_code: u8) -> String {
        let f = 90 + fg.min(7);
        self.wrap(s, &format!("{f};{bg_code}"))
    }

    /// Bright foreground over a 256-colour background (`48;5;N`). Use the
    /// xterm greyscale ramp (232..255) for dark shades: lower = darker.
    pub fn on_bg256(&self, s: &str, fg: u8, bg256: u8) -> String {
        let f = 90 + fg.min(7);
        self.wrap(s, &format!("{f};48;5;{bg256}"))
    }
}

/// A language's colours: the foreground is the GitHub colour, and `bg` is a
/// derived darker shade used as the background for two-tone rendering (the
/// "darker background, lighter foreground" treatment requested).
#[derive(Debug, Clone, Copy)]
pub struct LogoColors {
    pub fg: Rgb,
    pub bg: Option<Rgb>,
}

/// The GitHub colour for a language (from `ozh/github-colors`), keyed by the
/// display name used in `Report`. All foreground colours are unique both as
/// hex and as the nearest 256-colour palette index, so any two languages
/// render distinctly even on 256-colour terminals.
pub fn logo_colors(name: &str) -> Option<LogoColors> {
    let (fg, two_tone) = match name {
        // Programming languages. Foregrounds are unique in hex and 256-index;
        // `two_tone` gives a darker derived background.
        "Rust" => (0xDEA584, false),
        "C" => (0x555555, false),
        "C++" => (0xF34B7D, true),
        "Python" => (0x3572A5, true),
        "JavaScript" => (0xF1E05A, true),
        "Bash" => (0x89E051, false),
        "Haskell" => (0x5E5086, true),
        "OCaml" => (0xEF7A08, false),
        "Elm" => (0x60B5CC, false),
        "Go" => (0x00ADD8, true),
        "TypeScript" => (0x3178C6, true),
        "TSX" => (0x2F78C6, false),
        "Java" => (0xB07219, false),
        "Scala" => (0xC22D40, false),
        "Ada" => (0x02F88C, false),
        "Agda" => (0x315665, false),
        "C#" => (0x7355DD, false),
        "Dart" => (0x00B4AB, false),
        "Elixir" => (0x744A7E, false),
        "Erlang" => (0xB83998, false),
        "Fish" => (0x4AAE47, false),
        "Fortran" => (0x4D2FB1, false),
        "F#" => (0xB845FC, false),
        "Gleam" => (0xFFAFF3, false),
        "GLSL" => (0x5686A5, false),
        "GraphQL" => (0xE10098, false),
        "Groovy" => (0x429CB8, false),
        "Julia" => (0xA270BA, false),
        "Lua" => (0x000080, false),
        "Make" => (0x427819, false),
        "Nix" => (0x7E7EFF, false),
        "Odin" => (0x60AFFE, false),
        "Pascal" => (0xE3F171, false),
        "Perl" => (0x0298C3, false),
        "PHP" => (0x4F7495, false),
        "PowerShell" => (0x012456, false),
        "R" => (0x198CEC, false),
        "Ruby" => (0x701516, false),
        "Scheme" => (0x1E4AEC, false),
        "Slint" => (0x3079F4, false),
        "Solidity" => (0xAA6746, false),
        "Swift" => (0xF05138, false),
        "Verilog" => (0xB2B7F8, false),
        "V" => (0x7487C4, false),
        "VHDL" => (0xADB2CB, false),
        "Zig" => (0xEC915C, false),
        "Zsh" => (0x89EC51, false),
        "Assembly" => (0x6E4C13, false),
        "Common Lisp" => (0x3FB68B, false),
        "Dafny" => (0xFFEC25, false),
        "Emacs Lisp" => (0xC065DB, false),
        "Nickel" => (0xE0C3FC, false),
        "SAS" => (0xC44936, false),
        "SQL" => (0xE38C00, false),

        // Machine / data / markup languages
        "CSS" => (0x662F99, false),
        "CMake" => (0xDA2F34, false),
        "HTML" => (0xE34C26, false),
        "JSON" => (0x292929, false),
        "Protobuf" => (0x808080, false),
        "YAML" => (0xCB171E, false),
        "HCL" => (0x844FBA, false),
        "reStructuredText" => (0x141414, false),
        "AsciiDoc" => (0x74A0C5, false),
        _ => return None,
    };

    let _ = fg;
    Some(LogoColors {
        fg: Rgb::hex(fg),
        bg: if two_tone { Some(Rgb::hex(fg).darker()) } else { None },
    })
}

/// Nearest 256-colour palette index for an RGB triple (xterm cube + grey
/// ramp), used to verify that every language maps to a distinct palette slot.
#[cfg(test)]
fn palette_index(c: Rgb) -> u8 {
    let (r, g, b) = (c.r as i16, c.g as i16, c.b as i16);
    if r == g && g == b {
        let steps = [8, 18, 28, 38, 48, 58, 68, 78, 88, 98, 108, 118, 128, 138, 148, 158, 168, 178, 188, 198, 208, 218, 228, 238];
        let mut best = 232u8; let mut bd = u16::MAX;
        for (i, v) in steps.iter().enumerate() {
            let d = (v - r).unsigned_abs();
            if d < bd { bd = d; best = 232 + i as u8; }
        }
        best
    } else {
        let near = |c: i16| -> u8 {
            let steps = [0, 95, 135, 175, 215, 255];
            let mut bi = 0u8; let mut bd = u16::MAX;
            for (i, v) in steps.iter().enumerate() {
                let d = (v - c).unsigned_abs();
                if d < bd { bd = d; bi = i as u8; }
            }
            bi
        };
        16 + 36 * near(r) + 6 * near(g) + near(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_languages_have_unique_hex_and_palette() {
        let reg = crate::language::registry();
        let mut names = HashSet::new();
        let mut hexes = HashSet::new();
        let mut indices = HashSet::new();
        for lang in reg.languages() {
            if !names.insert(lang.name) {
                continue; // same language registered for multiple extensions
            }
            let Some(lc) = logo_colors(lang.name) else { continue };
            assert!(hexes.insert(lc.fg), "duplicate hex for {}: {:?}", lang.name, lc.fg);
            let pi = palette_index(lc.fg);
            assert!(indices.insert(pi), "duplicate 256-index {pi} for {}", lang.name);
        }
        assert!(hexes.len() >= 60, "expected most languages to have a colour");
    }

    #[test]
    fn test_every_registry_language_has_a_colour() {
        let reg = crate::language::registry();
        for lang in reg.languages() {
            assert!(logo_colors(lang.name).is_some(), "no colour for {}", lang.name);
        }
    }

    #[test]
    fn test_two_tone_background_is_darker() {
        let python = logo_colors("Python").unwrap();
        let bg = python.bg.unwrap();
        assert!(bg.r <= python.fg.r && bg.g <= python.fg.g && bg.b <= python.fg.b);
    }
}
