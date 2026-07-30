use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Language;

type GrammarFn = fn() -> Language;

#[derive(Clone)]
pub struct LanguageSpec {
    pub name: &'static str,
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
            ($exts:expr, $shebangs:expr, $fnames:expr, $name:expr, $comments:expr, $fn:expr) => {{
                reg.languages.push(LanguageSpec {
                    name: $name,
                    extensions: $exts,
                    shebangs: $shebangs,
                    filenames: $fnames,
                    grammar_fn: $fn,
                    comment_kinds: $comments,
                });
            }};
        }

        // Default
        add_lang!(&[".rs"], &[], &[], "Rust", &["line_comment", "block_comment"],
            || Language::new(tree_sitter_rust::LANGUAGE));
        #[cfg(feature = "c")]
        add_lang!(&[".c"], &[], &[".h"], "C", &["comment"],
            || Language::new(tree_sitter_c::LANGUAGE));
        #[cfg(feature = "python")]
        add_lang!(&[".py", ".pyw"], &["python"], &[], "Python", &["comment"],
            || Language::new(tree_sitter_python::LANGUAGE));
        #[cfg(feature = "javascript")]
        add_lang!(&[".js", ".jsx", ".mjs", ".cjs"], &["node", "nodejs"], &[], "JavaScript", &["comment"],
            || Language::new(tree_sitter_javascript::LANGUAGE));
        #[cfg(feature = "bash")]
        add_lang!(&[".sh", ".bash"], &["sh", "bash", "dash", "zsh"], &[], "Bash", &["comment"],
            || Language::new(tree_sitter_bash::LANGUAGE));

        // Phase 2a
        #[cfg(feature = "haskell")]
        add_lang!(&[".hs", ".lhs"], &["runhaskell"], &[], "Haskell", &["comment"],
            || Language::new(tree_sitter_haskell::LANGUAGE));
        #[cfg(feature = "ocaml")]
        add_lang!(&[".ml"], &[], &[], "OCaml", &["comment"],
            || Language::new(tree_sitter_ocaml::LANGUAGE_OCAML));
        #[cfg(feature = "ocaml")]
        add_lang!(&[".mli"], &[], &[], "OCaml", &["comment"],
            || Language::new(tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE));
        #[cfg(feature = "elm")]
        add_lang!(&[".elm"], &[], &[], "Elm", &["comment"],
            || Language::new(tree_sitter_elm::LANGUAGE));
        #[cfg(feature = "go")]
        add_lang!(&[".go"], &[], &[], "Go", &["comment"],
            || Language::new(tree_sitter_go::LANGUAGE));
        #[cfg(feature = "typescript")]
        add_lang!(&[".ts"], &[], &[], "TypeScript", &["comment"],
            || Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT));
        #[cfg(feature = "typescript")]
        add_lang!(&[".tsx"], &[], &[], "TSX", &["comment"],
            || Language::new(tree_sitter_typescript::LANGUAGE_TSX));
        #[cfg(feature = "java")]
        add_lang!(&[".java"], &[], &[], "Java", &["comment"],
            || Language::new(tree_sitter_java::LANGUAGE));
        #[cfg(feature = "scala")]
        add_lang!(&[".scala", ".sc"], &[], &[], "Scala", &["comment"],
            || Language::new(tree_sitter_scala::LANGUAGE));

        // Standard LANGUAGE pattern
        macro_rules! lang_std {
            ($feat:expr, $exts:expr, $shebangs:expr, $fnames:expr, $name:expr, $crat:ident) => {
                #[cfg(feature = $feat)]
                add_lang!($exts, $shebangs, $fnames, $name, &["comment"],
                    || Language::new($crat::LANGUAGE));
            };
        }
        lang_std!("ada", &[".ada", ".ads", ".adb"], &[], &[], "Ada", tree_sitter_ada);
        lang_std!("agda", &[".agda", ".lagda"], &[], &[], "Agda", tree_sitter_agda);
        lang_std!("cpp", &[".cpp", ".cxx", ".cc", ".c++", ".hpp", ".hxx", ".hh", ".h++"], &[], &[], "C++", tree_sitter_cpp);
        lang_std!("csharp", &[".cs"], &[], &[], "C#", tree_sitter_c_sharp);
        lang_std!("css", &[".css", ".scss", ".less"], &[], &[], "CSS", tree_sitter_css);
        lang_std!("dart", &[".dart"], &[], &[], "Dart", tree_sitter_dart);
        lang_std!("elixir", &[".ex", ".exs"], &["elixir"], &[], "Elixir", tree_sitter_elixir);
        lang_std!("erlang", &[".erl", ".hrl"], &[], &[], "Erlang", tree_sitter_erlang);
        lang_std!("fortran", &[".f", ".f90", ".f95", ".f03", ".f08"], &[], &[], "Fortran", tree_sitter_fortran);
        lang_std!("gleam", &[".gleam"], &[], &[], "Gleam", tree_sitter_gleam);
        lang_std!("graphql", &[".graphql", ".gql"], &[], &[], "GraphQL", tree_sitter_graphql);
        lang_std!("groovy", &[".groovy", ".gvy", ".gy", ".gsh"], &[], &[], "Groovy", tree_sitter_groovy);
        lang_std!("hcl", &[".tf", ".tfvars", ".hcl"], &[], &[], "HCL", tree_sitter_hcl);
        lang_std!("html", &[".html", ".htm", ".xhtml"], &[], &[], "HTML", tree_sitter_html);
        lang_std!("json", &[".json"], &[], &[], "JSON", tree_sitter_json);
        lang_std!("julia", &[".jl"], &[], &[], "Julia", tree_sitter_julia);
        lang_std!("lua", &[".lua"], &["lua"], &[], "Lua", tree_sitter_lua);
        lang_std!("make", &[".mak", ".mk"], &[], &["Makefile", "makefile", "GNUmakefile"], "Make", tree_sitter_make);
        lang_std!("nix", &[".nix"], &[], &[], "Nix", tree_sitter_nix);
        lang_std!("odin", &[".odin"], &[], &[], "Odin", tree_sitter_odin);
        lang_std!("pascal", &[".pas"], &[], &[], "Pascal", tree_sitter_pascal);
        lang_std!("perl", &[".pl", ".pm", ".t"], &["perl"], &[], "Perl", tree_sitter_perl);
        lang_std!("powershell", &[".ps1", ".psm1", ".psd1"], &["pwsh"], &[], "PowerShell", tree_sitter_powershell);
        lang_std!("proto", &[".proto"], &[], &[], "Protobuf", tree_sitter_proto);
        lang_std!("r", &[".r", ".R", ".rmd"], &["R"], &[], "R", tree_sitter_r);
        lang_std!("ruby", &[".rb"], &["ruby"], &[], "Ruby", tree_sitter_ruby);
        lang_std!("scheme", &[".scm", ".ss"], &[], &[], "Scheme", tree_sitter_scheme);
        lang_std!("slint", &[".slint"], &[], &[], "Slint", tree_sitter_slint);
        lang_std!("solidity", &[".sol"], &[], &[], "Solidity", tree_sitter_solidity);
        lang_std!("swift", &[".swift"], &[], &[], "Swift", tree_sitter_swift);
        lang_std!("verilog", &[".sv", ".svh"], &[], &[".v", ".vh"], "Verilog", tree_sitter_verilog);
        lang_std!("vhdl", &[".vhdl", ".vhd"], &[], &[], "VHDL", tree_sitter_vhdl);
        lang_std!("yaml", &[".yaml", ".yml"], &[], &[], "YAML", tree_sitter_yaml);
        lang_std!("zig", &[".zig"], &[], &[], "Zig", tree_sitter_zig);
        lang_std!("zsh", &[".zsh"], &["zsh"], &[], "Zsh", tree_sitter_zsh);

        // Named LANGUAGE_* constants
        macro_rules! lang_named {
            ($feat:expr, $exts:expr, $shebangs:expr, $fnames:expr, $name:expr, $crat:ident, $const:ident) => {
                #[cfg(feature = $feat)]
                add_lang!($exts, $shebangs, $fnames, $name, &["comment"],
                    || Language::new($crat::$const));
            };
        }
        lang_named!("fsharp", &[".fs", ".fsx", ".fsi"], &[], &[], "F#", tree_sitter_fsharp, LANGUAGE_FSHARP);
        lang_named!("glsl", &[".vert", ".frag", ".geom", ".comp", ".tesc", ".tese", ".glsl"], &[], &[], "GLSL", tree_sitter_glsl, LANGUAGE_GLSL);
        lang_named!("php", &[".php", ".phtml", ".php3", ".php4", ".php5"], &[], &[], "PHP", tree_sitter_php, LANGUAGE_PHP);

        // V (only matches .v as exact filename, not extension, to avoid conflict with Verilog)
        #[cfg(feature = "v")]
        add_lang!(&[], &[], &[".v"], "V", &["comment"],
            || Language::new(tree_sitter_v::LANGUAGE));

        // language() → Language (direct function call)
        macro_rules! lang_direct {
            ($feat:expr, $exts:expr, $shebangs:expr, $fnames:expr, $name:expr, $crat:ident) => {
                #[cfg(feature = $feat)]
                add_lang!($exts, $shebangs, $fnames, $name, &["comment"],
                    $crat::language);
            };
        }
        // Fish: uses compatible tree-sitter version
        lang_direct!("fish", &[".fish"], &["fish"], &[], "Fish", tree_sitter_fish);

        // Blocked crates (depend on old tree-sitter versions, incompatible with v0.26.x):
        // dockerfile (v0.19), hare (v0.20), markdown (v0.19), prisma (?),
        // sql (?), toml (v0.20), vue (?), wgsl (?), kotlin (v0.20)

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
