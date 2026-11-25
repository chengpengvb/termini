(function_definition (function_declarator (identifier) @def.function))
(function_definition (function_declarator (qualified_identifier (identifier) @def.function)))
(class_specifier (type_identifier) @def.class)
(struct_specifier (type_identifier) @def.struct)
(enum_specifier (type_identifier) @def.enum)
(call_expression (identifier) @ref.call)
(type_identifier) @ref.type
