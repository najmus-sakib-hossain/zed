// Convert parsed structures to DX format

use super::table::ParseError;
use super::*;

impl ToDxFormat for ParsedStructure<'_> {
    fn to_dx_format(&self) -> String {
        match &self.data {
            StructureData::Table {
                columns,
                rows,
                alignments,
            } => table_to_dx(self.name.as_deref(), columns, rows, alignments.as_deref()),
            StructureData::Graph {
                direction,
                nodes,
                edges,
            } => graph_to_dx(direction, nodes, edges),
            StructureData::Sequence {
                participants,
                messages,
            } => sequence_to_dx(participants, messages),
            StructureData::Tree { root, children } => tree_to_dx(root.as_deref(), children),
            StructureData::KeyValue { title, items } => keyvalue_to_dx(title.as_deref(), items),
            StructureData::Schedule { title, sections } => {
                schedule_to_dx(title.as_deref(), sections)
            }
            StructureData::Relations {
                entities,
                relationships,
            } => relations_to_dx(entities, relationships),
        }
    }
}

fn table_to_dx(
    name: Option<&str>,
    columns: &[Cow<'_, str>],
    rows: &[Vec<Cow<'_, str>>],
    _alignments: Option<&[Alignment]>,
) -> String {
    let mut output = String::with_capacity(256);

    let table_name = name.unwrap_or("t");
    output.push_str(table_name);
    output.push(':');
    output.push_str(&rows.len().to_string());
    output.push('(');

    for (i, col) in columns.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(col);
    }
    output.push(')');

    output.push('[');
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        for (j, cell) in row.iter().enumerate() {
            if j > 0 {
                output.push(' ');
            }
            output.push_str(cell);
        }
    }
    output.push(']');

    output
}

fn graph_to_dx(direction: &Option<Direction>, _nodes: &[Node<'_>], _edges: &[Edge<'_>]) -> String {
    let mut output = String::with_capacity(256);

    output.push_str("@flow");

    if let Some(dir) = direction {
        output.push(':');
        output.push_str(match dir {
            Direction::TopDown => "TD",
            Direction::LeftRight => "LR",
            Direction::BottomTop => "BT",
            Direction::RightLeft => "RL",
        });
    }

    output.push_str("[]");
    output
}

fn sequence_to_dx(_participants: &[Cow<'_, str>], _messages: &[Message<'_>]) -> String {
    "@seq[]".to_string()
}

fn tree_to_dx(root: Option<&str>, _children: &[TreeNode<'_>]) -> String {
    let mut output = String::from("@tree");

    if let Some(r) = root {
        output.push(':');
        output.push_str(r);
    }

    output.push_str("[]");
    output
}

fn keyvalue_to_dx(title: Option<&str>, _items: &[(Cow<'_, str>, Cow<'_, str>)]) -> String {
    let mut output = String::from("@pie");

    if let Some(t) = title {
        output.push(':');
        output.push_str(&t.replace(' ', "_"));
    }

    output.push_str("[]");
    output
}

fn schedule_to_dx(title: Option<&str>, _sections: &[ScheduleSection<'_>]) -> String {
    let mut output = String::from("@gantt");

    if let Some(t) = title {
        output.push(':');
        output.push_str(&t.replace(' ', "_"));
    }

    output.push_str("[]");
    output
}

fn relations_to_dx(_entities: &[Entity<'_>], _relationships: &[Relationship<'_>]) -> String {
    "@class[]".to_string()
}

/// Convert any structure to DX format
pub fn convert_to_dx(input: &str, lang_hint: Option<&str>) -> Result<String, ParseError> {
    use super::detector::StructureDetector;

    let structure_type = if let Some(lang) = lang_hint {
        StructureDetector::detect_code_block(input, Some(lang))
    } else if input.contains('|') && input.lines().count() >= 2 {
        StructureDetector::detect_table(input)
    } else {
        StructureDetector::detect_code_block(input, None)
    };

    let structure_type = structure_type.ok_or(ParseError::InvalidStructure)?;

    let parsed = match structure_type {
        StructureType::Table
        | StructureType::TableWithLinks
        | StructureType::TableFeatureMatrix => super::table::TableParser::parse(input)?,

        StructureType::MermaidFlowchart
        | StructureType::MermaidSequence
        | StructureType::MermaidPie => super::mermaid::parse_mermaid(input)?,

        StructureType::AsciiTree => super::ascii::parse_ascii(input)?,

        _ => return Err(ParseError::Other(format!("Unsupported type: {:?}", structure_type))),
    };

    Ok(parsed.to_dx_format())
}

/// Calculate token savings
pub fn calculate_savings(original: &str, optimized: &str) -> f64 {
    let orig_len = original.len() as f64;
    let opt_len = optimized.len() as f64;

    if orig_len == 0.0 {
        return 0.0;
    }

    ((orig_len - opt_len) / orig_len) * 100.0
}
