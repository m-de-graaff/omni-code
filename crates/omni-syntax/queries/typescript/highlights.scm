; TypeScript highlights.scm

(identifier) @variable

; Literals
(string) @string
(template_string) @string
(template_substitution) @string.special
(number) @number
(true) @boolean
(false) @boolean
(null) @constant.builtin
(undefined) @constant.builtin
(escape_sequence) @escape

; Comments
(comment) @comment

; Types
(type_identifier) @type
(predefined_type) @type.builtin
(type_parameter (type_identifier) @type)
(interface_declaration name: (type_identifier) @type)
(type_alias_declaration name: (type_identifier) @type)
(enum_declaration name: (identifier) @type)

; Functions
(function_declaration name: (identifier) @function)
(method_definition name: (property_identifier) @function.method)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function.method))
(arrow_function)

; Parameters
(required_parameter (identifier) @variable.parameter)
(optional_parameter (identifier) @variable.parameter)

; Properties
(property_identifier) @property
(shorthand_property_identifier) @property

; Variables
((identifier) @variable.builtin
  (#match? @variable.builtin "^(this|arguments|console|window|document|module|exports|require|process)$"))

; Constants
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))
