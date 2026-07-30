use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sloccount-rs", version, about = "Count Source Lines of Code")]
pub struct Args {
    pub paths: Vec<PathBuf>,

    #[arg(long, help = "Output as JSON")]
    pub json: bool,
}
