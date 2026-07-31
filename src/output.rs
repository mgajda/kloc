use crate::color::{Colors, logo_colors};
use crate::report::Report;
use crate::history::HistoryReport;

pub enum OutputFormat {
    Text,
    Json,
}

pub fn format(report: &Report, format: &OutputFormat, full: bool, colors: Colors) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(report).unwrap(),
        OutputFormat::Text => format_text(report, full, colors),
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

/// Humanise an effort in person-months, choosing the largest sensible unit:
/// person-years beyond 12 months, person-weeks below 2 months, person-days
/// below a week.
fn human_person_months(pm: f64) -> String {
    if pm < 0.0 || !pm.is_finite() { return "0 person-months".to_string(); }
    const WEEK: f64 = 12.0 / 52.0;
    const DAY: f64 = WEEK / 7.0;

    let (value, unit) = if pm >= 12.0 {
        (pm / 12.0, "person-years")
    } else if pm >= 2.0 {
        (pm, "person-months")
    } else if pm >= WEEK {
        (pm / WEEK, "person-weeks")
    } else if pm >= DAY {
        (pm / DAY, "person-days")
    } else {
        (pm, "person-months")
    };
    format!("{value:.1} {unit}")
}

fn format_text(report: &Report, full: bool, colors: Colors) -> String {
    let mut out = String::new();
    out.push_str("SLOC by language:\n\n");
    for lang in &report.by_language {
        let pct = if report.total_sloc > 0 {
            (lang.sloc as f64 / report.total_sloc as f64) * 100.0
        } else { 0.0 };
        let name_field = format!("{:12}", lang.name);
        let name_field = match logo_colors(&lang.name) {
            Some(lc) => match lc.bg {
                Some(bg) => colors.on(&name_field, lc.fg, bg),
                None => colors.fg(&name_field, lc.fg),
            },
            None => name_field,
        };
        out.push_str(&format!("{} {:>8} ({:.2}%)\n", name_field, lang.sloc, pct));
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
        out.push_str("\n--- Schedule ---\n\n");
        // Rows: Schedule / Effort / Team size. Columns: methodologies, each
        // with a distinct standard colour to ease reading. Cells a model
        // does not produce are shown as "—".
        let months = |m: f64| human_duration(m * 30.44 * 24.0 * 3600.0);

        struct Col<'a> {
            label: &'a str,
            color: u8,
            schedule: String,
            effort: String,
            team: String,
        }
        let cols = [
            Col { label: "Basic COCOMO", color: 4, schedule: months(s.cocomo.schedule_months), effort: human_person_months(s.cocomo.effort_person_months), team: format!("{:.1}", s.cocomo.avg_people) },
            Col { label: "COCOMO II", color: 2, schedule: months(s.cocomo_ii.schedule_months), effort: human_person_months(s.cocomo_ii.effort_person_months), team: format!("{:.1}", s.cocomo_ii.avg_people) },
            Col { label: "Putnam", color: 3, schedule: months(s.putnam.schedule_months), effort: "—".to_string(), team: format!("{:.1}", s.putnam.avg_people) },
            Col { label: "Halstead", color: 5, schedule: "—".to_string(), effort: human_person_months(s.halstead_person_months), team: "—".to_string() },
        ];

        let mut widths = [10, 0, 0, 0, 0];
        for (i, c) in cols.iter().enumerate() {
            widths[i + 1] = c.label.len().max(c.schedule.len()).max(c.effort.len()).max(c.team.len());
        }
        let total = widths.iter().sum::<usize>() + 2 * cols.len();
        let _ = total;

        let line = |label: &str, values: [String; 4], color_fns: [Option<u8>; 4]| {
            let mut l = format!("{:<width$}", label, width = widths[0]);
            for (i, v) in values.iter().enumerate() {
                let cell = format!("{:<width$}", v, width = widths[i + 1]);
                let cell = match color_fns[i] {
                    Some(c) => colors.ansi(&cell, c),
                    None => cell,
                };
                l.push_str(&format!("  {cell}"));
            }
            l
        };

        // Header: methodology names coloured per column.
        let mut hdr = format!("{:<width$}", "Methodology", width = widths[0]);
        for (i, c) in cols.iter().enumerate() {
            let cell = format!("{:<width$}", c.label, width = widths[i + 1]);
            hdr.push_str(&format!("  {}", colors.ansi(&cell, c.color)));
        }
        out.push_str(&format!("{hdr}\n"));

        let schedule_vals = [
            cols[0].schedule.clone(), cols[1].schedule.clone(), cols[2].schedule.clone(), cols[3].schedule.clone(),
        ];
        out.push_str(&format!("{}\n", line("Schedule", schedule_vals, [Some(4), Some(2), Some(3), Some(5)])));

        let effort_vals = [
            cols[0].effort.clone(), cols[1].effort.clone(), cols[2].effort.clone(), cols[3].effort.clone(),
        ];
        out.push_str(&format!("{}\n", line("Effort", effort_vals, [Some(4), Some(2), Some(3), Some(5)])));

        let team_vals = [
            cols[0].team.clone(), cols[1].team.clone(), cols[2].team.clone(), cols[3].team.clone(),
        ];
        out.push_str(&format!("{}\n", line("Team size", team_vals, [Some(4), Some(2), Some(3), Some(5)])));
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
    let mut perf = String::new();
    perf.push_str("--- Performance ---\n\n");
    let p = &report.performance;
    let gbps = if p.elapsed_secs > 0.0 {
        (p.bytes_parsed as f64 / 1e9) / p.elapsed_secs
    } else { 0.0 };
    perf.push_str(&format!("{:44}= {:.3} s\n", "Total runtime", p.elapsed_secs));
    perf.push_str(&format!("{:44}= {:.3} GB/s\n", "Sources parsed", gbps));
    perf.push_str(&format!("{:44}= {:.1} files/s\n", "Files", p.files_per_sec));
    perf.push_str(&format!("{:44}= {:.1} declarations/s\n", "Declarations (functions)", p.functions_per_sec));
    if report.cache_hits > 0 || report.cache_misses > 0 {
        perf.push_str(&format!("{:44}= {} hits / {} misses\n", "Cache", report.cache_hits, report.cache_misses));
    }
    out.push_str(&colors.gray(&perf));

    out
}

