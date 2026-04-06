//! Extract document symbols from a tree-sitter parse tree.

use omni_core::Text;

/// The kind of a document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
    Type,
    Module,
    Macro,
    Other,
}

impl SymbolKind {
    /// Display icon for the symbol kind.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Function => "\u{f0295}", // 󰊕
            Self::Struct => "\u{f0317}",   // 󰌗
            Self::Enum => "\u{f0702}",     // 󰜂
            Self::Impl => "\u{f061f}",     // 󰘟
            Self::Const | Self::Static => "\u{f0627}", // 󰘧
            Self::Trait | Self::Type | Self::Other => "\u{f06e4}", // 󰛤
            Self::Module => "\u{f07b}",    //
            Self::Macro => "\u{f04a4}",    // 󰒤
        }
    }
}

/// A symbol found in a document.
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    /// Symbol name.
    pub name: String,
    /// Kind of symbol.
    pub kind: SymbolKind,
    /// Line number (0-based).
    pub line: usize,
    /// Byte offset in the document.
    pub start_byte: usize,
}

/// Extract document symbols from a tree-sitter tree.
///
/// Walks the AST looking for named definitions (functions, structs, enums, etc.)
/// and extracts their names and positions.
#[must_use]
pub fn extract_symbols(tree: &tree_sitter::Tree, text: &Text) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    collect_symbols(root, text, &mut symbols);
    symbols.sort_by_key(|s| s.line);
    symbols
}

fn collect_symbols(
    node: tree_sitter::Node<'_>,
    text: &Text,
    symbols: &mut Vec<DocumentSymbol>,
) {
    let kind_str = node.kind();
    let symbol_kind = match kind_str {
        "function_item" | "function_definition" => Some(SymbolKind::Function),
        "struct_item" => Some(SymbolKind::Struct),
        "enum_item" => Some(SymbolKind::Enum),
        "trait_item" => Some(SymbolKind::Trait),
        "impl_item" => Some(SymbolKind::Impl),
        "const_item" => Some(SymbolKind::Const),
        "static_item" => Some(SymbolKind::Static),
        "type_item" => Some(SymbolKind::Type),
        "mod_item" => Some(SymbolKind::Module),
        "macro_definition" => Some(SymbolKind::Macro),
        _ => None,
    };

    if let Some(kind) = symbol_kind {
        // Look for a `name` child node
        if let Some(name_node) = node.child_by_field_name("name") {
            let start = name_node.start_byte();
            let end = name_node.end_byte();
            let name = extract_text(text, start, end);
            if !name.is_empty() {
                symbols.push(DocumentSymbol {
                    name,
                    kind,
                    line: node.start_position().row,
                    start_byte: node.start_byte(),
                });
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_symbols(child, text, symbols);
    }
}

/// Extract text from a rope by byte range.
fn extract_text(text: &Text, start_byte: usize, end_byte: usize) -> String {
    if start_byte >= text.len_bytes() || end_byte > text.len_bytes() {
        return String::new();
    }
    let start_char = text.byte_to_char(start_byte);
    let end_char = text.byte_to_char(end_byte);
    text.slice(start_char..end_char).chars().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "lang-rust")]
    #[test]
    fn extract_rust_symbols() {
        let code = r#"
fn main() {}
struct Foo { x: i32 }
enum Bar { A, B }
const MAX: usize = 100;
mod utils {}
"#;
        let text = Text::from(code);
        let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(code.as_bytes(), None).unwrap();

        let symbols = extract_symbols(&tree, &text);

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"), "should find main fn: {names:?}");
        assert!(names.contains(&"Foo"), "should find Foo struct: {names:?}");
        assert!(names.contains(&"Bar"), "should find Bar enum: {names:?}");
    }
}
