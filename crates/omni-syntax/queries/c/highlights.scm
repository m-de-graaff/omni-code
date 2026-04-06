; C highlights.scm

(identifier) @variable

; Literals
(string_literal) @string
(system_lib_string) @string
(char_literal) @string
(number_literal) @number
(true) @boolean
(false) @boolean
(null) @constant.builtin
(escape_sequence) @escape

; Comments
(comment) @comment

; Types
(type_identifier) @type
(primitive_type) @type.builtin
(sized_type_specifier) @type.builtin

; Functions
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))

; Parameters
(parameter_declaration declarator: (identifier) @variable.parameter)

; Properties
(field_identifier) @property
(field_expression field: (field_identifier) @property)

; Preprocessor
(preproc_include) @keyword.control
(preproc_def) @keyword.control
(preproc_ifdef) @keyword.control
(preproc_directive) @keyword.control

; Constants
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))
