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

/// Humanise a token count with a k/M/B/T suffix, keeping at most 3 digits.
fn human_tokens(n: u64) -> String {
    const K: u64 = 1000;
    const M: u64 = K * 1000;
    const B: u64 = M * 1000;
    const T: u64 = B * 1000;
    let (value, suffix) = if n >= T {
        (n as f64 / T as f64, "T")
    } else if n >= B {
        (n as f64 / B as f64, "B")
    } else if n >= M {
        (n as f64 / M as f64, "M")
    } else if n >= K {
        (n as f64 / K as f64, "k")
    } else {
        (n as f64, "")
    };
    if suffix.is_empty() {
        n.to_string()
    } else if value >= 100.0 {
        format!("{value:.0}{suffix}")
    } else if value >= 10.0 {
        format!("{value:.1}{suffix}")
    } else {
        format!("{value:.2}{suffix}")
    }
}

/// Align a column of cells so the decimal points line up.
///
/// Each cell is split into its leading number (up to the first space) and the
/// unit that follows (e.g. `"5.5 person-months"` → number `"5.5"`, unit
/// `"person-months"`). The numbers are padded so the `.` sits at the same
/// column across the whole column; non-numeric cells (like `—`) are
/// right-aligned to the same width.
fn align_dots(cells: &[String]) -> Vec<String> {
    let parts: Vec<(&str, &str)> = cells.iter()
        .map(|c| match c.split_once(' ') {
            Some((n, u)) => (n, u),
            None => (c.as_str(), ""),
        })
        .collect();
    let mut before = 0usize;
    let mut after = 0usize;
    for (n, _) in &parts {
        if let Some(i) = n.find('.') {
            before = before.max(i);
            after = after.max(n.len() - i - 1);
        }
    }
    let width = before + 1 + after;
    parts.iter().map(|(n, u)| {
        let num = if let Some(i) = n.find('.') {
            format!("{:>before$}.{:<after$}", &n[..i], &n[i + 1..])
        } else {
            format!("{n:>width$}")
        };
        if u.is_empty() { num } else { format!("{num} {u}") }
    }).collect()
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
        out.push_str(&format!("{:44}= {}\n", "LLM tokens (DeepSeek V4)", human_tokens(t.deepseek_v4)));
        out.push_str(&format!("{:44}= {}\n", "LLM tokens (Claude Sonnet)", human_tokens(t.claude_sonnet)));
    }
    out.push_str(&format!("{:44}= {}\n", "Tree-sitter leaf tokens", human_tokens(report.nodes.leaf_tokens)));
    out.push_str(&format!("{:44}= {}\n", "Tree-sitter named nodes", human_tokens(report.nodes.named_nodes)));

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

        // Grouped, parameterised table. Rows are Metric / Effort / Team size /
        // Schedule; columns are estimation methodologies grouped into
        // families. Each column is a `MetricCol` with a label, a group, a
        // colour, and the four row values — add a new methodology by pushing
        // another `MetricCol`. Cells a model does not produce are "—".
        let months = |m: f64| human_duration(m * 30.44 * 24.0 * 3600.0);
        let halstead_schedule = report.halstead.as_ref().map(|h| human_duration(h.time_seconds))
            .unwrap_or_else(|| "—".to_string());

        struct MetricCol {
            group: &'static str,
            label: &'static str,
            color: u8,
            metric: String,
            effort: String,
            team: String,
            schedule: String,
        }

        // Token models: one column per LLM (Claude Max, OpenCode Go /
        // DeepSeek V4). Metric is the token count; effort & schedule are the
        // AI-plan time to process those tokens; no team size.
        let token_cols: Vec<MetricCol> = match report.llm_tokens {
            Some(t) => vec![
                MetricCol {
                    group: "Token", label: "Claude Max", color: 6,
                    metric: if t.claude_sonnet > 0 { format!("{} tokens", human_tokens(t.claude_sonnet)) } else { "—".to_string() },
                    effort: if t.claude_sonnet > 0 { human_duration(crate::history::ai_time_seconds(t.claude_sonnet)) } else { "—".to_string() },
                    team: "—".to_string(),
                    schedule: if t.claude_sonnet > 0 { human_duration(crate::history::ai_time_seconds(t.claude_sonnet)) } else { "—".to_string() },
                },
                MetricCol {
                    group: "Token", label: "OpenCode Go", color: 6,
                    metric: if t.deepseek_v4 > 0 { format!("{} tokens", human_tokens(t.deepseek_v4)) } else { "—".to_string() },
                    effort: if t.deepseek_v4 > 0 { human_duration(crate::history::ai_time_seconds(t.deepseek_v4)) } else { "—".to_string() },
                    team: "—".to_string(),
                    schedule: if t.deepseek_v4 > 0 { human_duration(crate::history::ai_time_seconds(t.deepseek_v4)) } else { "—".to_string() },
                },
            ],
            None => vec![],
        };

        // Classical models: metric is code size (lines of code, with a k
        // human suffix); effort in person-months; team size where the model
        // has one.
        let mut cols = vec![
            MetricCol {
                group: "COCOMO", label: "COCOMO 1", color: 4,
                metric: format!("{:.1} k lines of code", s.ksloc),
                effort: human_person_months(s.cocomo.effort_person_months),
                team: format!("{:.1}", s.cocomo.avg_people),
                schedule: months(s.cocomo.schedule_months),
            },
            MetricCol {
                group: "COCOMO", label: "COCOMO 2", color: 2,
                metric: format!("{:.1} k lines of code", s.ksloc),
                effort: human_person_months(s.cocomo_ii.effort_person_months),
                team: format!("{:.1}", s.cocomo_ii.avg_people),
                schedule: months(s.cocomo_ii.schedule_months),
            },
            MetricCol {
                group: "Putnam", label: "Putnam", color: 3,
                metric: format!("{:.1} k lines of code", s.ksloc),
                effort: human_person_months(s.cocomo_ii.effort_person_months),
                team: format!("{:.1}", s.putnam.avg_people),
                schedule: months(s.putnam.schedule_months),
            },
            MetricCol {
                group: "Halstead", label: "Halstead", color: 5,
                metric: match report.halstead.as_ref() {
                    Some(h) => format!("{:.0} volume", h.volume),
                    None => "—".to_string(),
                },
                effort: human_person_months(s.halstead_person_months),
                team: "—".to_string(),
                schedule: halstead_schedule,
            },
        ];
        cols.extend(token_cols);

        if cols.is_empty() {
            out.push_str("(no schedule data)\n");
        } else {
            let n = cols.len();

            // Align each column's 4 row values on the decimal point.
            let mut aligned: Vec<[String; 4]> = Vec::with_capacity(n);
            for c in &cols {
                let cells = [c.metric.clone(), c.effort.clone(), c.team.clone(), c.schedule.clone()];
                let a = align_dots(&cells);
                aligned.push([a[0].clone(), a[1].clone(), a[2].clone(), a[3].clone()]);
            }

            // Column widths; index 0 is the row-label column.
            let mut widths: Vec<usize> = vec![8];
            for (i, c) in cols.iter().enumerate() {
                let w = c.label.len()
                    .max(aligned[i][0].len()).max(aligned[i][1].len())
                    .max(aligned[i][2].len()).max(aligned[i][3].len());
                widths.push(w);
            }

            let render = |cells: &[String], color_fns: &[Option<u8>]| -> String {
                let mut l = format!("{:<8}", cells[0]);
                for (i, v) in cells[1..].iter().enumerate() {
                    let cell = format!("{:>width$}", v, width = widths[i + 1]);
                    let cell = match color_fns[i] {
                        Some(c) => colors.ansi(&cell, c),
                        None => cell,
                    };
                    l.push_str(&format!("  {cell}"));
                }
                l
            };

            // Group header row: span the columns of each group.
            let mut group_header = String::new();
            let mut gi = 0;
            while gi < n {
                let g = cols[gi].group;
                let mut span = 1;
                while gi + span < n && cols[gi + span].group == g { span += 1; }
                let w: usize = (0..span).map(|k| widths[gi + k + 1]).sum::<usize>() + 2 * (span - 1);
                group_header.push_str(&format!("  {g:^width$}", width = w));
                gi += span;
            }
            out.push_str(&format!("{:<8}{group_header}\n", ""));

            // Column header row.
            let mut hdr = format!("{:<8}", "Method");
            for (i, c) in cols.iter().enumerate() {
                let cell = format!("{:>width$}", c.label, width = widths[i + 1]);
                hdr.push_str(&format!("  {}", colors.ansi(&cell, c.color)));
            }
            out.push_str(&format!("{hdr}\n"));

            let row_colors: Vec<Option<u8>> = cols.iter().map(|c| Some(c.color)).collect();

            let mut metric_row = vec!["Metric".to_string()];
            metric_row.extend(aligned.iter().map(|a| a[0].clone()));
            out.push_str(&format!("{}\n", render(&metric_row, &row_colors)));

            let mut effort_row = vec!["Effort".to_string()];
            effort_row.extend(aligned.iter().map(|a| a[1].clone()));
            out.push_str(&format!("{}\n", render(&effort_row, &row_colors)));

            let mut team_row = vec!["Team size".to_string()];
            team_row.extend(aligned.iter().map(|a| a[2].clone()));
            out.push_str(&format!("{}\n", render(&team_row, &row_colors)));

            let mut schedule_row = vec!["Schedule".to_string()];
            schedule_row.extend(aligned.iter().map(|a| a[3].clone()));
            out.push_str(&format!("{}\n", render(&schedule_row, &row_colors)));
        }
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
    out.push_str(&format!("{:44}= {}\n", "Changed tokens (Claude)", human_tokens(report.total_changed_tokens)));

    if let Some(llm) = &report.llm_changed_tokens {
        out.push_str(&format!("{:44}= {}\n", "Changed tokens (DeepSeek V4)", human_tokens(llm.deepseek_v4)));
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
                name_field, lang.added_lines, lang.removed_lines, human_tokens(lang.changed_tokens)
            ));
        }
    }

    if !report.ai_estimates.is_empty() {
        out.push_str("\n--- AI time to process ---\n\n");
        for e in &report.ai_estimates {
            out.push_str(&format!(
                "{:44}= {} ({} / 5-hour window)\n",
                e.plan, human_duration(e.elapsed_seconds), human_tokens(e.tokens_per_5h)
            ));
            out.push_str(&format!(
                "  {:44}= {} changed tokens; {} 5-hour windows\n",
                "",
                human_tokens(e.changed_tokens), e.windows_5h
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
    fn test_human_tokens_suffixes() {
        assert_eq!(human_tokens(0), "0");
        assert_eq!(human_tokens(999), "999");
        assert_eq!(human_tokens(1000), "1.00k");
        assert_eq!(human_tokens(12345), "12.3k");
        assert_eq!(human_tokens(100000), "100k");
        assert_eq!(human_tokens(1234567), "1.23M");
        assert_eq!(human_tokens(1234567890), "1.23B");
        assert_eq!(human_tokens(2_000_000_000_000), "2.00T");
    }

    #[test]
    fn test_align_dots() {
        let cells = vec![
            "5.7 person-months".to_string(),
            "6.2 person-months".to_string(),
            "6.4 person-years".to_string(),
            "—".to_string(),
        ];
        let aligned = align_dots(&cells);
        // The decimal points should line up: each numeric value is padded so
        // the '.' sits at the same column. The em-dash cell has no dot and is
        // skipped.
        let dots: Vec<usize> = aligned.iter()
            .filter(|s| s.trim() != "—")
            .map(|s| s.find('.').unwrap())
            .collect();
        assert!(dots.windows(2).all(|w| w[0] == w[1]), "dots not aligned: {aligned:?}");
    }

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
