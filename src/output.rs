use crate::report::Report;

pub enum OutputFormat {
    Text,
    Json,
}

pub fn format(report: &Report, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(report).unwrap(),
        OutputFormat::Text => format_text(report),
    }
}

fn format_text(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("SLOC by language:\n");
    out.push('\n');
    for lang in &report.by_language {
        let pct = if report.total_sloc > 0 {
            (lang.sloc as f64 / report.total_sloc as f64) * 100.0
        } else {
            0.0
        };
        out.push_str(&format!("{:12} {:>8} ({:.2}%)\n", lang.name, lang.sloc, pct));
    }
    out.push('\n');
    out.push_str(&format!(
        "Total Physical Source Lines of Code (SLOC) = {}\n",
        report.total_sloc
    ));
    out.push_str(&format!("Total Files                     = {}\n", report.total_files));
    out
}
