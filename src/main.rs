use clap::Parser;
use sloccount_rs::{self, cli::Args, output::OutputFormat};

fn main() {
    let args = Args::parse();
    let output_format = if args.json { OutputFormat::Json } else { OutputFormat::Text };
    let report = sloccount_rs::run(&args.paths);
    println!("{}", sloccount_rs::output::format(&report, &output_format));
}
