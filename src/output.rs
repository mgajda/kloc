use crate::color::{Colors, logo_colors};
use crate::history::HistoryReport;
use crate::report::Report;

pub enum OutputFormat {
    Text,
    Json,
}

/// One estimation methodology column in the grouped schedule table. Rows are
/// Metric / Effort / Team size / Schedule. Add a new methodology by pushing
/// another `MetricCol`; `group` names the spanning header it belongs to.
#[derive(Clone)]
pub(crate) struct MetricCol {
    pub group: &'static str,
    pub label: String,
    /// Foreground colour for this column (1=red..7=white).
    pub color: u8,
    /// Background colour shared by all columns in the same group.
    /// `None` = no background (transparent, matches the terminal).
    pub bg: Option<u8>,
    pub metric: String,
    pub effort: String,
    pub team: String,
    pub schedule: String,
}

/// Render the grouped schedule table from an arbitrary list of columns.
///
/// The table has four fixed rows (Metric / Effort / Team size / Schedule)
/// and one column per `MetricCol`, grouped under spanning headers. Cells a
/// model does not produce are "—". Decimals are aligned within each column.
/// Kept separate from [`format_text`] so the layout can be tested
/// independently of how the columns are populated.
fn render_schedule_table(cols: &[MetricCol], colors: &Colors) -> String {
    let mut out = String::new();
    if cols.is_empty() {
        out.push_str("(no schedule data)\n");
        return out;
    }
    let n = cols.len();

    // Width of the row-label column = the widest row label.
    let label_w = ["Method", "Metric", "Effort", "Team size", "Schedule"]
        .iter().map(|s| s.len()).max().unwrap();

    // Align each column's 4 row values on the decimal point.
    let mut aligned: Vec<[String; 4]> = Vec::with_capacity(n);
    for c in cols {
        let cells = [c.metric.clone(), c.effort.clone(), c.team.clone(), c.schedule.clone()];
        let a = align_dots(&cells);
        aligned.push([a[0].clone(), a[1].clone(), a[2].clone(), a[3].clone()]);
    }

    // Column widths; index 0 is the row-label column.
    let mut widths: Vec<usize> = vec![label_w];
    for (i, c) in cols.iter().enumerate() {
        let w = c.label.len()
            .max(aligned[i][0].len()).max(aligned[i][1].len())
            .max(aligned[i][2].len()).max(aligned[i][3].len());
        widths.push(w);
    }

    // Colour a cell: bright foreground over the column's group background
    // (a 256-colour grey shade), or just bright foreground when the group has
    // no background.
    let paint = |cell: &str, fg: u8, bg: Option<u8>| -> String {
        match bg {
            Some(b) => colors.on_bg256(cell, fg, b),
            None => colors.ansi(cell, fg),
        }
    };

    let render = |cells: &[String], fgs: &[u8]| -> String {
        let mut l = format!("{:<label_w$}", cells[0]);
        for (i, v) in cells[1..].iter().enumerate() {
            // Left-align: align_dots pads the integer part so the '.' sits at
            // a fixed index from the cell start; right-aligning would undo it.
            let cell = format!("{:<width$}", v, width = widths[i + 1]);
            let cell = paint(&cell, fgs[i], cols[i].bg);
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
        let label = format!("{g:^width$}", width = w);
        group_header.push_str(&format!("  {}", paint(&label, 7, cols[gi].bg)));
        gi += span;
    }
    out.push_str(&format!("{:<label_w$}{group_header}\n", ""));

    // Column header row.
    let mut hdr = format!("{:<label_w$}", "Method");
    for (i, c) in cols.iter().enumerate() {
        let cell = format!("{:<width$}", c.label, width = widths[i + 1]);
        hdr.push_str(&format!("  {}", paint(&cell, c.color, c.bg)));
    }
    out.push_str(&format!("{hdr}\n"));

    let row_fgs: Vec<u8> = cols.iter().map(|c| c.color).collect();

    let mut metric_row = vec!["Metric".to_string()];
    metric_row.extend(aligned.iter().map(|a| a[0].clone()));
    out.push_str(&format!("{}\n", render(&metric_row, &row_fgs)));

    let mut effort_row = vec!["Effort".to_string()];
    effort_row.extend(aligned.iter().map(|a| a[1].clone()));
    out.push_str(&format!("{}\n", render(&effort_row, &row_fgs)));

    let mut team_row = vec!["Team size".to_string()];
    team_row.extend(aligned.iter().map(|a| a[2].clone()));
    out.push_str(&format!("{}\n", render(&team_row, &row_fgs)));

    let mut schedule_row = vec!["Schedule".to_string()];
    schedule_row.extend(aligned.iter().map(|a| a[3].clone()));
    out.push_str(&format!("{}\n", render(&schedule_row, &row_fgs)));

    out
}

/// Build the schedule-table columns from a [`crate::schedule::ScheduleReport`]
/// plus LLM token counts and Halstead volume/time. Shared by the normal
/// report and the `--history` report so both render identical tables —
/// the only difference is what feeds the schedule estimate.
///
/// `leaf_tokens` is the tree-sitter token count, used for the human
/// "Tree-sitter" column: a human programmer writes roughly 200–2000
/// tree-sitter tokens (≈50–500 LOC) per day, so effort/schedule is
/// `leaf_tokens / daily_rate` in person-days.
/// Build the `(platform name, token count)` list for the schedule table from
/// the known `TokenCounts` fields. Unknown config platforms get count 0.
fn ai_tokens_for_config(tokens: Option<crate::TokenCounts>, cfg: &crate::ai_config::AiConfig) -> Vec<(String, u64)> {
    let t = tokens.unwrap_or_default();
    cfg.platforms.iter().map(|p| {
        let count = match p.name.as_str() {
            "Claude Sonnet" | "Claude Pro" | "Claude Max 5x" | "Claude Max 20x" => t.claude_sonnet,
            "DeepSeek V4" | "DeepSeek V4 (OpenCode)" => t.deepseek_v4,
            _ => 0,
        };
        (p.name.clone(), count)
    }).collect()
}

fn build_schedule_cols(
    s: &crate::schedule::ScheduleReport,
    ai_tokens: &[(String, u64)],
    ai_config: &crate::ai_config::AiConfig,
    ai_multiplier_override: Option<f64>,
    leaf_tokens: u64,
    halstead_volume: Option<f64>,
) -> Vec<MetricCol> {
    let months = |m: f64| human_duration(m * 30.44 * 24.0 * 3600.0);

    // Human tree-sitter productivity: ~200–2000 tokens (≈50–500 LOC) per day.
    // We use the middle of that range as the daily rate.
    const HUMAN_TOKENS_PER_DAY: f64 = 1100.0;
    let human_duration_str = if leaf_tokens > 0 {
        human_duration((leaf_tokens as f64 / HUMAN_TOKENS_PER_DAY) * 24.0 * 3600.0)
    } else {
        "—".to_string()
    };
    let human_metric = if leaf_tokens > 0 {
        format!("{} tokens", human_tokens(leaf_tokens))
    } else {
        "—".to_string()
    };

    // One AI column per configured platform. Metric and Effort are measured
    // in tokens (ISO magnitude suffix); Schedule is the platform's plan-cap
    // time to process those tokens. No team size. Each platform carries its
    // own caps and effort multiplier.
    let mut ai_cols: Vec<MetricCol> = Vec::new();
    for (idx, p) in ai_config.platforms.iter().enumerate() {
        let count = ai_tokens.iter().find(|(name, _)| name == &p.name)
            .map(|&(_, n)| n).unwrap_or(0);
        let caps = crate::history::AiCaps::from_cfg(p);
        let multiplier = ai_multiplier_override.unwrap_or(p.multiplier.unwrap_or(5.0));
        let ai_dur = |n: u64| crate::history::ai_duration(n, &caps, multiplier);
        let ai_effort = |n: u64| format!("{} tokens", human_tokens(crate::history::effective_tokens(n, multiplier)));
        // Cycle colours so each column is distinct.
        let color = [6u8, 1, 3, 4, 2, 5][idx % 6];
        ai_cols.push(MetricCol {
            group: "AI", label: p.name.clone(), color, bg: Some(236),
            metric: if count > 0 { format!("{} tokens", human_tokens(count)) } else { "—".to_string() },
            effort: if count > 0 { ai_effort(count) } else { "—".to_string() },
            team: "—".to_string(),
            schedule: if count > 0 { ai_dur(count) } else { "—".to_string() },
        });
    }

    // Classical models: metric is code size (lines of code, with a k human
    // suffix); effort in person-months; team size where the model has one.
    let mut cols = vec![
        MetricCol {
            group: "LoC-driven", label: "COCOMO 1".to_string(), color: 4, bg: Some(235),
            metric: format!("{:.1} k lines of code", s.ksloc),
            effort: human_person_months(s.cocomo.effort_person_months),
            team: format!("{:.1}", s.cocomo.avg_people),
            schedule: months(s.cocomo.schedule_months),
        },
        MetricCol {
            group: "LoC-driven", label: "COCOMO 2".to_string(), color: 2, bg: Some(235),
            metric: format!("{:.1} k lines of code", s.ksloc),
            effort: human_person_months(s.cocomo_ii.effort_person_months),
            team: format!("{:.1}", s.cocomo_ii.avg_people),
            schedule: months(s.cocomo_ii.schedule_months),
        },
        MetricCol {
            group: "LoC-driven", label: "Putnam".to_string(), color: 3, bg: Some(235),
            metric: format!("{:.1} k lines of code", s.ksloc),
            effort: human_person_months(s.cocomo_ii.effort_person_months),
            team: format!("{:.1}", s.putnam.avg_people),
            schedule: months(s.putnam.schedule_months),
        },
        MetricCol {
            group: "AST-driven", label: "Tree-sitter".to_string(), color: 7, bg: None,
            metric: human_metric,
            effort: human_duration_str.clone(),
            team: "—".to_string(),
            schedule: human_duration_str,
        },
        MetricCol {
            group: "AST-driven", label: "Halstead".to_string(), color: 5, bg: None,
            metric: match halstead_volume {
                Some(v) => format!("{} volume", human_tokens(v as u64)),
                None => "—".to_string(),
            },
            effort: human_person_months(s.halstead.effort_person_months),
            team: format!("{:.1}", s.halstead.avg_people),
            schedule: months(s.halstead.schedule_months),
        },
    ];
    cols.extend(ai_cols);
    cols
}

pub fn format(
    report: &Report,
    format: &OutputFormat,
    full: bool,
    colors: Colors,
    ai_config: &crate::ai_config::AiConfig,
    ai_multiplier_override: Option<f64>,
) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(report).unwrap(),
        OutputFormat::Text => format_text(report, full, colors, ai_config, ai_multiplier_override),
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
    const KILOYEAR: f64 = 1000.0 * YEAR;
    const MEGAYEAR: f64 = 1000.0 * KILOYEAR;

    let (value, unit) = if seconds >= MEGAYEAR {
        (seconds / MEGAYEAR, "Mya")
    } else if seconds >= KILOYEAR {
        (seconds / KILOYEAR, "kya")
    } else if seconds >= YEAR {
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
    const KILO_YEARS: f64 = 12.0 * 1000.0;

    let (value, unit) = if pm >= KILO_YEARS {
        (pm / KILO_YEARS, "thousand person-years")
    } else if pm >= 12.0 {
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

fn format_text(
    report: &Report,
    full: bool,
    colors: Colors,
    ai_config: &crate::ai_config::AiConfig,
    ai_multiplier_override: Option<f64>,
) -> String {
    let mut out = String::new();
    out.push_str("SLOC by language:\n\n");
    // Column headers, coloured to match the schedule-table metrics
    // (LoC-driven = blue, files = yellow, tree-sitter = white, AI = cyan).
    let loc_fg = 4u8;     // blue (LoC-driven LOC)
    let files_fg = 3u8;   // yellow
    let ts_fg = 7u8;      // white (tree-sitter)
    let ai_fg = 6u8;      // cyan (AI)
    let hdr = format!(
        "{:<12}{} {} {} {}\n",
        "Language",
        colors.ansi(&format!("{:>10}", "LOC"), loc_fg),
        colors.ansi(&format!("{:>8}", "Files"), files_fg),
        colors.ansi(&format!("{:>12}", "Tree-sit. tok"), ts_fg),
        colors.ansi(&format!("{:>12}", "AI tokens"), ai_fg),
    );
    out.push_str(&hdr);
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
        let sloc = format!("{:>10}", lang.sloc);
        let files = format!("{:>8}", lang.files);
        let leaf = format!("{:>12}", human_tokens(lang.leaf_tokens));
        let ai = format!("{:>12}", human_tokens(lang.ai_tokens));
        // Colour after padding so ANSI codes don't affect column alignment.
        out.push_str(&format!(
            "{}{} {} {} {} ({:.2}%)\n",
            name_field,
            colors.ansi(&sloc, loc_fg),
            colors.ansi(&files, files_fg),
            colors.ansi(&leaf, ts_fg),
            colors.ansi(&ai, ai_fg),
            pct,
        ));
    }

    out.push('\n');
    out.push_str(&format!("{:44}= {}\n", "Total lines of code without comments", report.total_sloc));
    out.push_str(&format!("{:44}= {}\n", "Total non-empty lines with comments", report.total_comments));
    out.push_str(&format!("{:44}= {}\n", "Total files", report.total_files));

    out.push('\n');
    // Tree-sitter named nodes are not shown elsewhere; keep as a single line.
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

        let halstead_volume = report.halstead.as_ref().map(|h| h.volume);
        let ai_tokens = ai_tokens_for_config(report.llm_tokens, ai_config);
        let cols = build_schedule_cols(s, &ai_tokens, ai_config, ai_multiplier_override, report.nodes.leaf_tokens, halstead_volume);
        out.push_str(&render_schedule_table(&cols, &colors));
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
pub fn format_history(
    report: &HistoryReport,
    colors: Colors,
    ai_config: &crate::ai_config::AiConfig,
    ai_multiplier_override: Option<f64>,
) -> String {
    let mut out = String::new();
    out.push_str("Git history:\n\n");
    out.push_str(&format!("{:44}= {}\n", "Commits analyzed", report.commits));
    out.push_str(&format!("{:44}= {}\n", "Lines added (parsed)", report.total_added_lines));
    out.push_str(&format!("{:44}= {}\n", "Lines removed (parsed)", report.total_removed_lines));
    out.push_str(&format!("{:44}= {}\n", "All diff lines added", report.all_added_lines));
    out.push_str(&format!("{:44}= {}\n", "All diff lines removed", report.all_removed_lines));

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

    // Same grouped schedule table as the normal report, but estimated from
    // the diff-added lines and changed tokens rather than the full source.
    let halstead_volume = report.halstead.as_ref().map(|h| h.volume);
    let ai_tokens = ai_tokens_for_config(report.llm_changed_tokens, ai_config);
    let cols = build_schedule_cols(
        &report.schedule,
        &ai_tokens,
        ai_config,
        ai_multiplier_override,
        report.leaf_tokens,
        halstead_volume,
    );
    out.push_str("\n--- Schedule (from diffs) ---\n\n");
    out.push_str(&render_schedule_table(&cols, &colors));
    out.push_str("\nAI durations are approximate calibration (Anthropic publishes plan\n");
    out.push_str("multiples, not absolute token numbers); override with --ai-budget.\n");

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

    #[test]
    fn test_human_duration_kya_mya() {
        const YEAR: f64 = 12.0 * 30.44 * 24.0 * 3600.0;
        const KILOYEAR: f64 = 1000.0 * YEAR;
        const MEGAYEAR: f64 = 1_000_000.0 * YEAR;
        // Well under a year → months.
        assert_eq!(human_duration(0.5 * YEAR), "6.0 months");
        // Hundreds of years → years.
        assert_eq!(human_duration(500.0 * YEAR), "500.0 years");
        // Thousands of years → kya.
        assert_eq!(human_duration(2_400.0 * YEAR), "2.4 kya");
        // Millions of years → Mya.
        assert_eq!(human_duration(2_400_000.0 * YEAR), "2.4 Mya");
        // 2.4 billion years is still Mya.
        assert_eq!(human_duration(2_400_000_000.0 * YEAR), "2400.0 Mya");
        // Boundary: exactly 1000 years → kya, 1e6 years → Mya.
        assert_eq!(human_duration(KILOYEAR), "1.0 kya");
        assert_eq!(human_duration(MEGAYEAR), "1.0 Mya");
    }

    // ---- Schedule-table layout tests -------------------------------------

    fn col(group: &'static str, label: &str, metric: &str, effort: &str, team: &str, schedule: &str) -> MetricCol {
        MetricCol { group, label: label.to_string(), color: 1, bg: None, metric: metric.to_string(), effort: effort.to_string(), team: team.to_string(), schedule: schedule.to_string() }
    }

    fn csi_strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // consume until 'm'
                for ch in chars.by_ref() {
                    if ch == 'm' { break; }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn table_all_lines_equal_width() {
        // All data rows must be the same width regardless of column count.
        for ncols in 1..=6 {
            let labels = ["A", "B", "C", "D", "E", "F"];
            let mut cols = Vec::new();
            for i in 0..ncols {
                let g = if i % 2 == 0 { "G1" } else { "G2" };
                cols.push(col(g, labels[i], &format!("{i}.5 x"), "2.0 person-months", "3", "4.0 months"));
            }
            let out = render_schedule_table(&cols, &Colors::force(false));
            let lines: Vec<String> = out.lines().map(csi_strip).filter(|l| !l.trim().is_empty()).collect();
            // Skip the group-header row (it may not align); check data rows.
            let data: Vec<&String> = lines.iter().skip(2).collect();
            let widths: Vec<usize> = data.iter().map(|l| l.chars().count()).collect();
            assert!(widths.windows(2).all(|w| w[0] == w[1]),
                "unequal row widths for {ncols} cols: {widths:?}\n{out}");
        }
    }

    #[test]
    fn table_all_row_labels_present() {
        let cols = vec![
            col("G1", "A", "1 x", "2 person-months", "3", "4 months"),
            col("G1", "B", "1 x", "2 person-months", "—", "4 months"),
        ];
        let out = render_schedule_table(&cols, &Colors::force(false));
        let plain = csi_strip(&out);
        for label in ["Method", "Metric", "Effort", "Team size", "Schedule"] {
            assert!(plain.contains(label), "missing row label {label:?} in:\n{plain}");
        }
    }

    #[test]
    fn table_decimals_aligned_across_rows() {
        // Within each column the decimal points of numeric cells must align.
        let cols = vec![
            col("G1", "A", "10.5 x", "2.25 person-months", "1.5", "4.0 months"),
            col("G1", "B", "9.5 x", "1.5 person-months", "2.0", "30.5 months"),
        ];
        let out = render_schedule_table(&cols, &Colors::force(false));
        let lines: Vec<String> = out.lines().map(csi_strip).filter(|l| !l.trim().is_empty()).collect();
        // lines: [group, Method, Metric, Effort, Team size, Schedule]
        // For each column, the dot must sit at the same char index in the
        // Metric and Schedule rows (both contain decimals).
        let metric = &lines[2];
        let schedule = &lines[5];
        let find_dot = |l: &str, nth: usize| {
            l.char_indices().filter(|(_, c)| *c == '.').nth(nth).map(|(i, _)| i)
        };
        let dots: Vec<Option<usize>> = (0..2).map(|k| find_dot(&metric, k)).collect();
        let sdots: Vec<Option<usize>> = (0..2).map(|k| find_dot(&schedule, k)).collect();
        assert_eq!(dots[0], sdots[0], "col A dot misaligned:\n{metric}\n{schedule}");
        assert_eq!(dots[1], sdots[1], "col B dot misaligned:\n{metric}\n{schedule}");
    }

    #[test]
    fn table_group_headers_span() {
        // Two groups; the group header must appear, and the "Method" row must
        // list every column label in order.
        let cols = vec![
            col("LoC-driven", "COCOMO 1", "1 x", "2", "3", "4 months"),
            col("LoC-driven", "COCOMO 2", "1 x", "2", "3", "4 months"),
            col("Halstead", "Halstead", "1 x", "2", "—", "4 months"),
            col("AI", "Claude Sonnet", "1 x", "2", "—", "4 months"),
            col("AI", "DeepSeek V4 (OpenCode)", "1 x", "2", "—", "4 months"),
        ];
        let out = render_schedule_table(&cols, &Colors::force(false));
        let plain = csi_strip(&out);
        for label in ["LoC-driven", "Halstead", "AI"] {
            assert!(plain.contains(label), "missing group header {label:?}:\n{plain}");
        }
        // Column labels in order.
        let method_line = plain.lines().find(|l| l.contains("Method")).unwrap().to_string();
        for label in ["COCOMO 1", "COCOMO 2", "Halstead", "Claude Sonnet", "DeepSeek V4 (OpenCode)"] {
            assert!(method_line.contains(label), "missing column {label:?} in {method_line:?}");
        }
    }

    #[test]
    fn table_empty_yields_message() {
        let out = render_schedule_table(&[], &Colors::force(false));
        assert!(out.contains("no schedule data"));
    }

    #[test]
    fn table_single_column() {
        let cols = vec![col("Only", "Solo", "5.5 x", "2.0 person-months", "—", "1.0 months")];
        let out = render_schedule_table(&cols, &Colors::force(false));
        assert!(out.contains("Solo"));
        assert!(out.contains("5.5 x"));
        assert!(out.contains("1.0 months"));
    }

    #[test]
    fn table_colors_do_not_break_alignment() {
        // Colored vs uncolored output must produce the same aligned plain text.
        let mk = |color: bool| {
            let cols = vec![
                col("G1", "COCOMO 1", "1.5 x", "2.0 person-months", "3", "4.0 months"),
                col("G2", "Halstead", "1.5 x", "2.0 person-months", "—", "4.0 months"),
            ];
            render_schedule_table(&cols, &Colors::force(color))
        };
        let colored = mk(true);
        let plain_out = mk(false);
        // Stripping ANSI from the colored output must equal the uncolored output.
        let stripped: Vec<String> = colored.lines().map(csi_strip).collect();
        let expected: Vec<String> = plain_out.lines().map(|l| l.to_string()).collect();
        assert_eq!(stripped, expected, "color changed layout");
    }

}

