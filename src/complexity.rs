use std::collections::HashMap;
use crate::language::LanguageSpec;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HalsteadMetrics {
    pub distinct_operators: u64,
    pub distinct_operands: u64,
    pub total_operators: u64,
    pub total_operands: u64,
    pub vocabulary: u64,
    pub length: u64,
    pub estimated_length: f64,
    pub volume: f64,
    pub difficulty: f64,
    pub effort: f64,
    pub time_seconds: f64,
    pub bugs: f64,
}

impl HalsteadMetrics {
    /// Add another metrics' raw counts (operators/operands) into this one.
    pub fn accumulate(&mut self, other: &HalsteadMetrics) {
        self.distinct_operators += other.distinct_operators;
        self.distinct_operands += other.distinct_operands;
        self.total_operators += other.total_operators;
        self.total_operands += other.total_operands;
    }

    /// Recompute the derived metrics (volume, difficulty, effort, time, bugs)
    /// from the raw operator/operand counts. Call after accumulation.
    pub fn derive(&mut self) {
        let n1 = self.distinct_operators; let n2 = self.distinct_operands;
        let t1 = self.total_operators; let t2 = self.total_operands;
        let n_vocab = n1 + n2; let n_len = t1 + t2;
        let volume = if n_vocab > 0 { (n_len as f64) * (n_vocab as f64).log2() } else { 0.0 };
        let diff = if n1 > 0 { (n1 as f64 / 2.0) * (t2 as f64 / n2.max(1) as f64) } else { 0.0 };
        self.vocabulary = n_vocab; self.length = n_len;
        self.estimated_length = if n1 > 0 && n2 > 0 {
            n1 as f64 * (n1 as f64).log2() + n2 as f64 * (n2 as f64).log2()
        } else { 0.0 };
        self.volume = volume; self.difficulty = diff;
        self.effort = diff * volume;
        self.time_seconds = self.effort / 18.0;
        self.bugs = volume / 3000.0;
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct McCabeMetrics {
    pub function_count: u64,
    pub total_cyclomatic: u64,
    pub average_cyclomatic: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HenryKafuraMetrics {
    pub total_modules: u64,
    pub total_fan_in: u64,
    pub total_fan_out: u64,
    pub total_information_flow: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeCounts {
    pub named_nodes: u64,
    pub leaf_tokens: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplexityResult {
    pub halstead: HalsteadMetrics,
    pub mccabe: McCabeMetrics,
    pub henry_kafura: HenryKafuraMetrics,
}

fn kind_is_function(kind: &str) -> bool {
    matches!(kind, "function_definition" | "method_definition"
        | "function_declaration" | "function_item"
        | "function_signature_item" | "function"
        | "method_declaration"
        | "local_function_statement" | "anonymous_function"
        | "lambda_expression" | "closure_expression"
        | "arrow_function"
        | "procedure_definition" | "procedure"
        | "constructor_declaration" | "constructor"
        | "getter" | "setter"
        | "fn" | "fun")
}

fn kind_is_decision(kind: &str) -> bool {
    matches!(kind, "if_statement" | "if_expression"
        | "while_statement" | "while_expression"
        | "for_statement" | "for_expression"
        | "for_in_statement" | "for_of_statement"
        | "loop_expression" | "do_statement"
        | "case_statement" | "switch_statement" | "switch_case"
        | "match_expression" | "match_case"
        | "catch_clause" | "catch"
        | "conditional_expression" | "ternary_expression"
        | "binary_expression" | "else_clause" | "else"
        | "elif" | "except" | "except_handler")
}

fn kind_is_binary_condition(kind: &str, text: &str) -> bool {
    kind == "binary_expression"
        && matches!(text, "&&" | "||" | "and" | "or")
}

fn kind_is_literal(kind: &str) -> bool {
    matches!(kind, "string" | "string_literal" | "character" | "number"
        | "integer_literal" | "float_literal" | "boolean" | "true" | "false"
        | "null" | "nil" | "none" | "undefined"
        | "comment" | "line_comment" | "block_comment"
        | "hash_comment" | "shebang" | "ERROR" | "MISSING")
}

fn kind_is_punctuation(kind: &str) -> bool {
    matches!(kind, "," | ";" | ":" | "." | "->" | "=>" | "::"
        | "(" | ")" | "{" | "}" | "[" | "]"
        | "template_literal" | "interpolation"
        | "escape_sequence")
}

fn kind_is_keyword(kind: &str) -> bool {
    matches!(kind, "if" | "else" | "while" | "for" | "do" | "switch"
        | "case" | "break" | "continue" | "return" | "throw"
        | "try" | "catch" | "finally" | "new" | "delete"
        | "typeof" | "instanceof" | "void" | "in" | "of"
        | "let" | "const" | "var" | "fn" | "fun" | "func"
        | "def" | "lambda" | "match" | "with" | "where"
        | "class" | "struct" | "enum" | "trait" | "impl"
        | "import" | "export" | "from" | "as" | "pub"
        | "use" | "mod" | "type" | "interface" | "abstract"
        | "static" | "virtual" | "override" | "async" | "await"
        | "yield" | "self" | "super" | "this"
        | "sizeof" | "alignof" | "offsetof" | "cast"
        | "and" | "or" | "not")
}

fn classify(kind: &str, text: &str, parent_kind: &str) -> bool {
    if kind_is_punctuation(kind) || kind_is_keyword(kind) {
        return true;
    }
    if kind_is_literal(kind) {
        return false;
    }
    if kind_is_function(kind)
        || matches!(parent_kind, "call_expression" | "binary_expression"
            | "unary_expression" | "assignment_expression"
            | "update_expression" | "type_cast_expression")
    {
        return true;
    }
    if kind_is_decision(kind) || kind_is_binary_condition(kind, text) {
        return true;
    }
    let is_identifier = matches!(kind, "identifier" | "variable_name"
        | "type_identifier" | "field_identifier"
        | "shorthand_property_identifier"
        | "shorthand_property_identifier_pattern");
    if is_identifier {
        return matches!(parent_kind, "call_expression" | "function_definition"
            | "method_definition" | "function_declaration"
            | "function_item" | "function_signature_item");
    }
    let is_expression = kind.ends_with("_expression")
        || matches!(kind, "parameter" | "arguments" | "argument"
            | "parameters" | "body" | "block" | "declaration"
            | "statement" | "program" | "source_file"
            | "ERROR" | "MISSING");
    !is_expression
}

pub fn analyze(source: &[u8], spec: &LanguageSpec) -> ComplexityResult {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&spec.grammar()).is_err() {
        return empty_result();
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return empty_result(),
    };

    let root = tree.root_node();
    let mut op_counts: HashMap<String, u64> = HashMap::new();
    let mut opd_counts: HashMap<String, u64> = HashMap::new();

    struct WalkState<'a> {
        ops: &'a mut HashMap<String, u64>,
        opds: &'a mut HashMap<String, u64>,
        decisions: u64,
        functions: u64,
        in_fn: bool,
        fn_depth: u32,
    }

    fn walk_node(node: tree_sitter::Node, source: &[u8], state: &mut WalkState) {
        let kind = node.kind();
        let is_named = node.is_named();
        let parent_kind = node.parent().as_ref().map_or("", |p| p.kind());

        if (!is_named || node.child_count() == 0)
            && let Ok(raw) = node.utf8_text(source) {
                let text = raw.trim();
                if !text.is_empty() {
                    let is_op = classify(kind, text, parent_kind);
                    let m = if is_op { &mut *state.ops } else { &mut *state.opds };
                    m.entry(text.to_string()).and_modify(|c| *c += 1).or_insert(1);
                }
            }

        if is_named && kind_is_function(kind) && !state.in_fn {
            state.functions += 1;
            state.in_fn = true;
            state.fn_depth = 0;
        }

        if is_named && state.in_fn && kind_is_decision(kind) {
            state.decisions += 1;
        }

        let mut child = node.walk();
        if child.goto_first_child() {
            if state.in_fn { state.fn_depth += 1; }
            loop {
                walk_node(child.node(), source, state);
                if !child.goto_next_sibling() { break; }
            }
            if state.in_fn {
                if state.fn_depth == 0 { state.in_fn = false; }
                else { state.fn_depth -= 1; }
            }
        }
    }

    let mut ws = WalkState {
        ops: &mut op_counts, opds: &mut opd_counts,
        decisions: 0, functions: 0,
        in_fn: false, fn_depth: 0,
    };
    walk_node(root, source, &mut ws);
    let decisions = ws.decisions;
    let functions = ws.functions;

    let mut halstead = HalsteadMetrics {
        distinct_operators: op_counts.len() as u64,
        distinct_operands: opd_counts.len() as u64,
        total_operators: op_counts.values().sum(),
        total_operands: opd_counts.values().sum(),
        ..Default::default()
    };
    halstead.derive();

    let total_cyclomatic = decisions + functions;
    let avg_cyclomatic = if functions > 0 { total_cyclomatic as f64 / functions as f64 } else { 0.0 };
    let mccabe = McCabeMetrics { function_count: functions, total_cyclomatic, average_cyclomatic: avg_cyclomatic };

    let hk = HenryKafuraMetrics { total_modules: 0, total_fan_in: 0, total_fan_out: 0, total_information_flow: 0.0 };

    ComplexityResult { halstead, mccabe, henry_kafura: hk }
}

fn empty_result() -> ComplexityResult {
    ComplexityResult {
        halstead: HalsteadMetrics::default(),
        mccabe: McCabeMetrics::default(),
        henry_kafura: HenryKafuraMetrics::default(),
    }
}

/// Aggregate per-language Halstead metrics into one: sum the raw
/// operator/operand counts, then derive volume/effort/time. `None` when the
/// input is empty.
pub fn aggregate_halstead<'a>(
    metrics: impl IntoIterator<Item = &'a HalsteadMetrics>,
) -> Option<HalsteadMetrics> {
    let mut acc = HalsteadMetrics::default();
    let mut any = false;
    for h in metrics {
        acc.accumulate(h);
        any = true;
    }
    if !any { return None; }
    acc.derive();
    Some(acc)
}
