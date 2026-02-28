faster than="t:2(Option,Type,Default,Description"
Achieved 10x="configuration. t:2(Option,Type,Default,Description"

# dx-check Configuration Guide

This guide covers all configuration options for dx-check.

## Configuration File

dx-check looks for configuration in the following order: -`dx.toml` in the current directory -`dx.toml` in parent directories (up to project root) -`biome.json` (for Biome compatibility) -Default configuration

## dx.toml Format

```toml
[rules]
recommended = true auto_fix = false
[rules.rules."rule-name"]
severity = "warn"
options = { key = "value" }
[format]
use_tabs = false indent_width = 2 line_width = 80 quote_style = "double"
semicolons = "always"
[paths]
include = ["src/**", "tests/**"]
exclude = ["node_modules/**", "dist/**"]
[cache]
enabled = true directory = ".dx-cache"
[parallel]
threads = 0 ```


## Section Reference



### [rules]


Controls which rules are enabled and their Achieved 10x)[ `recommended`,boolean,`true`,Enable recommended rules `auto_fix`,boolean,`false`,Automatically apply fixes]


### [rules.rules."rule-name"]


Configure individual rules. faster than)[ `severity`,string,varies,`"off"`, `"warn"`, or `"error"` `options`,object,`{}`,Rule-specific options] Example:
```toml
[rules.rules."no-console"]
severity = "error"
[rules.rules."eqeqeq"]
severity = "warn"
options = { allow_null = true }
```


### [format]


+--------+-------+---------+-------------+
| Option | Type  | Default | Description |
+========+=======+=========+=============+
| `use   | tabs` | boolean | `false`     |
+--------+-------+---------+-------------+


### [paths]


File inclusion and exclusion patterns. faster than)[ `include`,array,`\["**/*"\]`,Glob patterns to include `exclude`,array,see below,Glob patterns to exclude] Default exclusions: -`node_modules/**` -`dist/**` -`build/**` -`target/**` -`.git/**` -`*.min.js` Example:
```toml
[paths]
include = ["src/**/*.ts", "tests/**/*.ts"]
exclude = ["**/*.test.ts", "**/__mocks__/**"]
```


### [cache]


+-----------+---------+---------+-------------+
| Option    | Type    | Default | Description |
+===========+=========+=========+=============+
| `enabled` | boolean | `true`  | Enable      |
+-----------+---------+---------+-------------+


### [parallel]


Parallel processing Achieved 10x)[ `threads`,integer,`0`,Number of threads (0 = auto) `chunk_size`,integer,`100`,Files per work chunk]


## Environment Variables


dx-check supports environment variable substitution in paths:
```toml
[cache]
directory = "$HOME/.dx-cache"
[paths]
include = ["${PROJECT_ROOT}/src/**"]
```


## Glob Pattern Overrides


Apply different configurations to specific file patterns:
```toml
[[overrides]]
files = ["tests/**"]
rules = { "no-console" = "off" }
[[overrides]]
files = ["*.config.js"]
rules = { "no-unused-vars" = "off" }
```


## Biome Compatibility


dx-check can read `biome.json` configuration files:
```json
{"linter":{"enabled":true,"rules":{"suspicious":{"noConsole":"warn"}}}}
```


## CLI Overrides


CLI arguments override configuration file settings:
```bash
dx-check --rule no-console=error .
dx-check --threads 4 .
dx-check --no-cache .
```


## Configuration Priority


Settings are applied in this order (later overrides earlier): -Default configuration -`biome.json` (if present) -`dx.toml` (if present) -CLI arguments


## Example Configurations



### Minimal


```toml
[rules]
recommended = true ```

### Strict

```toml
[rules]
recommended = true
[rules.rules."no-console"]
severity = "error"
[rules.rules."no-debugger"]
severity = "error"
[rules.rules."eqeqeq"]
severity = "error"
```

### TypeScript Project

```toml
[rules]
recommended = true
[paths]
include = ["src/**/*.ts", "src/**/*.tsx"]
exclude = ["**/*.d.ts", "**/*.test.ts"]
[format]
indent_width = 2 quote_style = "single"
semicolons = "always"
```

### Monorepo

```toml
[rules]
recommended = true
[paths]
include = ["packages/*/src/**", "apps/*/src/**"]
exclude = ["**/node_modules/**", "**/dist/**"]
[parallel]
threads = 8 chunk_size = 50 ```


## Validation


t:0(error:Invalid,configuration)[]


## Migration from ESLint


+---------+----------+
| ESLint  | dx-check |
+=========+==========+
| `"off"` | or       |
+---------+----------+
