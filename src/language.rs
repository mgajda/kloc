use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Language;

type GrammarFn = fn() -> Language;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageCategory {
    Programming,
    Machine,
}

#[derive(Clone)]
pub struct LanguageSpec {
    pub name: &'static str,
    pub category: LanguageCategory,
    pub extensions: &'static [&'static str],
    pub shebangs: &'static [&'static str],
    pub filenames: &'static [&'static str],
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
            ($cat:ident, $exts:expr, $shebangs:expr, $fnames:expr, $name:expr, $comments:expr, $fn:expr) => {{
                reg.languages.push(LanguageSpec {
                    name: $name,
                    category: LanguageCategory::$cat,
                    extensions: $exts,
                    shebangs: $shebangs,
                    filenames: $fnames,
                    grammar_fn: $fn,
                    comment_kinds: $comments,
                });
            }};
        }

        // Default: Programming
        add_lang!(Programming, &[".rs"], &[], &[], "Rust", &["line_comment", "block_comment"],
            || Language::new(tree_sitter_rust::LANGUAGE));
        #[cfg(feature = "c")]
        add_lang!(Programming, &[".c", ".h"], &[], &[], "C", &["comment"],
            || Language::new(tree_sitter_c::LANGUAGE));
        #[cfg(feature = "python")]
        add_lang!(Programming, &[".py", ".pyw"], &["python"], &[], "Python", &["comment"],
            || Language::new(tree_sitter_python::LANGUAGE));
        #[cfg(feature = "javascript")]
        add_lang!(Programming, &[".js", ".jsx", ".mjs", ".cjs"], &["node", "nodejs"], &[], "JavaScript", &["comment"],
            || Language::new(tree_sitter_javascript::LANGUAGE));
        #[cfg(feature = "bash")]
        add_lang!(Programming, &[".sh", ".bash"], &["sh", "bash", "dash", "zsh"], &[], "Bash", &["comment"],
            || Language::new(tree_sitter_bash::LANGUAGE));
        #[cfg(feature = "haskell")]
        add_lang!(Programming, &[".hs", ".lhs"], &["runhaskell"], &[], "Haskell", &["comment"],
            || Language::new(tree_sitter_haskell::LANGUAGE));
        #[cfg(feature = "ocaml")]
        add_lang!(Programming, &[".ml"], &[], &[], "OCaml", &["comment"],
            || Language::new(tree_sitter_ocaml::LANGUAGE_OCAML));
        #[cfg(feature = "ocaml")]
        add_lang!(Programming, &[".mli"], &[], &[], "OCaml", &["comment"],
            || Language::new(tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE));
        #[cfg(feature = "elm")]
        add_lang!(Programming, &[".elm"], &[], &[], "Elm", &["comment"],
            || Language::new(tree_sitter_elm::LANGUAGE));
        #[cfg(feature = "go")]
        add_lang!(Programming, &[".go"], &[], &[], "Go", &["comment"],
            || Language::new(tree_sitter_go::LANGUAGE));
        #[cfg(feature = "typescript")]
        add_lang!(Programming, &[".ts"], &[], &[], "TypeScript", &["comment"],
            || Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT));
        #[cfg(feature = "typescript")]
        add_lang!(Programming, &[".tsx"], &[], &[], "TSX", &["comment"],
            || Language::new(tree_sitter_typescript::LANGUAGE_TSX));
        #[cfg(feature = "java")]
        add_lang!(Programming, &[".java"], &[], &[], "Java", &["line_comment", "block_comment"],
            || Language::new(tree_sitter_java::LANGUAGE));
        #[cfg(feature = "scala")]
        add_lang!(Programming, &[".scala", ".sc"], &[], &[], "Scala", &["comment"],
            || Language::new(tree_sitter_scala::LANGUAGE));
        #[cfg(feature = "ada")]
        add_lang!(Programming, &[".ada", ".ads", ".adb"], &[], &[], "Ada", &["comment"],
            || Language::new(tree_sitter_ada::LANGUAGE));
        #[cfg(feature = "agda")]
        add_lang!(Programming, &[".agda", ".lagda"], &[], &[], "Agda", &["comment"],
            || Language::new(tree_sitter_agda::LANGUAGE));
        #[cfg(feature = "cpp")]
        add_lang!(Programming, &[".cpp", ".cxx", ".cc", ".c++", ".hpp", ".hxx", ".hh", ".h++"], &[], &[], "C++", &["comment"],
            || Language::new(tree_sitter_cpp::LANGUAGE));
        #[cfg(feature = "csharp")]
        add_lang!(Programming, &[".cs"], &[], &[], "C#", &["comment"],
            || Language::new(tree_sitter_c_sharp::LANGUAGE));
        #[cfg(feature = "dart")]
        add_lang!(Programming, &[".dart"], &[], &[], "Dart", &["comment"],
            || Language::new(tree_sitter_dart::LANGUAGE));
        #[cfg(feature = "elixir")]
        add_lang!(Programming, &[".ex", ".exs"], &["elixir"], &[], "Elixir", &["comment"],
            || Language::new(tree_sitter_elixir::LANGUAGE));
        #[cfg(feature = "erlang")]
        add_lang!(Programming, &[".erl", ".hrl"], &[], &[], "Erlang", &["comment"],
            || Language::new(tree_sitter_erlang::LANGUAGE));
        #[cfg(feature = "fish")]
        add_lang!(Programming, &[".fish"], &["fish"], &[], "Fish", &["comment"],
            tree_sitter_fish::language);
        #[cfg(feature = "fortran")]
        add_lang!(Programming, &[".f", ".f90", ".f95", ".f03", ".f08"], &[], &[], "Fortran", &["comment"],
            || Language::new(tree_sitter_fortran::LANGUAGE));
        #[cfg(feature = "fsharp")]
        add_lang!(Programming, &[".fs", ".fsx", ".fsi"], &[], &[], "F#", &["comment"],
            || Language::new(tree_sitter_fsharp::LANGUAGE_FSHARP));
        #[cfg(feature = "gleam")]
        add_lang!(Programming, &[".gleam"], &[], &[], "Gleam", &["comment"],
            || Language::new(tree_sitter_gleam::LANGUAGE));
        #[cfg(feature = "glsl")]
        add_lang!(Programming, &[".vert", ".frag", ".geom", ".comp", ".tesc", ".tese", ".glsl"], &[], &[], "GLSL", &["comment"],
            || Language::new(tree_sitter_glsl::LANGUAGE_GLSL));
        #[cfg(feature = "graphql")]
        add_lang!(Programming, &[".graphql", ".gql"], &[], &[], "GraphQL", &["comment"],
            || Language::new(tree_sitter_graphql::LANGUAGE));
        #[cfg(feature = "groovy")]
        add_lang!(Programming, &[".groovy", ".gvy", ".gy", ".gsh"], &[], &[], "Groovy", &["comment"],
            || Language::new(tree_sitter_groovy::LANGUAGE));
        #[cfg(feature = "julia")]
        add_lang!(Programming, &[".jl"], &[], &[], "Julia", &["comment"],
            || Language::new(tree_sitter_julia::LANGUAGE));
        #[cfg(feature = "lua")]
        add_lang!(Programming, &[".lua"], &["lua"], &[], "Lua", &["comment"],
            || Language::new(tree_sitter_lua::LANGUAGE));
        #[cfg(feature = "make")]
        add_lang!(Programming, &[".mak", ".mk"], &[], &["Makefile", "makefile", "GNUmakefile"], "Make", &["comment"],
            || Language::new(tree_sitter_make::LANGUAGE));
        #[cfg(feature = "nix")]
        add_lang!(Programming, &[".nix"], &[], &[], "Nix", &["comment"],
            || Language::new(tree_sitter_nix::LANGUAGE));
        #[cfg(feature = "odin")]
        add_lang!(Programming, &[".odin"], &[], &[], "Odin", &["comment"],
            || Language::new(tree_sitter_odin::LANGUAGE));
        #[cfg(feature = "pascal")]
        add_lang!(Programming, &[".pas"], &[], &[], "Pascal", &["comment"],
            || Language::new(tree_sitter_pascal::LANGUAGE));
        #[cfg(feature = "perl")]
        add_lang!(Programming, &[".pl", ".pm", ".t"], &["perl"], &[], "Perl", &["comment"],
            || Language::new(tree_sitter_perl::LANGUAGE));
        #[cfg(feature = "php")]
        add_lang!(Programming, &[".php", ".phtml", ".php3", ".php4", ".php5"], &[], &[], "PHP", &["comment"],
            || Language::new(tree_sitter_php::LANGUAGE_PHP));
        #[cfg(feature = "powershell")]
        add_lang!(Programming, &[".ps1", ".psm1", ".psd1"], &["pwsh"], &[], "PowerShell", &["comment"],
            || Language::new(tree_sitter_powershell::LANGUAGE));
        #[cfg(feature = "r")]
        add_lang!(Programming, &[".r", ".R", ".rmd"], &["R"], &[], "R", &["comment"],
            || Language::new(tree_sitter_r::LANGUAGE));
        #[cfg(feature = "ruby")]
        add_lang!(Programming, &[".rb"], &["ruby"], &[], "Ruby", &["comment"],
            || Language::new(tree_sitter_ruby::LANGUAGE));
        #[cfg(feature = "scheme")]
        add_lang!(Programming, &[".scm", ".ss"], &[], &[], "Scheme", &["comment"],
            || Language::new(tree_sitter_scheme::LANGUAGE));
        #[cfg(feature = "slint")]
        add_lang!(Programming, &[".slint"], &[], &[], "Slint", &["comment"],
            || Language::new(tree_sitter_slint::LANGUAGE));
        #[cfg(feature = "solidity")]
        add_lang!(Programming, &[".sol"], &[], &[], "Solidity", &["comment"],
            || Language::new(tree_sitter_solidity::LANGUAGE));
        #[cfg(feature = "swift")]
        add_lang!(Programming, &[".swift"], &[], &[], "Swift", &["comment"],
            || Language::new(tree_sitter_swift::LANGUAGE));
        #[cfg(feature = "verilog")]
        add_lang!(Programming, &[".v", ".vh", ".sv", ".svh"], &[], &[], "Verilog", &["comment"],
            || Language::new(tree_sitter_verilog::LANGUAGE));
        // V: same .v extension as Verilog. Verilog has priority (registered first).
        // To count .v files as V, enable only the "v" feature without "verilog".
        #[cfg(feature = "v")]
        add_lang!(Programming, &[".v"], &[], &[], "V", &["comment"],
            || Language::new(tree_sitter_v::LANGUAGE));
        #[cfg(feature = "vhdl")]
        add_lang!(Programming, &[".vhdl", ".vhd"], &[], &[], "VHDL", &["comment"],
            || Language::new(tree_sitter_vhdl::LANGUAGE));
        #[cfg(feature = "zig")]
        add_lang!(Programming, &[".zig"], &[], &[], "Zig", &["comment"],
            || Language::new(tree_sitter_zig::LANGUAGE));
        #[cfg(feature = "zsh")]
        add_lang!(Programming, &[".zsh"], &["zsh"], &[], "Zsh", &["comment"],
            || Language::new(tree_sitter_zsh::LANGUAGE));

        // Machine languages (markup + config)
        #[cfg(feature = "css")]
        add_lang!(Machine, &[".css", ".scss", ".less"], &[], &[], "CSS", &["comment"],
            || Language::new(tree_sitter_css::LANGUAGE));
        #[cfg(feature = "hcl")]
        add_lang!(Machine, &[".tf", ".tfvars", ".hcl"], &[], &[], "HCL", &["comment"],
            || Language::new(tree_sitter_hcl::LANGUAGE));
        #[cfg(feature = "html")]
        add_lang!(Machine, &[".html", ".htm", ".xhtml"], &[], &[], "HTML", &["comment"],
            || Language::new(tree_sitter_html::LANGUAGE));
        #[cfg(feature = "json")]
        add_lang!(Machine, &[".json"], &[], &[], "JSON", &["comment"],
            || Language::new(tree_sitter_json::LANGUAGE));
        #[cfg(feature = "proto")]
        add_lang!(Machine, &[".proto"], &[], &[], "Protobuf", &["comment"],
            || Language::new(tree_sitter_proto::LANGUAGE));
        #[cfg(feature = "yaml")]
        add_lang!(Machine, &[".yaml", ".yml"], &[], &[], "YAML", &["comment"],
            || Language::new(tree_sitter_yaml::LANGUAGE));

        reg
    })
}

