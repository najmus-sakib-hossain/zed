//! Canvas support for visual workspace

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub elements: Vec<CanvasElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: String,
    pub element_type: ElementType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Text { content: String },
    Image { url: String },
    Shape { shape: Shape },
    Code { language: String, code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Shape {
    Rectangle,
    Circle,
    Line,
    Arrow,
}

impl Canvas {
    pub fn new(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            width,
            height,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, element: CanvasElement) {
        self.elements.push(element);
    }

    pub fn render(&self) -> Result<String> {
        Ok(format!("Canvas {} ({}x{})", self.id, self.width, self.height))
    }
}
