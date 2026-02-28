//! RKYV-compatible types for DxDocument serialization

use crate::llm::types::{DxDocument, DxLlmValue, DxSection, EntryRef};

/// RKYV-compatible DxDocument (uses Vec instead of IndexMap)
#[derive(Debug, Clone, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(derive(Debug))]
pub struct RkyvDxDocument {
    pub context: Vec<(String, RkyvDxLlmValue)>,
    pub refs: Vec<(String, String)>,
    pub sections: Vec<(char, RkyvDxSection)>,
    pub section_names: Vec<(char, String)>,
    pub entry_order: Vec<RkyvEntryRef>,
}

#[derive(Debug, Clone, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(derive(Debug))]
pub struct RkyvDxSection {
    pub schema: Vec<String>,
    pub rows: Vec<Vec<RkyvDxLlmValue>>,
}

#[derive(Debug, Clone, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(derive(Debug))]
pub enum RkyvDxLlmValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Arr(Vec<RkyvDxLlmValue>),
    Obj(Vec<(String, RkyvDxLlmValue)>),
    Ref(String),
}

#[derive(Debug, Clone, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(derive(Debug))]
pub enum RkyvEntryRef {
    Context(String),
    Section(char),
}

// Conversion from DxDocument to RkyvDxDocument
impl From<&DxDocument> for RkyvDxDocument {
    fn from(doc: &DxDocument) -> Self {
        Self {
            context: doc.context.iter().map(|(k, v)| (k.clone(), v.into())).collect(),
            refs: doc.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            sections: doc.sections.iter().map(|(k, v)| (*k, v.into())).collect(),
            section_names: doc.section_names.iter().map(|(k, v)| (*k, v.clone())).collect(),
            entry_order: doc.entry_order.iter().map(|e| e.into()).collect(),
        }
    }
}

impl From<&RkyvDxDocument> for DxDocument {
    fn from(rkyv: &RkyvDxDocument) -> Self {
        let mut doc = DxDocument::new();
        for (k, v) in &rkyv.context {
            doc.context.insert(k.clone(), v.into());
        }
        for (k, v) in &rkyv.refs {
            doc.refs.insert(k.clone(), v.clone());
        }
        for (k, v) in &rkyv.sections {
            doc.sections.insert(*k, v.into());
        }
        for (k, v) in &rkyv.section_names {
            doc.section_names.insert(*k, v.clone());
        }
        for e in &rkyv.entry_order {
            doc.entry_order.push(e.into());
        }
        doc
    }
}

impl From<&DxSection> for RkyvDxSection {
    fn from(section: &DxSection) -> Self {
        Self {
            schema: section.schema.clone(),
            rows: section.rows.iter().map(|row| row.iter().map(|v| v.into()).collect()).collect(),
        }
    }
}

impl From<&RkyvDxSection> for DxSection {
    fn from(rkyv: &RkyvDxSection) -> Self {
        Self {
            schema: rkyv.schema.clone(),
            rows: rkyv.rows.iter().map(|row| row.iter().map(|v| v.into()).collect()).collect(),
        }
    }
}

impl From<&DxLlmValue> for RkyvDxLlmValue {
    fn from(value: &DxLlmValue) -> Self {
        match value {
            DxLlmValue::Str(s) => RkyvDxLlmValue::Str(s.clone()),
            DxLlmValue::Num(n) => RkyvDxLlmValue::Num(*n),
            DxLlmValue::Bool(b) => RkyvDxLlmValue::Bool(*b),
            DxLlmValue::Null => RkyvDxLlmValue::Null,
            DxLlmValue::Arr(arr) => RkyvDxLlmValue::Arr(arr.iter().map(|v| v.into()).collect()),
            DxLlmValue::Obj(obj) => {
                RkyvDxLlmValue::Obj(obj.iter().map(|(k, v)| (k.clone(), v.into())).collect())
            }
            DxLlmValue::Ref(r) => RkyvDxLlmValue::Ref(r.clone()),
        }
    }
}

impl From<&RkyvDxLlmValue> for DxLlmValue {
    fn from(rkyv: &RkyvDxLlmValue) -> Self {
        match rkyv {
            RkyvDxLlmValue::Str(s) => DxLlmValue::Str(s.clone()),
            RkyvDxLlmValue::Num(n) => DxLlmValue::Num(*n),
            RkyvDxLlmValue::Bool(b) => DxLlmValue::Bool(*b),
            RkyvDxLlmValue::Null => DxLlmValue::Null,
            RkyvDxLlmValue::Arr(arr) => DxLlmValue::Arr(arr.iter().map(|v| v.into()).collect()),
            RkyvDxLlmValue::Obj(obj) => {
                let mut map = indexmap::IndexMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), v.into());
                }
                DxLlmValue::Obj(map)
            }
            RkyvDxLlmValue::Ref(r) => DxLlmValue::Ref(r.clone()),
        }
    }
}

impl From<&EntryRef> for RkyvEntryRef {
    fn from(entry: &EntryRef) -> Self {
        match entry {
            EntryRef::Context(k) => RkyvEntryRef::Context(k.clone()),
            EntryRef::Section(c) => RkyvEntryRef::Section(*c),
        }
    }
}

impl From<&RkyvEntryRef> for EntryRef {
    fn from(rkyv: &RkyvEntryRef) -> Self {
        match rkyv {
            RkyvEntryRef::Context(k) => EntryRef::Context(k.clone()),
            RkyvEntryRef::Section(c) => EntryRef::Section(*c),
        }
    }
}
