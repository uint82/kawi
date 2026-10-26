(import_statement "import" @keyword.import)
(import_statement "from" @keyword.import)
(export_statement "export" @keyword.import)
(return_statement "return" @keyword.return)
"function" @keyword.function
(shorthand_property_identifier) @variable.member
(export_statement "from" @keyword.import)
