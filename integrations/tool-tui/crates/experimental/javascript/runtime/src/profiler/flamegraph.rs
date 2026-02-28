//! Flame Graph Generator for performance visualization

use super::cpu::CpuProfile;
use std::collections::HashMap;

pub struct FlameGraph {
    nodes: Vec<FlameNode>,
}

#[derive(Debug, Clone)]
pub struct FlameNode {
    pub name: String,
    pub value: u64,
    pub children: Vec<usize>,
}

impl FlameGraph {
    pub fn from_profile(profile: &CpuProfile) -> Self {
        let mut nodes = Vec::new();
        let mut node_map: HashMap<String, usize> = HashMap::new();

        for (func, count, _) in profile.hot_functions(100) {
            let idx = nodes.len();
            node_map.insert(func.clone(), idx);
            nodes.push(FlameNode {
                name: func,
                value: count,
                children: Vec::new(),
            });
        }

        Self { nodes }
    }

    pub fn to_svg(&self, width: u32, height: u32) -> String {
        let mut svg = format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">"#,
            width, height
        );

        let total: u64 = self.nodes.iter().map(|n| n.value).sum();
        let mut y = 0;

        for node in &self.nodes {
            let w = (node.value as f64 / total as f64 * width as f64) as u32;
            let color = self.color_for_node(&node.name);
            svg.push_str(&format!(
                r#"<rect x="0" y="{}" width="{}" height="20" fill="{}" stroke="white"/><text x="5" y="{}" font-size="12" fill="black">{}</text>"#,
                y, w, color, y + 15, node.name
            ));
            y += 25;
        }

        svg.push_str("</svg>");
        svg
    }

    fn color_for_node(&self, name: &str) -> String {
        let hash = name.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let r = (hash % 200) + 55;
        let g = ((hash >> 8) % 200) + 55;
        let b = ((hash >> 16) % 200) + 55;
        format!("rgb({},{},{})", r, g, b)
    }

    pub fn to_json(&self) -> String {
        let mut json = String::from("{\"nodes\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(r#"{{"name":"{}","value":{}}}"#, node.name, node.value));
        }
        json.push_str("]}");
        json
    }
}
