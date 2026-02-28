// Structure detection for tables and diagrams

use super::StructureType;

/// Detects structure type from content
pub struct StructureDetector;

impl StructureDetector {
    /// Detect from a code block with optional language hint
    pub fn detect_code_block(content: &str, lang: Option<&str>) -> Option<StructureType> {
        // Check language hint first
        if let Some(lang) = lang
            && lang.to_lowercase().as_str() == "mermaid"
        {
            return Self::detect_mermaid(content);
        }

        // Auto-detect from content
        if Self::is_ascii_tree(content) {
            return Some(StructureType::AsciiTree);
        }
        if Self::is_ascii_box(content) {
            return Some(StructureType::AsciiBox);
        }
        if Self::is_ascii_flowchart(content) {
            return Some(StructureType::AsciiFlowchart);
        }

        None
    }

    /// Detect from a markdown table
    pub fn detect_table(content: &str) -> Option<StructureType> {
        if !content.contains('|') {
            return None;
        }

        // Check for URLs in table
        if content.contains("http://") || content.contains("https://") {
            return Some(StructureType::TableWithLinks);
        }

        // Check for checkboxes/feature matrix
        if content.contains("✅")
            || content.contains("❌")
            || content.contains("[x]")
            || content.contains("[ ]")
        {
            return Some(StructureType::TableFeatureMatrix);
        }

        Some(StructureType::Table)
    }

    /// Detect Mermaid diagram type
    fn detect_mermaid(content: &str) -> Option<StructureType> {
        let first_line = content.lines().next()?.trim().to_lowercase();
        let first_word = first_line.split_whitespace().next()?;

        Some(match first_word {
            "graph" | "flowchart" => StructureType::MermaidFlowchart,
            "sequencediagram" => StructureType::MermaidSequence,
            "classdiagram" => StructureType::MermaidClass,
            "erdiagram" => StructureType::MermaidER,
            "statediagram" | "statediagram-v2" => StructureType::MermaidState,
            "gantt" => StructureType::MermaidGantt,
            "pie" => StructureType::MermaidPie,
            "gitgraph" => StructureType::MermaidGit,
            _ => return None,
        })
    }

    /// Check for ASCII tree structure
    fn is_ascii_tree(content: &str) -> bool {
        let tree_indicators = ['├', '└', '│', '─'];
        let has_tree_chars = content.chars().any(|c| tree_indicators.contains(&c));
        let has_tree_pattern = content
            .lines()
            .any(|l| l.contains("├──") || l.contains("└──") || l.trim().starts_with("│"));
        has_tree_chars && has_tree_pattern
    }

    /// Check for ASCII box diagram
    fn is_ascii_box(content: &str) -> bool {
        let box_chars = ['┌', '┐', '└', '┘', '╔', '╗', '╚', '╝', '+'];
        let corner_count = content.chars().filter(|c| box_chars.contains(c)).count();

        if corner_count >= 4 {
            return true;
        }

        let has_ascii_corners = content.contains("+--") && content.contains("--+");
        let has_arrows = content.contains("-->")
            || content.contains("--->")
            || content.contains("─>")
            || content.contains("──►");

        has_ascii_corners && has_arrows
    }

    /// Check for ASCII flowchart (arrows without box structure)
    fn is_ascii_flowchart(content: &str) -> bool {
        let arrow_patterns = ["-->", "--->", "->", "=>", "─>", "──►", "▼", "▲"];
        let has_arrows = arrow_patterns.iter().any(|p| content.contains(p));

        has_arrows && !Self::is_ascii_tree(content) && !Self::is_ascii_box(content)
    }
}
