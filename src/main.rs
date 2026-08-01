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

    // --write-ai-config: emit the default config and exit.
    if let Some(path) = args.write_ai_config.as_deref() {
        let target = kloc::ai_config::config_path(Some(path))
            .unwrap_or_else(|| std::path::PathBuf::from("ai.toml"));
        match kloc::ai_config::write_default(&target) {
            Ok(()) => { println!("wrote {}", target.display()); }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
        return;
    }

    // Load the AI config (XDG file if present, else embedded default).
    let ai_config = match kloc::ai_config::load(args.ai_config.as_deref()) {
        Ok(c) => c,
        Err(e) => { eprintln!("{e}"); std::process::exit(1); }
    };

    if args.history {
        let report = match kloc::history::run_history(
            &paths, &filter, args.from.as_deref(), args.to.as_deref(),
            &ai_config, args.ai_multiplier)
        {
            Ok(r) => r,
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        };
        println!("{}", kloc::output::format_history(&report, color, &ai_config, args.ai_multiplier));
        return;
    }

    let output_format = if args.json { OutputFormat::Json } else { OutputFormat::Text };
    let opts = kloc::RunOptions::from_args(&args);
    let report = kloc::run(&paths, &filter, &opts);
    let full = args.full;
    println!("{}", kloc::output::format(&report, &output_format, full, color, &ai_config, args.ai_multiplier));
}
