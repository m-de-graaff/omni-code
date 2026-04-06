; Go highlights.scm

(identifier) @variable

; Literals
(interpreted_string_literal) @string
(raw_string_literal) @string
(rune_literal) @string
(int_literal) @number
(float_literal) @number
(imaginary_literal) @number
(true) @boolean
(false) @boolean
(nil) @constant.builtin
(escape_sequence) @escape

; Comments
(comment) @comment

; Types
(type_identifier) @type
(struct_type) @type
(interface_type) @type

; Functions
(function_declaration name: (identifier) @function)
(method_declaration name: (field_identifier) @function.method)
(call_expression function: (identifier) @function)
(call_expression function: (selector_expression field: (field_identifier) @function.method))

; Parameters
(parameter_declaration name: (identifier) @variable.parameter)

; Properties / Fields
(field_identifier) @property

; Packages
(package_identifier) @namespace
(package_clause (package_identifier) @namespace)
(import_spec path: (interpreted_string_literal) @string)

; Builtins
((identifier) @function.builtin
  (#match? @function.builtin "^(make|len|cap|new|append|copy|delete|close|panic|recover|print|println|complex|real|imag)$"))
