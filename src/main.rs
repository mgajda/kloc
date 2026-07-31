use clap::Parser;
use kloc::{self, cli::Args, output::OutputFormat, LanguageFilter};

fn main() {
    let args = Args::parse();
    let paths: Vec<std::path::PathBuf> = if args.paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let color = kloc::color::Colors::from_mode(args.color);
    let filter = LanguageFilter::from(&args);

    if args.history {
        let plan = args.ai_plan.unwrap_or(kloc::history::AiPlan::Max20);
        let report = match kloc::history::run_history(
            &paths, &filter, args.from.as_deref(), args.to.as_deref(), &[plan], args.ai_budget)
        {
            Ok(r) => r,
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        };
        println!("{}", kloc::output::format_history(&report, color));
        return;
    }

    let output_format = if args.json { OutputFormat::Json } else { OutputFormat::Text };
    let opts = kloc::RunOptions::from_args(&args);
    let report = kloc::run(&paths, &filter, &opts);
    let full = args.full;
    println!("{}", kloc::output::format(&report, &output_format, full, color));
}
