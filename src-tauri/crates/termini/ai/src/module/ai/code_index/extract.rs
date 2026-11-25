use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use anyhow::{anyhow, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};
use crate::module::ai::code_index::walker::language_for_path;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub is_def: bool,
}

struct LangBundle {
    language: Language,
    query: Query,
}

fn rust_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_rust::LANGUAGE.into(), include_str!("queries/rust.scm")))
}

fn typescript_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), include_str!("queries/typescript.scm")))
}

fn tsx_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_typescript::LANGUAGE_TSX.into(), include_str!("queries/typescript.scm")))
}

fn javascript_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_javascript::LANGUAGE.into(), include_str!("queries/javascript.scm")))
}

fn python_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_python::LANGUAGE.into(), include_str!("queries/python.scm")))
}

fn go_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_go::LANGUAGE.into(), include_str!("queries/go.scm")))
}

fn bash_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_bash::LANGUAGE.into(), include_str!("queries/bash.scm")))
}

fn java_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_java::LANGUAGE.into(), include_str!("queries/java.scm")))
}

fn c_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_c::LANGUAGE.into(), include_str!("queries/c.scm")))
}

fn cpp_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_cpp::LANGUAGE.into(), include_str!("queries/cpp.scm")))
}

fn yaml_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_yaml::LANGUAGE.into(), include_str!("queries/yaml.scm")))
}

fn toml_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_toml_ng::LANGUAGE.into(), include_str!("queries/toml.scm")))
}

fn ruby_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_ruby::LANGUAGE.into(), include_str!("queries/ruby.scm")))
}

fn lua_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_lua::LANGUAGE.into(), include_str!("queries/lua.scm")))
}

fn php_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_php::LANGUAGE_PHP.into(), include_str!("queries/php.scm")))
}

fn powershell_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_powershell::LANGUAGE.into(), include_str!("queries/powershell.scm")))
}

fn json_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_json::LANGUAGE.into(), include_str!("queries/json.scm")))
}

fn hcl_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_hcl::LANGUAGE.into(), include_str!("queries/hcl.scm")))
}

fn make_bundle() -> &'static LangBundle {
    static CELL: OnceLock<LangBundle> = OnceLock::new();
    CELL.get_or_init(|| build_bundle(tree_sitter_make::LANGUAGE.into(), include_str!("queries/make.scm")))
}

fn build_bundle(language: Language, source: &str) -> LangBundle {
    let query = match Query::new(&language, source) {
        Ok(query) => query,
        Err(error) => {
            summer::tracing::warn!("[ai] code_index query compile failed: {}", error);
            Query::new(&language, "(ERROR) @def.error").expect("fallback query")
        }
    };
    LangBundle { language, query }
}

fn bundle_for_lang(lang: &str) -> Option<&'static LangBundle> {
    match lang {
        "rust" => Some(rust_bundle()),
        "typescript" => Some(typescript_bundle()),
        "tsx" => Some(tsx_bundle()),
        "javascript" => Some(javascript_bundle()),
        "python" => Some(python_bundle()),
        "go" => Some(go_bundle()),
        "bash" => Some(bash_bundle()),
        "java" => Some(java_bundle()),
        "c" => Some(c_bundle()),
        "cpp" => Some(cpp_bundle()),
        "yaml" => Some(yaml_bundle()),
        "toml" => Some(toml_bundle()),
        "ruby" => Some(ruby_bundle()),
        "lua" => Some(lua_bundle()),
        "php" => Some(php_bundle()),
        "powershell" => Some(powershell_bundle()),
        "json" => Some(json_bundle()),
        "hcl" => Some(hcl_bundle()),
        "make" => Some(make_bundle()),
        _ => None,
    }
}

pub fn extract_file(path: &Path, relative: &str) -> Result<Vec<Symbol>> {
    let Some(lang) = language_for_path(path) else {
        return Ok(Vec::new());
    };
    let Some(bundle) = bundle_for_lang(lang) else {
        return Ok(Vec::new());
    };
    let source = fs::read_to_string(path).map_err(|error| anyhow!("{}: {}", path.display(), error))?;
    let mut parser = Parser::new();
    parser.set_language(&bundle.language).map_err(|error| anyhow!(error.to_string()))?;
    let Some(tree) = parser.parse(&source, None) else {
        return Ok(Vec::new());
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&bundle.query, tree.root_node(), source.as_bytes());
    let mut symbols = Vec::new();
    while let Some(item) = matches.next() {
        for capture in item.captures {
            let capture_name = bundle.query.capture_names()[capture.index as usize];
            let Some((is_def, kind)) = parse_capture(capture_name) else {
                continue;
            };
            let node = capture.node;
            let Ok(name) = node.utf8_text(source.as_bytes()) else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            symbols.push(Symbol { path: relative.replace('\\', "/"), name: name.to_string(), kind: kind.to_string(), start_line: node.start_position().row + 1, is_def });
        }
    }
    Ok(symbols)
}

fn parse_capture(name: &str) -> Option<(bool, &str)> {
    if let Some(kind) = name.strip_prefix("def.") {
        return Some((true, kind));
    }
    if let Some(kind) = name.strip_prefix("ref.") {
        return Some((false, kind));
    }
    None
}
