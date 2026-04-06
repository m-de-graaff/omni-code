; Rust highlights.scm — compatible with tree-sitter-rust 0.24.x
; Uses only named node types to avoid anonymous node compatibility issues.
;
; In tree-sitter queries, later patterns take priority over earlier ones
; for the same node. So the catch-all goes FIRST, and specific patterns
; override it below.

; ── Catch-all for identifiers (lowest priority) ─────────────────────

(identifier) @variable

; ── Literals ─────────────────────────────────────────────────────────

(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(boolean_literal) @boolean

(integer_literal) @number
(float_literal) @number

(escape_sequence) @escape

; ── Comments ─────────────────────────────────────────────────────────

(line_comment) @comment
(block_comment) @comment

; ── Types ────────────────────────────────────────────────────────────

(type_identifier) @type
(primitive_type) @type.builtin

; ── Functions (higher priority than catch-all) ───────────────────────

(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))
(generic_function function: (identifier) @function)

(macro_invocation macro: (identifier) @function.macro)
(macro_definition name: (identifier) @function.macro)

; ── Variables and Parameters ─────────────────────────────────────────

(parameter pattern: (identifier) @variable.parameter)

; ── Fields and Properties ────────────────────────────────────────────

(field_identifier) @property
(shorthand_field_initializer (identifier) @property)

; ── Constants ────────────────────────────────────────────────────────

(const_item name: (identifier) @constant)
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))

; ── Attributes ───────────────────────────────────────────────────────

(attribute_item) @attribute
(inner_attribute_item) @attribute

; ── Labels / Lifetimes ───────────────────────────────────────────────

(lifetime (identifier) @label)

; ── Modules / Namespaces ─────────────────────────────────────────────

(mod_item name: (identifier) @namespace)
(scoped_identifier path: (identifier) @namespace)
