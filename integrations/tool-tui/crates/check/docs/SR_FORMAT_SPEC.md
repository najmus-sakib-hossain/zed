
#.sr File Format Specification Source Rule (SR) Definition Format Version: 1.0 Status: Draft Date: January 20, 2025

## Overview

`.sr` (Source Rule) files are simple, declarative rule definition files designed for contributors with minimal coding experience. They define linting and formatting rules using pattern matching syntax and auto-fix definitions. `.sr` files compile to `.sr` format, which then compiles to binary `.dxm` files for runtime execution.

## Design Goals

- Simple Syntax: Easy to read and write without deep programming knowledge
- Pattern Matching: Declarative pattern syntax for matching code structures
- Auto-Fix Support: Built-in fix definition syntax
- Type Safety: Validated before compilation
- Composable: Rules can be built from smaller patterns

## File Naming Convention

```
<language>-<rule-name>.sr ```
Examples: -`js-no-console.sr` - JavaScript no-console rule -`py-no-print.sr` - Python no-print rule -`rs-unwrap-used.sr` - Rust unwrap-used rule


## File Structure


```sr
rule <rule_name> { language: <language_code> category: <category> severity: <warn|error|off> fixable: <true|false> recommended: <true|false> description: | <multi-line description>
docs_url: <optional_url> }
pattern { match: <pattern_expression> where: <optional_conditions> message: <diagnostic_message> suggestion: <optional_suggestion> }
fix { replace: <replacement_expression> position: <full|node|range> preserve: <whitespace|comments|both|none> }
examples { correct: | <example_code> incorrect: | <example_code> }
```


## Field Specifications



### Rule Section


+------------+--------+----------+-------------+
| Field      | Type   | Required | Description |
+============+========+==========+=============+
| `language` | string | ✅        | Language    |
+------------+--------+----------+-------------+


### Pattern Section


+---------+---------+----------+-------------+
| Field   | Type    | Required | Description |
+=========+=========+==========+=============+
| `match` | pattern | ✅        | Pattern     |
+---------+---------+----------+-------------+


### Fix Section


+-----------+------------+----------+-------------+
| Field     | Type       | Required | Description |
+===========+============+==========+=============+
| `replace` | expression | ✅        | Replacement |
+-----------+------------+----------+-------------+


### Examples Section


+-----------+------+----------+-------------+
| Field     | Type | Required | Description |
+===========+======+==========+=============+
| `correct` | code | ❌        | Example     |
+-----------+------+----------+-------------+


## Pattern Matching Syntax



### Basic Patterns


Pattern expressions use a simple, declarative syntax:


#### 1. Identifier Matching


```sr
$name console console.log ```

#### 2. Expression Matching

```sr
$expr $expr:CallExpression $expr:BinaryExpression $expr:MemberExpression ```


#### 3. Statement Matching


```sr
$stmt $stmt:IfStatement $stmt:ForStatement $stmt:WhileStatement ```

#### 4. Wildcard Matching

```sr
_ ...
..+ ```


#### 5. Literal Matching


```sr
"string"
42 true false null ```

### Composite Patterns

#### Function Calls

```sr
$func($args...)
console.log($args...)
$obj.$method($args...)
$func($arg1, $arg2)
```

#### Variable Declarations

```sr
let $name = $init const $name = $init var $name = $init let $name ```


#### Binary Operations


```sr
$left $op $right $left == $right $left === $right $left + $right ```

#### Member Access

```sr
$obj.$prop $obj[$prop]
$obj.$prop1.$prop2 ```


#### Control Flow


```sr
if ($cond) { $body... }
while ($cond) { $body... }
for ($init; $cond; $update) { $body... }
```


### Conditions (where clause)


The `where` clause adds additional constraints:
```sr
where: $expr is CallExpression where: $name == "console"
where: $expr is not modified where: $name == "console" and $method == "log"
where: $severity == "warn" or $severity == "error"
where: $var is not declared where: $var is not used where: $var is not reassigned ```

## Fix Definition Syntax

### Replacement Expressions

Fix expressions use captured variables from patterns:
```sr
replace: ""
replace: "logger.log"
replace: "const $name = $init"
replace: "$name.toString()"
replace: | if ($cond) { $body...
}
```

### Position Modes

```sr
position: full position: node($expr)
position: range($start, $end)
```

### Preservation Options

```sr
preserve: both preserve: whitespace preserve: comments preserve: none ```


## Complete Examples



### Example 1: no-console (JavaScript)


```sr
rule no-console { language: js category: suspicious severity: warn fixable: true recommended: true description: |
Disallow the use of console statements.
Console statements are often used for debugging and should be removed before production deployment.
docs_url: https:
}
pattern { match: console.$method($args...)
where: $method in ["log", "error", "warn", "info", "debug"]
message: "Unexpected console statement"
suggestion: "Remove or replace with proper logging"
}
fix { replace: ""
position: full preserve: comments }
examples { incorrect: | console.log('debug');
console.error('error');
correct: | logger.info('production log');
}
```


### Example 2: prefer-const (JavaScript)


