use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Language;
use tree_sitter_language::LanguageFn;

#[derive(Clone)]
pub struct LanguageSpec {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub shebangs: &'static [&'static str],
    pub grammar_fn: LanguageFn,
    pub comment_kinds: &'static [&'static str],
}

impl LanguageSpec {
    pub fn grammar(&self) -> Language {
        Language::new(self.grammar_fn)
    }
}

pub fn registry() -> &'static LanguageRegistry {
    static REGISTRY: OnceLock<LanguageRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut reg = LanguageRegistry { languages: Vec::new() };
        reg.add(tree_sitter_c::LANGUAGE, "C", &[".c", ".h"], &[], &["comment"]);
        reg.add(
            tree_sitter_python::LANGUAGE,
            "Python",
            &[".py", ".pyw"],
            &["python"],
            &["comment"],
        );
        reg.add(
            tree_sitter_javascript::LANGUAGE,
            "JavaScript",
            &[".js", ".jsx", ".mjs", ".cjs"],
            &["node", "nodejs"],
            &["comment"],
        );
        reg.add(
            tree_sitter_rust::LANGUAGE,
            "Rust",
            &[".rs"],
            &[],
            &["line_comment", "block_comment"],
        );
        reg.add(
            tree_sitter_bash::LANGUAGE,
            "Bash",
            &[".sh", ".bash"],
            &["sh", "bash", "dash", "zsh"],
            &["comment"],
        );
        reg
    })
}

pub struct LanguageRegistry {
    languages: Vec<LanguageSpec>,
}

impl LanguageRegistry {
    fn add(
        &mut self,
        grammar_fn: LanguageFn,
        name: &'static str,
        extensions: &'static [&'static str],
        shebangs: &'static [&'static str],
        comment_kinds: &'static [&'static str],
    ) {
        self.languages.push(LanguageSpec { name, extensions, shebangs, grammar_fn, comment_kinds });
    }

    pub fn detect_by_ext(&self, path: &Path) -> Option<&LanguageSpec> {
        let ext = path.extension().and_then(|e| e.to_str())?;
        let dotted = format!(".{}", ext);
        self.languages.iter().find(|l| l.extensions.iter().any(|e| *e == dotted))
    }

    pub fn detect_by_shebang(&self, first_line: &[u8]) -> Option<&LanguageSpec> {
        if !first_line.starts_with(b"#!") {
            return None;
        }
        let line_str = std::str::from_utf8(first_line).unwrap_or("");
        self.languages.iter().find(|l| l.shebangs.iter().any(|s| line_str.contains(s)))
    }

    pub fn detect(&self, path: &Path, first_line: Option<&[u8]>) -> Option<&LanguageSpec> {
        if let Some(spec) = self.detect_by_ext(path) {
            return Some(spec);
        }
        self.detect_by_shebang(first_line?)
    }

    pub fn languages(&self) -> &[LanguageSpec] {
        &self.languages
    }
}
