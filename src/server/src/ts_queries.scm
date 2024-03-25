;; Find the internal name for the telemetry logger and save it inside @assignementName 
(constructor_declaration
  parameters: 
    (parameter_list
      (parameter
        type: 
          (generic_name
            name: (identifier) @typeName)
        name: (identifier) @varLoggerName
      )
    )
  body: 
    (block 
      (expression_statement 
        (assignment_expression
          left: (identifier) @assignementName
          right: (identifier) @varName) 
      )
    )
  (#eq? @typeName "IName") ;; the interface name
  (#eq? @varLoggerName @varName)
)

;; Find all invocations TODO programmactly match internalName
(invocation_expression
  function: 
    (member_access_expression
      expression: (identifier) @internalName
      name: (identifier) @methodName
    )
  arguments: 
    (argument_list 
      (argument) @string)
  (#eq? @internalName "TODO") ;; the assignementName found in the first query
)