pub struct LanguageRegistry {
    languages: Vec<LanguageSpec>,
}

impl LanguageRegistry {
    pub fn detect_by_ext(&self, path: &Path) -> Option<&LanguageSpec> {
        let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        for lang in &self.languages {
            if lang.filenames.iter().any(|f| *f == fname) {
                return Some(lang);
            }
        }
        let ext = path.extension().and_then(|e| e.to_str())?;
        let dotted = format!(".{}", ext);
        self.languages.iter().find(|l| l.extensions.iter().any(|e| *e == dotted))
    }

    pub fn detect_by_shebang(&self, first_line: &[u8]) -> Option<&LanguageSpec> {
        if !first_line.starts_with(b"#!") {
            return None;
        }
        // Extract interpreter name from shebang: after "#!/" or "#!/usr/bin/env "
        let line_str = std::str::from_utf8(first_line).unwrap_or("");
        let interpreter = if let Some(env_pos) = line_str.find("/env ") {
            &line_str[env_pos + 5..]
        } else {
            // Strip "#!" and take the basename of the path
            let after_hash = line_str.trim_start_matches("#!");
            after_hash.rsplit('/').next().unwrap_or("")
        };
        // Match against full interpreter name (ignoring version suffixes like "python3")
        let interpreter = interpreter.trim();
        self.languages.iter().find(|l| l.shebangs.iter().any(|s| {
            interpreter == *s || interpreter.starts_with(s)
        }))
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