```sr
rule prefer-const { language: js category: style severity: warn fixable: true recommended: true description: |
Require const declarations for variables that are never reassigned.
Using const makes code more predictable and easier to understand.
docs_url: https:
}
pattern { match: let $name = $init where: $name is not reassigned message: "Use const instead of let for variable '$name'"
suggestion: "Change let to const"
}
fix { replace: "const $name = $init"
position: full preserve: both }
examples { incorrect: | let x = 1;
console.log(x);
correct: | const x = 1;
console.log(x);
}
```


### Example 3: no-unwrap (Rust)


```sr
rule no-unwrap { language: rs category: correctness severity: warn fixable: false recommended: true description: |
Disallow the use of .unwrap().
Use proper error handling with Result<T, E> and ? operator instead.
docs_url: https:
}
pattern { match: $expr.unwrap()
message: "Avoid using .unwrap(), use proper error handling"
suggestion: "Use the ? operator or match expression"
}
examples { incorrect: | let value = option.unwrap();
correct: | let value = option?;
let value = match option { Some(v) => v, None => return Err(Error::Missing), };
}
```


### Example 4: eqeqeq (JavaScript)


```sr
rule eqeqeq { language: js category: suspicious severity: warn fixable: true recommended: true description: | Require the use of === and !== instead of == and !=.
Strict equality operators prevent type coercion bugs.
docs_url: https:
}
pattern { match: $left == $right message: "Use '===' instead of '=='"
suggestion: "Replace with strict equality"
}
fix { replace: "$left === $right"
position: full preserve: both }
pattern { match: $left != $right message: "Use '!==' instead of '!='"
suggestion: "Replace with strict inequality"
}
fix { replace: "$left !== $right"
position: full preserve: both }
examples { incorrect: | if (x == null) {}
if (x != 0) {}
correct: | if (x === null) {}
if (x !== 0) {}
}
```


### Example 5: no-print (Python)


```sr
rule no-print { language: py category: suspicious severity: warn fixable: false recommended: false description: |
Disallow the use of print() statements.
Use proper logging instead of print for production code.
docs_url: https:
}
pattern { match: print($args...)
message: "Avoid using print(), use logging instead"
suggestion: "Replace with logger.info() or logger.debug()"
}
examples { incorrect: | print("debug message")
print(f"value: {x}")
correct: | logger.info("debug message")
logger.debug(f"value: {x}")
}
```


## Multiple Patterns


A rule can have multiple patterns:
```sr
rule no-equality-null { language: js category: suspicious severity: warn fixable: true recommended: true description: "Use strict equality with null"
}
pattern { match: $expr == null message: "Use '=== null' instead of '== null'"
}
fix { replace: "$expr === null"
position: full }
pattern { match: $expr != null message: "Use '!== null' instead of '!= null'"
}
fix { replace: "$expr !== null"
position: full }
```


## Pattern Composition


Rules can import and compose patterns:
```sr
@pattern console_method { match: console.$method($args...)
where: $method in ["log", "error", "warn", "info", "debug"]
}
rule no-console { language: js category: suspicious severity: warn fixable: true recommended: true description: "Disallow console statements"
}
pattern { use: console_method message: "Unexpected console statement"
}
fix { replace: ""
position: full }
```


## Parsing Rules


- Comments: Lines starting with `#` are comments (ignored)
- Blocks: Use `{ }` for block definitions
- Multi-line Values: Use `|` after colon, indent content
- Strings: Use quotes for string literals
- Booleans: `true`, `false` (lowercase)
- Enums: Use exact enum variant names (case-sensitive)
- Lists: Use `[item1, item2,...]` syntax
- Variables: Use `$name` for pattern variables
- Wildcards: Use `_` for single, `...` for zero-or-more, `..+` for one-or-more


## Validation Rules


- All required fields must be present
- Rule names must be unique within a language
- Pattern expressions must be valid syntax
- Fix expressions must reference only captured variables
- Conditions must use valid operators
- Categories and severities must be valid enum variants
- Language codes must be supported


## Compilation Process


```
.sr files Parser Validator Pattern Compiler .sr format .dxm binary ```
- Parse: Read.sr files and parse into RuleDefinition structs
- Validate: Check all rules meet specification
- Compile Patterns: Convert pattern expressions to AST matchers
- Generate.sr: Convert to.sr format
- Compile Binary: Serialize to.dxm format
- Verify: Validate binary format integrity

## Advanced Features

### Conditional Fixes

```sr
fix { replace: "const $name = $init"
position: full when: $init is not undefined }
fix { replace: "const $name"
position: full when: $init is undefined }
```

### Multiple Fix Options

```sr
fix { option: "Remove console statement"
replace: ""
position: full }
fix { option: "Replace with logger"
replace: "logger.$method($args...)"
position: full }
```

### Scoped Patterns

```sr
pattern { match: $var where: $var is declared in function scope: function message: "Variable is function-scoped"
}
```

## Benefits

- Contributor Friendly: No Rust knowledge required
- Declarative: Focus on what to match, not how
- Type Safe: Validated before compilation
- Composable: Reuse patterns across rules
- Maintainable: Clear, readable syntax
- Fast: Compiles to optimized binary format

## Migration from.sr

Existing.sr files can be converted to.sr format:
```bash
dx-check rule convert --input rules/js-rules.sr --output rules/sr/ ```


## File Organization


@tree:rules[]


## Hot Reload


The system watches for changes to.sr files:
```bash
dx-check watch --rules-dir rules/sr ```
Status: Specification complete, ready for implementation.
