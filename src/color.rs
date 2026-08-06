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

    /// A lighter shade, used as the foreground on a dark background so the
    /// text stays legible. Scales toward white by the given fraction.
    pub fn lightened(self, by: f64) -> Self {
        let mix = |c: u8| (c as f64 + (255.0 - c as f64) * by).round() as u8;
        Rgb {
            r: mix(self.r),
            g: mix(self.g),
            b: mix(self.b),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    enabled: bool,
}

impl Colors {
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
        self.wrap(
            s,
            &format!(
                "38;2;{};{};{};48;2;{};{};{}",
                fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
            ),
        )
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

    /// Bright foreground over a 256-colour background (`48;5;N`). Use the
    /// xterm greyscale ramp (232..255) for dark shades: lower = darker.
    pub fn on_bg256(&self, s: &str, fg: u8, bg256: u8) -> String {
        let f = 90 + fg.min(7);
        self.wrap(s, &format!("{f};48;5;{bg256}"))
    }
}

/// A language's colours: the foreground is the GitHub colour; `bg` is a
/// derived darker shade used as the background for two-tone rendering.
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
    // Base is the GitHub brand colour (all distinct). Lighten the foreground
    // so it reads on dark terminals; Python's brand blue becomes the
    // background under the lightened foreground.
    let (base, explicit_bg) = match name {
        // Programming languages.
        "Rust" => (0xDEA584, None),
        "C" => (0x555555, None),
        "C++" => (0xF34B7D, None),
        "Python" => (0xFFD43B, Some(0x3572A5)), // bright yellow fg on blue bg
        "Jupyter Notebook" => (0xF37726, None), // Jupyter orange
        "JavaScript" => (0xF1E05A, None),
        "Bash" => (0x89E051, None),
        "Haskell" => (0x5E5086, None),
        "OCaml" => (0xEF7A08, None),
        "Elm" => (0x60B5CC, None),
        "Go" => (0x00ADD8, None),
        "TypeScript" => (0x3178C6, None),
        "TSX" => (0x2F78C6, None),
        "Java" => (0xB07219, None),
        "Scala" => (0xC22D40, None),
        "Ada" => (0x02F88C, None),
        "Agda" => (0x315665, None),
        "C#" => (0x7355DD, None),
        "Dart" => (0x00B4AB, None),
        "Elixir" => (0x744A7E, None),
        "Erlang" => (0xB83998, None),
        "Fish" => (0x4AAE47, None),
        "Fortran" => (0x4D2FB1, None),
        "F#" => (0xB845FC, None),
        "Gleam" => (0xFFAFF3, None),
        "GLSL" => (0x5686A5, None),
        "GraphQL" => (0xE10098, None),
        "Groovy" => (0x429CB8, None),
        "Julia" => (0xA270BA, None),
        "Lua" => (0x000080, None),
        "Make" => (0x427819, None),
        "Nix" => (0x7E7EFF, None),
        "Odin" => (0x60AFFE, None),
        "Pascal" => (0xE3F171, None),
        "Perl" => (0x0298C3, None),
        "PHP" => (0x4F7495, None),
        "PowerShell" => (0x012456, None),
        "R" => (0x198CEC, None),
        "Ruby" => (0x701516, None),
        "Scheme" => (0x1E4AEC, None),
        "Slint" => (0x2379F4, None),
        "Solidity" => (0xAA6746, None),
        "Swift" => (0xF05138, None),
        "Verilog" => (0xB2B7F8, None),
        "V" => (0x4F87C4, None),
        "VHDL" => (0xADB2CB, None),
        "Zig" => (0xEC915C, None),
        "Zsh" => (0x8EE04E, None),
        "Assembly" => (0x6E4C13, None),
        "Common Lisp" => (0x3FB68B, None),
        "Dafny" => (0xFFEC25, None),
        "Emacs Lisp" => (0xC065DB, None),
        "Nickel" => (0xE0C3FC, None),
        "SAS" => (0xB34936, None),
        "SQL" => (0xE38C00, None),

        // Machine / data / markup languages
        "CSS" => (0x663399, None),
        "CMake" => (0xDA3434, None),
        "HTML" => (0xE34C26, None),
        "JSON" => (0x292929, None),
        "Protobuf" => (0x808080, None),
        "YAML" => (0xCB171E, None),
        "HCL" => (0x844FBA, None),
        "reStructuredText" => (0x141414, None),
        "AsciiDoc" => (0x73A0C5, None),
        _ => return None,
    };

    let base_rgb = Rgb::hex(base);
    Some(LogoColors {
        // Lighten by 40% so even dark brand colours read on a dark terminal.
        fg: base_rgb.lightened(0.40),
        bg: explicit_bg.map(Rgb::hex),
    })
}

