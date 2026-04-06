//! Syntax highlight scopes and spans.

/// The semantic category of a syntax highlight.
///
/// These map to tree-sitter capture names (e.g., `@keyword.function` →
/// `KeywordFunction`). The theme system maps scopes to colors/styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightScope {
    // Keywords
    Keyword,
    KeywordFunction,
    KeywordReturn,
    KeywordOperator,
    KeywordControl,

    // Functions
    Function,
    FunctionMethod,
    FunctionMacro,
    FunctionBuiltin,

    // Types
    Type,
    TypeBuiltin,

    // Variables
    Variable,
    VariableBuiltin,
    VariableParameter,

    // Strings
    String,
    StringSpecial,

    // Numbers
    Number,
    Boolean,

    // Comments
    Comment,
    CommentDoc,

    // Operators & Punctuation
    Operator,
    Punctuation,
    PunctuationBracket,
    PunctuationDelimiter,

    // Constants
    Constant,
    ConstantBuiltin,

    // Properties
    Property,

    // Namespaces
    Namespace,

    // Labels / Lifetimes
    Label,

    // Attributes
    Attribute,

    // Tags (HTML/XML)
    Tag,

    // Escape sequences
    Escape,
}

impl HighlightScope {
    /// Map a tree-sitter capture name (e.g., `"keyword.function"`) to a scope.
    ///
    /// Handles dot-separated hierarchical names by trying the full name first,
    /// then falling back to the prefix. For example, `"keyword.function"` matches
    /// `KeywordFunction`; `"keyword.unknown"` falls back to `Keyword`.
    #[must_use]
    pub fn from_capture(name: &str) -> Option<Self> {
        // Try exact match first
        if let Some(scope) = Self::exact_match(name) {
            return Some(scope);
        }
        // Fall back to prefix (e.g., "keyword.whatever" → Keyword)
        if let Some(dot) = name.find('.') {
            return Self::exact_match(&name[..dot]);
        }
        None
    }

    fn exact_match(name: &str) -> Option<Self> {
        match name {
            "keyword" => Some(Self::Keyword),
            "keyword.function" => Some(Self::KeywordFunction),
            "keyword.return" => Some(Self::KeywordReturn),
            "keyword.operator" => Some(Self::KeywordOperator),
            "keyword.control" | "keyword.control.flow" | "keyword.control.repeat"
            | "keyword.control.conditional" | "keyword.control.import" => {
                Some(Self::KeywordControl)
            }

            "function" | "function.call" => Some(Self::Function),
            "function.method" | "function.method.call" => Some(Self::FunctionMethod),
            "function.macro" => Some(Self::FunctionMacro),
            "function.builtin" => Some(Self::FunctionBuiltin),

            "type" => Some(Self::Type),
            "type.builtin" => Some(Self::TypeBuiltin),

            "variable" => Some(Self::Variable),
            "variable.builtin" => Some(Self::VariableBuiltin),
            "variable.parameter" => Some(Self::VariableParameter),

            "string" => Some(Self::String),
            "string.special" | "string.special.url" | "string.special.path"
            | "string.special.symbol" | "string.regex" => Some(Self::StringSpecial),

            "number" | "number.integer" | "number.float" => Some(Self::Number),
            "boolean" | "constant.builtin.boolean" => Some(Self::Boolean),

            "comment" => Some(Self::Comment),
            "comment.doc" | "comment.documentation" => Some(Self::CommentDoc),

            "operator" => Some(Self::Operator),
            "punctuation" => Some(Self::Punctuation),
            "punctuation.bracket" => Some(Self::PunctuationBracket),
            "punctuation.delimiter" | "punctuation.special" => Some(Self::PunctuationDelimiter),

            "constant" => Some(Self::Constant),
            "constant.builtin" | "constant.builtin.character" => Some(Self::ConstantBuiltin),

            "property" | "field" | "variable.other.member" => Some(Self::Property),

            "namespace" | "module" => Some(Self::Namespace),

            "label" => Some(Self::Label),
            "attribute" => Some(Self::Attribute),
            "tag" => Some(Self::Tag),

            "escape" | "string.escape" | "constant.character.escape" => Some(Self::Escape),

            _ => None,
        }
    }
}

/// A highlighted span in the document, identified by byte range and scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Start byte offset in the document.
    pub start_byte: usize,
    /// End byte offset in the document (exclusive).
    pub end_byte: usize,
    /// The highlight scope/category.
    pub scope: HighlightScope,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_capture_exact() {
        assert_eq!(HighlightScope::from_capture("keyword"), Some(HighlightScope::Keyword));
        assert_eq!(
            HighlightScope::from_capture("keyword.function"),
            Some(HighlightScope::KeywordFunction),
        );
        assert_eq!(
            HighlightScope::from_capture("function.macro"),
            Some(HighlightScope::FunctionMacro),
        );
    }

    #[test]
    fn from_capture_prefix_fallback() {
        // "keyword.unknown" falls back to Keyword
        assert_eq!(
            HighlightScope::from_capture("keyword.unknown"),
            Some(HighlightScope::Keyword),
        );
    }

    #[test]
    fn from_capture_unknown() {
        assert_eq!(HighlightScope::from_capture("totally_unknown"), None);
    }

    #[test]
    fn from_capture_nested() {
        assert_eq!(
            HighlightScope::from_capture("comment.doc"),
            Some(HighlightScope::CommentDoc),
        );
        assert_eq!(
            HighlightScope::from_capture("type.builtin"),
            Some(HighlightScope::TypeBuiltin),
        );
    }
}
