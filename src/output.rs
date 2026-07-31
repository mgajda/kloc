use crate::report::Report;

pub enum OutputFormat {
    Text,
    Json,
}

pub fn format(report: &Report, format: &OutputFormat, full: bool) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(report).unwrap(),
        OutputFormat::Text => format_text(report, full),
    }
}

/// Humanise a duration in seconds, choosing the largest sensible unit.
fn human_duration(seconds: f64) -> String {
    if seconds < 0.0 || !seconds.is_finite() { return "0 s".to_string(); }
    const MINUTE: f64 = 60.0;
    const HOUR: f64 = 60.0 * MINUTE;
    const DAY: f64 = 24.0 * HOUR;
    const MONTH: f64 = 30.44 * DAY;
    const YEAR: f64 = 12.0 * MONTH;

    let (value, unit) = if seconds >= YEAR {
        (seconds / YEAR, "years")
    } else if seconds >= MONTH {
        (seconds / MONTH, "months")
    } else if seconds >= DAY {
        (seconds / DAY, "days")
    } else if seconds >= HOUR {
        (seconds / HOUR, "hours")
    } else if seconds >= MINUTE {
        (seconds / MINUTE, "minutes")
    } else {
        (seconds, "seconds")
    };
    if unit == "seconds" {
        format!("{value:.0} s")
    } else {
        format!("{value:.1} {unit}")
    }
}

fn format_text(report: &Report, full: bool) -> String {
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

    out.push('\n');
    out.push_str("--- Tokens ---\n\n");
    if let Some(t) = report.llm_tokens {
        out.push_str(&format!("{:44}= {}\n", "LLM tokens (DeepSeek V4)", t.deepseek_v4));
        out.push_str(&format!("{:44}= {}\n", "LLM tokens (Claude Sonnet)", t.claude_sonnet));
    }
    out.push_str(&format!("{:44}= {}\n", "Tree-sitter leaf tokens", report.nodes.leaf_tokens));
    out.push_str(&format!("{:44}= {}\n", "Tree-sitter named nodes", report.nodes.named_nodes));

    // Concise default: one complexity line + schedule.
    if let Some(ref h) = report.halstead {
        out.push('\n');
        out.push_str(&format!(
            "{:44}= {}\n",
            "Time to implement (Halstead T = E / 18 s)",
            human_duration(h.time_seconds)
        ));
    }
    if let Some(ref m) = report.mccabe {
        out.push_str(&format!(
            "{:44}= {:.1}\n",
            "Average cyclomatic complexity per function",
            m.average_cyclomatic
        ));
        out.push_str(&format!(
            "{:44}= {}\n",
            "Functions / methods",
            m.function_count
        ));
    }

    if let Some(ref s) = report.schedule {
        out.push_str(&format!(
            "{:44}= {:.1} person-months\n",
            "Basic COCOMO effort (organic)",
            s.cocomo.effort_person_months
        ));
        out.push_str(&format!(
            "{:44}= {}\n",
            "Basic COCOMO schedule",
            human_duration(s.cocomo.schedule_months * 30.44 * 24.0 * 3600.0)
        ));
        out.push_str(&format!(
            "{:44}= {:.1}\n",
            "Optimal team size (COCOMO PM / TDEV)",
            s.cocomo.avg_people
        ));
        out.push_str(&format!(
            "{:44}= {:.1} person-months\n",
            "COCOMO II effort (nominal)",
            s.cocomo_ii.effort_person_months
        ));
        out.push_str(&format!(
            "{:44}= {}\n",
            "COCOMO II schedule",
            human_duration(s.cocomo_ii.schedule_months * 30.44 * 24.0 * 3600.0)
        ));
        out.push_str(&format!(
            "{:44}= {:.1}\n",
            "Optimal team size (COCOMO II)",
            s.cocomo_ii.avg_people
        ));
        out.push_str(&format!(
            "{:44}= {}\n",
            "Putnam schedule",
            human_duration(s.putnam.schedule_years * 365.25 * 24.0 * 3600.0)
        ));
        out.push_str(&format!(
            "{:44}= {:.1}\n",
            "Optimal team size (Putnam)",
            s.putnam.avg_people
        ));
        out.push_str(&format!(
            "{:44}= {:.1} person-months\n",
            "Halstead effort",
            s.halstead_person_months
        ));
    }

    if full {
        if let Some(ref h) = report.halstead {
            out.push_str("\n--- Halstead metrics ---\n\n");
            out.push_str(&format!("{:44}= {}\n", "Distinct operators (n1)", h.distinct_operators));
            out.push_str(&format!("{:44}= {}\n", "Distinct operands (n2)", h.distinct_operands));
            out.push_str(&format!("{:44}= {}\n", "Total operators (N1)", h.total_operators));
            out.push_str(&format!("{:44}= {}\n", "Total operands (N2)", h.total_operands));
            out.push_str(&format!("{:44}= {}\n", "Vocabulary (n = n1 + n2)", h.vocabulary));
            out.push_str(&format!("{:44}= {}\n", "Length (N = N1 + N2)", h.length));
            out.push_str(&format!("{:44}= {:.1}\n", "Estimated length", h.estimated_length));
            out.push_str(&format!("{:44}= {:.1}\n", "Volume (V = N log2 n)", h.volume));
            out.push_str(&format!("{:44}= {:.2}\n", "Difficulty (D = n1/2 * N2/n2)", h.difficulty));
            out.push_str(&format!("{:44}= {:.0}\n", "Effort (E = D * V)", h.effort));
            out.push_str(&format!("{:44}= {}\n", "Time to implement (T = E / 18 s)", human_duration(h.time_seconds)));
            out.push_str(&format!("{:44}= {:.2}\n", "Estimated bugs (B = V / 3000)", h.bugs));
        }
        if let Some(ref m) = report.mccabe {
            out.push_str("\n--- McCabe cyclomatic complexity ---\n\n");
            out.push_str(&format!("{:44}= {}\n", "Functions / methods", m.function_count));
            out.push_str(&format!("{:44}= {}\n", "Total cyclomatic complexity", m.total_cyclomatic));
            out.push_str(&format!("{:44}= {:.1}\n", "Average per function", m.average_cyclomatic));
        }
    }

    out.push('\n');
    out.push_str("--- Performance ---\n\n");
    let p = &report.performance;
    let gbps = if p.elapsed_secs > 0.0 {
        (p.bytes_parsed as f64 / 1e9) / p.elapsed_secs
    } else { 0.0 };
    out.push_str(&format!("{:44}= {:.3} s\n", "Total runtime", p.elapsed_secs));
    out.push_str(&format!("{:44}= {:.3} GB/s\n", "Sources parsed", gbps));
    out.push_str(&format!("{:44}= {:.1} files/s\n", "Files", p.files_per_sec));
    out.push_str(&format!("{:44}= {:.1} declarations/s\n", "Declarations (functions)", p.functions_per_sec));
    if report.cache_hits > 0 || report.cache_misses > 0 {
        out.push_str(&format!("{:44}= {} hits / {} misses\n", "Cache", report.cache_hits, report.cache_misses));
    }

    out
}
