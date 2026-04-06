; Bash highlights.scm

(word) @variable

; Literals
(string) @string
(raw_string) @string
(heredoc_body) @string
(number) @number

; Comments
(comment) @comment

; Commands
(command_name) @function
(function_definition name: (word) @function)

; Variables
(variable_name) @variable
(special_variable_name) @variable.builtin
(simple_expansion) @variable
(expansion) @variable

; Properties / Options
(command argument: (word) @variable)

; Builtins
((command_name) @function.builtin
  (#match? @function.builtin "^(echo|cd|exit|return|source|export|unset|eval|exec|set|shift|test|read|printf|local|declare|readonly|typeset|trap|wait|kill|jobs|fg|bg|pushd|popd|dirs)$"))
