use std::path::PathBuf;
use clap::Parser;

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

    #[arg(long, help = "Only count SLOC/comments/blanks (skip complexity analysis)")]
    pub sloc_only: bool,

    #[arg(long, help = "Show detailed complexity and schedule metrics (default is concise)")]
    pub full: bool,

    #[arg(long, help = "Disable on-disk result caching")]
    pub no_cache: bool,
}
