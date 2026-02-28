// Mermaid diagram parsers

use super::table::ParseError;
use super::*;

/// Parse any Mermaid diagram
pub fn parse_mermaid(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    let first_line =
        input.lines().next().ok_or(ParseError::InvalidStructure)?.trim().to_lowercase();

    let first_word = first_line.split_whitespace().next().ok_or(ParseError::InvalidStructure)?;

    match first_word {
        "graph" | "flowchart" => parse_flowchart(input),
        "sequencediagram" => parse_sequence(input),
        "pie" => parse_pie(input),
        _ => Err(ParseError::Other(format!("Unsupported Mermaid type: {}", first_word))),
    }
}

fn parse_flowchart(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    let mut lines = input.lines().peekable();

    let first_line = lines.next().ok_or(ParseError::InvalidStructure)?;
    let direction = parse_direction(first_line)?;

    let nodes = Vec::new();
    let edges = Vec::new();

    Ok(ParsedStructure {
        structure_type: StructureType::MermaidFlowchart,
        name: None,
        data: StructureData::Graph {
            direction: Some(direction),
            nodes,
            edges,
        },
        original_len: input.len(),
    })
}

fn parse_direction(line: &str) -> Result<Direction, ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return Err(ParseError::InvalidStructure);
    }

    if parts.len() > 1 {
        match parts[1].to_uppercase().as_str() {
            "TD" | "TB" => Ok(Direction::TopDown),
            "LR" => Ok(Direction::LeftRight),
            "BT" => Ok(Direction::BottomTop),
            "RL" => Ok(Direction::RightLeft),
            _ => Ok(Direction::TopDown),
        }
    } else {
        Ok(Direction::TopDown)
    }
}

fn parse_sequence(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    let participants = Vec::new();
    let messages = Vec::new();

    Ok(ParsedStructure {
        structure_type: StructureType::MermaidSequence,
        name: None,
        data: StructureData::Sequence {
            participants,
            messages,
        },
        original_len: input.len(),
    })
}

fn parse_pie(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
    let items = Vec::new();

    Ok(ParsedStructure {
        structure_type: StructureType::MermaidPie,
        name: None,
        data: StructureData::KeyValue { title: None, items },
        original_len: input.len(),
    })
}