/// Format the git-history report.
pub fn format_history(report: &HistoryReport, colors: Colors) -> String {
    let mut out = String::new();
    out.push_str("Git history:\n\n");
    out.push_str(&format!("{:44}= {}\n", "Commits analyzed", report.commits));
    out.push_str(&format!("{:44}= {}\n", "Lines added", report.total_added_lines));
    out.push_str(&format!("{:44}= {}\n", "Lines removed", report.total_removed_lines));
    out.push_str(&format!("{:44}= {}\n", "Changed tokens (Claude)", report.total_changed_tokens));

    if let Some(llm) = &report.llm_changed_tokens {
        out.push_str(&format!("{:44}= {}\n", "Changed tokens (DeepSeek V4)", llm.deepseek_v4));
    }

    if !report.by_language.is_empty() {
        out.push_str("\nChanged lines by language:\n\n");
        for lang in &report.by_language {
            let name_field = format!("{:12}", lang.name);
            let name_field = match logo_colors(&lang.name) {
                Some(lc) => match lc.bg {
                    Some(bg) => colors.on(&name_field, lc.fg, bg),
                    None => colors.fg(&name_field, lc.fg),
                },
                None => name_field,
            };
            out.push_str(&format!(
                "{} +{:>8}  -{:>8}  {:>12} tokens\n",
                name_field, lang.added_lines, lang.removed_lines, lang.changed_tokens
            ));
        }
    }

    if !report.ai_estimates.is_empty() {
        out.push_str("\n--- AI time to process ---\n\n");
        for e in &report.ai_estimates {
            out.push_str(&format!(
                "{:44}= {} ({} / 5-hour window)\n",
                e.plan, human_duration(e.elapsed_seconds), e.tokens_per_5h
            ));
            out.push_str(&format!(
                "  {:44}= {} changed tokens; {} 5-hour windows\n",
                "",
                e.changed_tokens, e.windows_5h
            ));
        }
        out.push_str("\nCalibration is approximate (Anthropic publishes plan multiples,\n");
        out.push_str("not absolute token numbers); override with --ai-budget.\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_person_months_units() {
        assert_eq!(human_person_months(36.0), "3.0 person-years");
        assert_eq!(human_person_months(12.0), "1.0 person-years");
        assert_eq!(human_person_months(11.9), "11.9 person-months");
        assert_eq!(human_person_months(2.0), "2.0 person-months");
        assert_eq!(human_person_months(1.5), "6.5 person-weeks");
        assert_eq!(human_person_months(0.5), "2.2 person-weeks");
        assert_eq!(human_person_months(0.1), "3.0 person-days");
        assert_eq!(human_person_months(0.0), "0.0 person-months");
    }
}
