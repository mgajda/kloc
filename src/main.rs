use clap::Parser;
use sloccount_rs::{self, cli::Args, output::OutputFormat, LanguageFilter};

fn main() {
    let args = Args::parse();
    let output_format = if args.json { OutputFormat::Json } else { OutputFormat::Text };
    let filter = LanguageFilter::from(&args);
    let report = sloccount_rs::run(&args.paths, &filter);
    println!("{}", sloccount_rs::output::format(&report, &output_format));
}
