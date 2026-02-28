
# DX Serializer Syntax Reference

This document describes the two official formats for DX Serializer.

## LLM Format (Dx Serializer)

The LLM format is a compact, token-efficient format stored on disk.

### Objects

```dsr
config(host=localhost port=5432 debug=true)
server(url="https://api.example.com" timeout=30)
```
- Name followed by parentheses containing space-separated key=value pairs
- No spaces around `=`
- Use quotes for multi-word strings

### Arrays

```dsr
tags=[rust performance serialization]
editors=[neovim zed "firebase studio"]
```
- Square brackets with space-separated items
- Use quotes for multi-word items

### Tables

```dsr
users[id name email](
1 Alice alice@ex.com
2 Bob bob@ex.com
)
```
- Headers in square brackets, rows in parentheses
- Each row on its own line

### Strings with Spaces

Use double quotes:
```dsr
config[title="Enhanced Developing Experience",desc="My description"]
```

### Nested Sections

Use dot notation in the section name:
```dsr
js.dependencies[react=19.0.1,next=16.0.1]
i18n.locales[path=@/locales,default=en-US]
```

### Complete Example

```dsr
name=dx
version=0.0.1
title="Enhanced Developing Experience"
workspace(paths=[@/www @/backend])
editors(items=[neovim zed vscode] default=neovim)
forge(repository="https://github.com/user/repo" tools=[cli docs tests])
js.dependencies(react=19.0.1 next=16.0.1)
```

## Human Format

The Human format is designed for readability in text editors.

### Scalars

```dx
key = value ```
- Key followed by `=` and value
- Keys are padded with spaces for alignment
- Strings with spaces use quotes: `title = "My Title"`


### Arrays


```dx
key:
- item1
- item2
- item3
```
- Key followed by `:` on its own line
- Each item on a new line prefixed with `- `


### Sections


```dx
[section]
key = value
[section.subsection]
key = value ```
- Section headers in brackets
- Nested sections use dot notation

### Complete Example

```dx
name = dx version = 0.0.1 title = "Enhanced Developing Experience"
[workspace]
paths:
- @/www
- @/backend
[editors]
items:
- neovim
- zed
- vscode
default = neovim
[forge]
repository = https://github.com/user/repo tools:
- cli
- docs
- tests
[js.dependencies]
react = 19.0.1 next = 16.0.1 ```


## Conversion Rules

### LLM → Human

- Objects `name(key=val)` become `[name]` sections with key-value pairs
- Arrays `key=[item1 item2]` become `key:` followed by `- item` lines
- Keys are padded for alignment

### Human → LLM

- `[section]` headers with key-value pairs become `section(key=val)`
- `key:` followed by `- item` lines become `key=[item1 item2]`
- All whitespace padding is removed

## Grammar (EBNF)

### LLM Format

```ebnf
document = (scalar | object | array | table)* ;
scalar = key "=" value ;
object = identifier "(" pairs ")" ;
array = key "=" "[" items "]" ;
table = identifier "[" headers "]" "(" rows ")" ;
pairs = pair (" " pair)* ;
pair = key "=" value ;
headers = identifier (" " identifier)* ;
rows = row* ;
row = value (" " value)* ;
items = value (" " value)* ;
key = identifier ;
value = string | identifier | number ;
string = '"' [^"]* '"' ;
identifier = [a-zA-Z_][a-zA-Z0-9_.-]* ;
number = [0-9]+ ("." [0-9]+)? ;
```


### Human Format


```ebnf
document = (root_pair | section)* ;
root_pair = key "=" value | key ":" array_items ;
section = "[" identifier "]" section_content ;
section_content = (pair | array_def)* ;
pair = key "=" value ;
array_def = key ":" array_items ;
array_items = ("- " value)+ ;
key = identifier ;
value = string | identifier ;
string = '"' [^"]* '"' ;
identifier = [a-zA-Z_][a-zA-Z0-9_.-]* ;
```
