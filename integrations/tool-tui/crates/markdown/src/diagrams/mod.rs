// Diagram and table parsing for DX format conversion

pub mod ascii;
pub mod detector;
pub mod mermaid;
pub mod serializer;
pub mod table;

use std::borrow::Cow;

/// All supported diagram/table types for DX conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureType {
    // Standard Markdown
    Table,
    TableWithLinks,
    TableFeatureMatrix,

    // Mermaid Diagrams
    MermaidFlowchart,
    MermaidSequence,
    MermaidClass,
    MermaidER,
    MermaidState,
    MermaidGantt,
    MermaidPie,
    MermaidGit,

    // ASCII Art
    AsciiTree,
    AsciiBox,
    AsciiFlowchart,
}

/// Result of parsing any structure
#[derive(Debug)]
pub struct ParsedStructure<'a> {
    pub structure_type: StructureType,
    pub name: Option<Cow<'a, str>>,
    pub data: StructureData<'a>,
    pub original_len: usize,
}

/// Unified data representation for all structures
#[derive(Debug)]
pub enum StructureData<'a> {
    /// Tabular data with schema
    Table {
        columns: Vec<Cow<'a, str>>,
        rows: Vec<Vec<Cow<'a, str>>>,
        alignments: Option<Vec<Alignment>>,
    },
    /// Graph/flow data
    Graph {
        direction: Option<Direction>,
        nodes: Vec<Node<'a>>,
        edges: Vec<Edge<'a>>,
    },
    /// Sequence/interaction data
    Sequence {
        participants: Vec<Cow<'a, str>>,
        messages: Vec<Message<'a>>,
    },
    /// Hierarchical/tree data
    Tree {
        root: Option<Cow<'a, str>>,
        children: Vec<TreeNode<'a>>,
    },
    /// Key-value pairs (pie charts, simple configs)
    KeyValue {
        title: Option<Cow<'a, str>>,
        items: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    },
    /// Timeline/schedule data
    Schedule {
        title: Option<Cow<'a, str>>,
        sections: Vec<ScheduleSection<'a>>,
    },
    /// Class/ER relationships
    Relations {
        entities: Vec<Entity<'a>>,
        relationships: Vec<Relationship<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    TopDown,
    LeftRight,
    BottomTop,
    RightLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub id: Cow<'a, str>,
    pub label: Option<Cow<'a, str>>,
    pub shape: NodeShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    #[default]
    Rectangle,
    RoundRect,
    Diamond,
    Circle,
    Stadium,
    Hexagon,
    Database,
}

#[derive(Debug, Clone)]
pub struct Edge<'a> {
    pub from: Cow<'a, str>,
    pub to: Cow<'a, str>,
    pub label: Option<Cow<'a, str>>,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeType {
    #[default]
    Arrow,
    Open,
    Dotted,
    Thick,
    Bidirectional,
}

#[derive(Debug, Clone)]
pub struct Message<'a> {
    pub from: Cow<'a, str>,
    pub to: Cow<'a, str>,
    pub text: Cow<'a, str>,
    pub msg_type: MessageType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageType {
    #[default]
    Sync,
    SyncReply,
    Async,
    AsyncReply,
}

#[derive(Debug, Clone)]
pub struct TreeNode<'a> {
    pub name: Cow<'a, str>,
    pub is_dir: bool,
    pub children: Vec<TreeNode<'a>>,
}

#[derive(Debug, Clone)]
pub struct ScheduleSection<'a> {
    pub name: Cow<'a, str>,
    pub tasks: Vec<ScheduleTask<'a>>,
}

#[derive(Debug, Clone)]
pub struct ScheduleTask<'a> {
    pub id: Cow<'a, str>,
    pub name: Option<Cow<'a, str>>,
    pub start: Cow<'a, str>,
    pub duration: Cow<'a, str>,
}

#[derive(Debug, Clone)]
pub struct Entity<'a> {
    pub name: Cow<'a, str>,
    pub properties: Vec<Property<'a>>,
}

#[derive(Debug, Clone)]
pub struct Property<'a> {
    pub name: Cow<'a, str>,
    pub prop_type: Cow<'a, str>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone)]
pub struct Relationship<'a> {
    pub from: Cow<'a, str>,
    pub to: Cow<'a, str>,
    pub rel_type: RelationType,
    pub label: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    OneToOne,
    OneToMany,
    ManyToMany,
}

/// Trait for converting structures to DX format
pub trait ToDxFormat {
    fn to_dx_format(&self) -> String;
}
