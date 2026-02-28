// ASCII diagram parsers

use super::table::ParseError;
use super::*;
use std::borrow::Cow;

/// Parse any ASCII diagram
pub fn parse_ascii(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    if is_tree(input) {
        return parse_tree(input);
    }

    Err(ParseError::Other("Could not determine ASCII diagram type".to_string()))
}

fn is_tree(content: &str) -> bool {
    let tree_indicators = ['├', '└', '│', '─'];
    content.chars().any(|c| tree_indicators.contains(&c))
}

fn parse_tree(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return Err(ParseError::InvalidStructure);
    }

    let (root, _start_idx) = if !has_tree_chars(lines[0]) {
        let root_name = lines[0].trim().trim_end_matches('/');
        (Some(Cow::Borrowed(root_name)), 1)
    } else {
        (None, 0)
    };

    let children = Vec::new();

    Ok(ParsedStructure {
        structure_type: StructureType::AsciiTree,
        name: None,
        data: StructureData::Tree { root, children },
        original_len: input.len(),
    })
}

fn has_tree_chars(line: &str) -> bool {
    line.chars().any(|c| matches!(c, '├' | '└' | '│' | '─'))
}
