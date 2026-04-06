; Python highlights.scm

(identifier) @variable

; Literals
(string) @string
(concatenated_string) @string
(integer) @number
(float) @number
(true) @boolean
(false) @boolean
(none) @constant.builtin
(escape_sequence) @escape

; Comments
(comment) @comment

; Types
(type (identifier) @type)
(class_definition name: (identifier) @type)

; Functions
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(call function: (attribute attribute: (identifier) @function.method))
(decorator) @attribute

; Parameters
(parameters (identifier) @variable.parameter)
(default_parameter name: (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)
(keyword_argument name: (identifier) @variable.parameter)

; Properties
(attribute attribute: (identifier) @property)

; Builtins
((identifier) @variable.builtin
  (#match? @variable.builtin "^(self|cls)$"))
((identifier) @function.builtin
  (#match? @function.builtin "^(print|len|range|type|int|str|float|list|dict|set|tuple|bool|isinstance|hasattr|getattr|setattr|super|property|staticmethod|classmethod|enumerate|zip|map|filter|sorted|reversed|min|max|sum|abs|round|open|input)$"))