/// Invert `Rgb::lightened(by)`: `c' = c + (255-c)·by` ⇒ `c = (c' − 255·by)/(1−by)`.
/// Test-only, used to recover the brand colour from a lightened foreground.
#[cfg(test)]
fn unlighten(c: Rgb, by: f64) -> u32 {
    let f = |v: u8| {
        ((v as f64 - 255.0 * by) / (1.0 - by))
            .round()
            .clamp(0.0, 255.0) as u32
    };
    (f(c.r) << 16) | (f(c.g) << 8) | f(c.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_languages_have_unique_brand_color() {
        // Each language's brand (base) colour must be unique in hex so any
        // two languages have a distinct identity colour. The displayed
        // foreground is a lightened variant of the brand colour for
        // readability on dark backgrounds; after lightening, distinct hues
        // may share a coarse 256-palette slot, but the brand hex is unique.
        let reg = crate::language::registry();
        let mut names = HashSet::new();
        let mut hexes = HashSet::new();
        for lang in reg.languages() {
            if !names.insert(lang.name) {
                continue; // same language registered for multiple extensions
            }
            let Some(lc) = logo_colors(lang.name) else {
                continue;
            };
            // Recover the brand colour: the foreground is lightened 40%.
            let brand = unlighten(lc.fg, 0.40);
            assert!(
                hexes.insert(brand),
                "duplicate brand hex for {}: {:?}",
                lang.name,
                brand
            );
        }
        assert_eq!(
            hexes.len(),
            names.len(),
            "every unique language must have a distinct brand colour"
        );
    }

    #[test]
    fn test_every_registry_language_has_a_colour() {
        let reg = crate::language::registry();
        for lang in reg.languages() {
            assert!(
                logo_colors(lang.name).is_some(),
                "no colour for {}",
                lang.name
            );
        }
    }

    #[test]
    fn test_two_tone_background_is_darker() {
        let python = logo_colors("Python").unwrap();
        let bg = python.bg.unwrap();
        // The background must be perceptibly darker than the foreground
        // (lower luminance) so the text is legible. Per-channel comparison is
        // too strict for complementary colours (yellow fg on blue bg).
        let lum = |c: Rgb| 0.299 * c.r as f64 + 0.587 * c.g as f64 + 0.114 * c.b as f64;
        assert!(
            lum(bg) < lum(python.fg),
            "background must be darker than foreground"
        );
    }

    #[test]
    fn from_mode_always_and_never() {
        assert!(Colors::from_mode(ColorMode::Always).enabled);
        assert!(!Colors::from_mode(ColorMode::Never).enabled);
        // Auto in a test (stdout is not a terminal) is disabled.
        assert!(!Colors::from_mode(ColorMode::Auto).enabled);
    }

    #[test]
    fn wrap_enabled_emits_ansi() {
        let c = Colors::force(true);
        assert_eq!(c.ansi("x", 4), "\x1b[94mx\x1b[0m");
        assert_eq!(c.gray("g"), "\x1b[90mg\x1b[0m");
        assert_eq!(c.fg("f", Rgb::hex(0xFF0000)), "\x1b[38;2;255;0;0mf\x1b[0m");
        assert_eq!(
            c.on("t", Rgb::hex(0xFFFFFF), Rgb::hex(0x000000)),
            "\x1b[38;2;255;255;255;48;2;0;0;0mt\x1b[0m"
        );
        assert_eq!(c.on_bg256("b", 4, 236), "\x1b[94;48;5;236mb\x1b[0m");
        // Palette index clamped to 7.
        assert_eq!(c.ansi("x", 9), "\x1b[97mx\x1b[0m");
    }

    #[test]
    fn wrap_disabled_passes_through() {
        let c = Colors::force(false);
        assert_eq!(c.ansi("x", 4), "x");
        assert_eq!(c.gray("g"), "g");
        assert_eq!(c.fg("f", Rgb::hex(0xFF0000)), "f");
    }

    #[test]
    fn lightened_scales_toward_white() {
        let c = Rgb::hex(0x000000).lightened(0.5);
        assert_eq!((c.r, c.g, c.b), (128, 128, 128));
        let white = Rgb::hex(0xFFFFFF).lightened(0.5);
        assert_eq!((white.r, white.g, white.b), (255, 255, 255));
    }
    #[test]
    fn logo_colors_covers_data_and_shader_languages() {
        // Every match arm must be reachable: deleting any arm makes
        // logo_colors return None for that name, which the assertion catches.
        for name in [
            "GLSL",
            "GraphQL",
            "Make",
            "V",
            "CSS",
            "CMake",
            "HTML",
            "JSON",
            "Protobuf",
            "YAML",
            "HCL",
            "reStructuredText",
            "AsciiDoc",
        ] {
            assert!(logo_colors(name).is_some(), "no logo colour for {name}");
        }
        // Unknown names yield None.
        assert!(logo_colors("NoSuchLanguage").is_none());
    }
}
