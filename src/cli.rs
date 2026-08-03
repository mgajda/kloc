use crate::color::ColorMode;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kloc", version, about = "Count Source Lines of Code")]
pub struct Args {
    pub paths: Vec<PathBuf>,

    #[arg(long, help = "Output as JSON")]
    pub json: bool,

    #[arg(long, value_delimiter = ',', help = "Only count these languages")]
    pub only: Vec<String>,

    #[arg(long, value_delimiter = ',', help = "Exclude these languages")]
    pub exclude: Vec<String>,

    #[arg(long, help = "Only count programming languages")]
    pub only_programming: bool,

    #[arg(long, help = "Only count machine/data languages")]
    pub only_machine: bool,

    #[arg(
        long,
        help = "Only count SLOC/comments/blanks (skip complexity analysis)"
    )]
    pub sloc_only: bool,

    #[arg(
        long,
        help = "Show detailed Halstead/McCabe/Henry-Kafura metrics (default is concise)"
    )]
    pub full: bool,

    #[arg(long, help = "Disable on-disk result caching")]
    pub no_cache: bool,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Also ignore these directory names (in addition to defaults)"
    )]
    pub ignore: Vec<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Stop ignoring these directory names (remove from defaults)"
    )]
    pub no_ignore: Vec<String>,

    #[arg(long, help = "Disable the default-ignored directory list entirely")]
    pub no_ignore_defaults: bool,

    #[arg(long, value_enum, default_value_t = ColorMode::Auto, help = "When to use colors in output (auto, always, never)")]
    pub color: ColorMode,

    #[arg(
        long,
        help = "Analyze git history: changed tokens per commit, AI-plan time to process them, and effort estimate"
    )]
    pub history: bool,

    #[arg(
        long,
        help = "With --history: start commit or revision (default: from the initial commit(s))"
    )]
    pub from: Option<String>,

    #[arg(
        long,
        help = "With --history: end commit or revision (default: the current branch tip)"
    )]
    pub to: Option<String>,

    #[arg(
        long,
        help = "Path to the AI-platform config file (default: $XDG_CONFIG_HOME/kloc/ai.toml)"
    )]
    pub ai_config: Option<String>,

    #[arg(
        long,
        help = "Write the embedded default AI config to a file and exit (default: the XDG path)"
    )]
    pub write_ai_config: Option<String>,

    #[arg(
        long,
        help = "Override the AI effort token multiplier for all platforms (e.g. 3-5 standard, 10-20 complex)"
    )]
    pub ai_multiplier: Option<f64>,

    #[arg(short = 'v', long, action = clap::ArgAction::Count,
        help = "Verbose logging: -v shows info, -vv shows debug (default: errors and warnings only)")]
    pub verbose: u8,
}
