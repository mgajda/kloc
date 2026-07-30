use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Language;

type GrammarFn = fn() -> Language;

#[derive(Clone)]
pub struct LanguageSpec {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub shebangs: &'static [&'static str],
    pub grammar_fn: GrammarFn,
    pub comment_kinds: &'static [&'static str],
}

impl LanguageSpec {
    pub fn grammar(&self) -> Language {
        (self.grammar_fn)()
    }
}

pub fn registry() -> &'static LanguageRegistry {
    static REGISTRY: OnceLock<LanguageRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut reg = LanguageRegistry { languages: Vec::new() };
        macro_rules! add_lang {
            ($exts:expr, $shebangs:expr, $name:expr, $comments:expr, $fn:expr) => {{
                reg.languages.push(LanguageSpec {
                    name: $name,
                    extensions: $exts,
                    shebangs: $shebangs,
                    grammar_fn: $fn,
                    comment_kinds: $comments,
                });
            }};
        }

        // Rust (always in default)
        add_lang!(&[], &[], "Rust", &["line_comment", "block_comment"], || {
            Language::new(tree_sitter_rust::LANGUAGE)
        });

        // C (default)
        #[cfg(feature = "c")]
        add_lang!(&[".c", ".h"], &[], "C", &["comment"], || {
            Language::new(tree_sitter_c::LANGUAGE)
        });

        // Python (default)
        #[cfg(feature = "python")]
        add_lang!(&[".py", ".pyw"], &["python"], "Python", &["comment"], || {
            Language::new(tree_sitter_python::LANGUAGE)
        });

        // JavaScript (default)
        #[cfg(feature = "javascript")]
        add_lang!(
            &[".js", ".jsx", ".mjs", ".cjs"],
            &["node", "nodejs"],
            "JavaScript",
            &["comment"],
            || { Language::new(tree_sitter_javascript::LANGUAGE) }
        );

        // Bash (default)
        #[cfg(feature = "bash")]
        add_lang!(&[".sh", ".bash"], &["sh", "bash", "dash", "zsh"], "Bash", &["comment"], || {
            Language::new(tree_sitter_bash::LANGUAGE)
        });

        // Haskell
        #[cfg(feature = "haskell")]
        add_lang!(&[".hs", ".lhs"], &["runhaskell"], "Haskell", &["comment"], || {
            Language::new(tree_sitter_haskell::LANGUAGE)
        });

        // OCaml
        #[cfg(feature = "ocaml")]
        add_lang!(&[".ml"], &[], "OCaml", &["comment"], || {
            Language::new(tree_sitter_ocaml::LANGUAGE_OCAML)
        });
        #[cfg(feature = "ocaml")]
        add_lang!(&[".mli"], &[], "OCaml", &["comment"], || {
            Language::new(tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE)
        });

        // Elm
        #[cfg(feature = "elm")]
        add_lang!(&[".elm"], &[], "Elm", &["comment"], || {
            Language::new(tree_sitter_elm::LANGUAGE)
        });

        // Go
        #[cfg(feature = "go")]
        add_lang!(&[".go"], &[], "Go", &["comment"], || {
            Language::new(tree_sitter_go::LANGUAGE)
        });

        // TypeScript / TSX
        #[cfg(feature = "typescript")]
        add_lang!(&[".ts"], &[], "TypeScript", &["comment"], || {
            Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT)
        });
        #[cfg(feature = "typescript")]
        add_lang!(&[".tsx"], &[], "TSX", &["comment"], || {
            Language::new(tree_sitter_typescript::LANGUAGE_TSX)
        });

        // Java
        #[cfg(feature = "java")]
        add_lang!(&[".java"], &[], "Java", &["comment"], || {
            Language::new(tree_sitter_java::LANGUAGE)
        });

        // Scala
        #[cfg(feature = "scala")]
        add_lang!(&[".scala", ".sc"], &[], "Scala", &["comment"], || {
            Language::new(tree_sitter_scala::LANGUAGE)
        });

        reg
    })
}

pub struct LanguageRegistry {
    languages: Vec<LanguageSpec>,
}

impl LanguageRegistry {
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
