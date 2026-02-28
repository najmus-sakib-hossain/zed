//! RKYV-compatible types for machine format serialization
//!
//! Uses a flattened arena-based approach to avoid recursive types.
//! All nested values are stored in a flat Vec with indices.

use rkyv::{Archive, Deserialize, Serialize};

/// RKYV-compatible document with flattened value arena
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub struct MachineDocument {
    pub context: Vec<(String, usize)>, // key -> value index
    pub refs: Vec<(String, String)>,
    pub sections: Vec<(char, MachineSection)>,
    pub section_names: Vec<(char, String)>,
    pub entry_order: Vec<MachineEntryRef>,
    pub value_arena: Vec<MachineValue>, // Flat storage for all values
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub enum MachineEntryRef {
    Context(String),
    Section(char),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub struct MachineSection {
    pub schema: Vec<String>,
    pub rows: Vec<Vec<usize>>, // Each cell is an index into value_arena
}

/// Non-recursive value type - arrays/objects store indices
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub enum MachineValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Arr(Vec<usize>),           // Indices into value_arena
    Obj(Vec<(String, usize)>), // key -> index into value_arena
    Ref(String),
}

impl From<&crate::llm::types::DxDocument> for MachineDocument {
    fn from(doc: &crate::llm::types::DxDocument) -> Self {
        use crate::llm::types::EntryRef;

        let mut value_arena = Vec::new();
        let mut context = Vec::new();

        // Convert context values
        for (k, v) in &doc.context {
            let idx = add_value_to_arena(v, &mut value_arena);
            context.push((k.clone(), idx));
        }

        // Convert sections
        let sections: Vec<(char, MachineSection)> = doc
            .sections
            .iter()
            .map(|(k, v)| {
                let schema = v.schema.clone();
                let rows: Vec<Vec<usize>> = v
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter().map(|cell| add_value_to_arena(cell, &mut value_arena)).collect()
                    })
                    .collect();
                (*k, MachineSection { schema, rows })
            })
            .collect();

        Self {
            context,
            refs: doc.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            sections,
            section_names: doc.section_names.iter().map(|(k, v)| (*k, v.clone())).collect(),
            entry_order: doc
                .entry_order
                .iter()
                .map(|e| match e {
                    EntryRef::Context(s) => MachineEntryRef::Context(s.clone()),
                    EntryRef::Section(c) => MachineEntryRef::Section(*c),
                })
                .collect(),
            value_arena,
        }
    }
}

fn add_value_to_arena(v: &crate::llm::types::DxLlmValue, arena: &mut Vec<MachineValue>) -> usize {
    use crate::llm::types::DxLlmValue;

    let idx = arena.len();
    match v {
        DxLlmValue::Str(s) => arena.push(MachineValue::Str(s.clone())),
        DxLlmValue::Num(n) => arena.push(MachineValue::Num(*n)),
        DxLlmValue::Bool(b) => arena.push(MachineValue::Bool(*b)),
        DxLlmValue::Null => arena.push(MachineValue::Null),
        DxLlmValue::Arr(items) => {
            let indices: Vec<usize> =
                items.iter().map(|item| add_value_to_arena(item, arena)).collect();
            arena.push(MachineValue::Arr(indices));
        }
        DxLlmValue::Obj(fields) => {
            let pairs: Vec<(String, usize)> =
                fields.iter().map(|(k, v)| (k.clone(), add_value_to_arena(v, arena))).collect();
            arena.push(MachineValue::Obj(pairs));
        }
        DxLlmValue::Ref(r) => arena.push(MachineValue::Ref(r.clone())),
    }
    idx
}

impl From<&MachineDocument> for crate::llm::types::DxDocument {
    fn from(m: &MachineDocument) -> Self {
        use crate::llm::types::{DxDocument, DxSection, EntryRef};

        let mut doc = DxDocument::new();

        // Convert context
        for (k, idx) in &m.context {
            let value = get_value_from_arena(*idx, &m.value_arena);
            doc.context.insert(k.clone(), value);
        }

        doc.refs = m.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Convert sections
        for (k, section) in &m.sections {
            let mut dx_section = DxSection::new(section.schema.clone());
            for row_indices in &section.rows {
                let row: Vec<_> = row_indices
                    .iter()
                    .map(|idx| get_value_from_arena(*idx, &m.value_arena))
                    .collect();
                dx_section.rows.push(row);
            }
            doc.sections.insert(*k, dx_section);
        }

        doc.section_names = m.section_names.iter().map(|(k, v)| (*k, v.clone())).collect();
        doc.entry_order = m
            .entry_order
            .iter()
            .map(|e| match e {
                MachineEntryRef::Context(s) => EntryRef::Context(s.clone()),
                MachineEntryRef::Section(c) => EntryRef::Section(*c),
            })
            .collect();
        doc
    }
}

fn get_value_from_arena(idx: usize, arena: &[MachineValue]) -> crate::llm::types::DxLlmValue {
    use crate::llm::types::DxLlmValue;
    use indexmap::IndexMap;

    match &arena[idx] {
        MachineValue::Str(s) => DxLlmValue::Str(s.clone()),
        MachineValue::Num(n) => DxLlmValue::Num(*n),
        MachineValue::Bool(b) => DxLlmValue::Bool(*b),
        MachineValue::Null => DxLlmValue::Null,
        MachineValue::Arr(indices) => {
            let items: Vec<DxLlmValue> =
                indices.iter().map(|i| get_value_from_arena(*i, arena)).collect();
            DxLlmValue::Arr(items)
        }
        MachineValue::Obj(pairs) => {
            let mut fields = IndexMap::new();
            for (k, i) in pairs {
                fields.insert(k.clone(), get_value_from_arena(*i, arena));
            }
            DxLlmValue::Obj(fields)
        }
        MachineValue::Ref(r) => DxLlmValue::Ref(r.clone()),
    }
}
