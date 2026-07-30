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
    out.push_str("SLOC by language:\n\n");
    for lang in &report.by_language {
        let pct = if report.total_sloc > 0 {
            (lang.sloc as f64 / report.total_sloc as f64) * 100.0
        } else { 0.0 };
        out.push_str(&format!("{:12} {:>8} ({:.2}%)\n", lang.name, lang.sloc, pct));
    }

    out.push('\n');
    out.push_str(&format!("{:44}= {}\n", "Total lines of code without comments", report.total_sloc));
    out.push_str(&format!("{:44}= {}\n", "Total non-empty lines with comments", report.total_comments));
    out.push_str(&format!("{:44}= {}\n", "Total files", report.total_files));

    if let Some(ref h) = report.halstead {
        out.push_str("\n--- Halstead metrics ---\n\n");
        out.push_str(&format!("{:44}= {}\n", "Distinct operators (n1)", h.distinct_operators));
        out.push_str(&format!("{:44}= {}\n", "Distinct operands (n2)", h.distinct_operands));
        out.push_str(&format!("{:44}= {}\n", "Total operators (N1)", h.total_operators));
        out.push_str(&format!("{:44}= {}\n", "Total operands (N2)", h.total_operands));
        out.push_str(&format!("{:44}= {}\n", "Vocabulary (n = n1 + n2)", h.vocabulary));
        out.push_str(&format!("{:44}= {}\n", "Length (N = N1 + N2)", h.length));
        out.push_str(&format!("{:44}= {:.1}\n", "Estimated length (n1 log n1 + n2 log n2)", h.estimated_length));
        out.push_str(&format!("{:44}= {:.1}\n", "Volume (V = N log2 n)", h.volume));
        out.push_str(&format!("{:44}= {:.2}\n", "Difficulty (D = n1/2 * N2/n2)", h.difficulty));
        out.push_str(&format!("{:44}= {:.0}\n", "Effort (E = D * V)", h.effort));
        out.push_str(&format!("{:44}= {:.1}\n", "Time to implement (T = E / 18 sec)", h.time_seconds));
        let minutes = h.time_seconds / 60.0;
        let hours = minutes / 60.0;
        let days = hours / 8.0;
        out.push_str(&format!("{:44}= {:.0} minutes\n", "", minutes));
        out.push_str(&format!("{:44}= {:.1} hours\n", "", hours));
        out.push_str(&format!("{:44}= {:.1} days\n", "", days));
        out.push_str(&format!("{:44}= {:.2}\n", "Estimated bugs (B = V / 3000)", h.bugs));
    }

    if let Some(ref m) = report.mccabe {
        out.push_str("\n--- McCabe cyclomatic complexity ---\n\n");
        out.push_str(&format!("{:44}= {}\n", "Functions / methods", m.function_count));
        out.push_str(&format!("{:44}= {}\n", "Total cyclomatic complexity", m.total_cyclomatic));
        out.push_str(&format!("{:44}= {:.1}\n", "Average per function", m.average_cyclomatic));
    }

    out
}
