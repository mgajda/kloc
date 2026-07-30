use clap::Parser;
use kloc::{self, cli::Args, output::OutputFormat, LanguageFilter};

fn main() {
    let args = Args::parse();
    let paths: Vec<std::path::PathBuf> = if args.paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let output_format = if args.json { OutputFormat::Json } else { OutputFormat::Text };
    let filter = LanguageFilter::from(&args);
    let report = kloc::run(&paths, &filter);
    println!("{}", kloc::output::format(&report, &output_format));
}
