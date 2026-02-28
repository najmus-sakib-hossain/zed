//! Dataclass information and code generation

/// Information about a dataclass for code generation
#[derive(Debug, Clone)]
pub struct DataclassInfo {
    /// Class name
    pub name: String,
    /// Fields with their types
    pub fields: Vec<DataclassField>,
    /// Whether the class is frozen (immutable)
    pub frozen: bool,
    /// Whether to use __slots__
    pub slots: bool,
}

/// A field in a dataclass
#[derive(Debug, Clone)]
pub struct DataclassField {
    /// Field name
    pub name: String,
    /// Field type (as string)
    pub type_hint: Option<String>,
    /// Default value (as bytecode constant index)
    pub default: Option<u32>,
    /// Whether field has a default factory
    pub has_default_factory: bool,
}

impl DataclassInfo {
    /// Create a new dataclass info
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            frozen: false,
            slots: false,
        }
    }

    /// Add a field
    pub fn add_field(&mut self, field: DataclassField) {
        self.fields.push(field);
    }

    /// Generate __init__ bytecode
    pub fn generate_init(&self) -> Vec<u8> {
        // Simplified bytecode generation
        // In a real implementation, this would generate proper DPB bytecode
        let mut bytecode = Vec::new();

        // For each field, generate: self.field = field_arg
        for (i, _field) in self.fields.iter().enumerate() {
            // LOAD_FAST self (0)
            bytecode.push(0x00);
            bytecode.push(0x00);
            bytecode.push(0x00);

            // LOAD_FAST field_arg (i + 1)
            bytecode.push(0x00);
            bytecode.push((i + 1) as u8);
            bytecode.push(0x00);

            // STORE_ATTR field_name_idx
            bytecode.push(0x06);
            bytecode.push(i as u8);
            bytecode.push(0x00);
        }

        // LOAD_CONST None
        bytecode.push(0x14);
        bytecode.push(0x00);

        // RETURN_VALUE
        bytecode.push(0x56);

        bytecode
    }

    /// Generate __repr__ bytecode
    pub fn generate_repr(&self) -> Vec<u8> {
        // Simplified: return f"{ClassName}(field1={self.field1}, ...)"
        // BUILD_STRING with format
        // For simplicity, just return a placeholder
        vec![
            // LOAD_CONST format_string
            0x14, 0x00, // RETURN_VALUE
            0x56,
        ]
    }

    /// Generate __eq__ bytecode
    pub fn generate_eq(&self) -> Vec<u8> {
        // Check type first
        vec![
            // LOAD_FAST self
            0x00, 0x00, 0x00, // LOAD_GLOBAL type
            0x03, 0x00, 0x00,
            // For each field, compare self.field == other.field
            // Simplified: just return True for now
            // LOAD_CONST True
            0x14, 0x01, // RETURN_VALUE
            0x56,
        ]
    }

    /// Generate __hash__ bytecode
    pub fn generate_hash(&self) -> Vec<u8> {
        let mut bytecode = Vec::new();

        // hash(tuple(self.field1, self.field2, ...))

        // For each field, load it
        for i in 0..self.fields.len() {
            // LOAD_FAST self
            bytecode.push(0x00);
            bytecode.push(0x00);
            bytecode.push(0x00);

            // LOAD_ATTR field
            bytecode.push(0x05);
            bytecode.push(i as u8);
            bytecode.push(0x00);
        }

        // BUILD_TUPLE
        bytecode.push(0x80);
        bytecode.push(self.fields.len() as u8);

        // LOAD_GLOBAL hash
        bytecode.push(0x03);
        bytecode.push(0x01);
        bytecode.push(0x00);

        // CALL_FUNCTION 1
        bytecode.push(0x70);
        bytecode.push(0x01);
        bytecode.push(0x00);

        // RETURN_VALUE
        bytecode.push(0x56);

        bytecode
    }

    /// Generate comparison method bytecode
    pub fn generate_comparison(&self, method: &str) -> Vec<u8> {
        let mut bytecode = Vec::new();

        // Compare tuples of fields
        // Simplified implementation

        let op = match method {
            "__lt__" => 0x40, // COMPARE_LT
            "__le__" => 0x41, // COMPARE_LE
            "__gt__" => 0x44, // COMPARE_GT
            "__ge__" => 0x45, // COMPARE_GE
            _ => 0x42,        // COMPARE_EQ
        };

        // Build tuple of self fields
        for i in 0..self.fields.len() {
            bytecode.push(0x00); // LOAD_FAST self
            bytecode.push(0x00);
            bytecode.push(0x00);
            bytecode.push(0x05); // LOAD_ATTR
            bytecode.push(i as u8);
            bytecode.push(0x00);
        }
        bytecode.push(0x80); // BUILD_TUPLE
        bytecode.push(self.fields.len() as u8);

        // Build tuple of other fields
        for i in 0..self.fields.len() {
            bytecode.push(0x00); // LOAD_FAST other
            bytecode.push(0x01);
            bytecode.push(0x00);
            bytecode.push(0x05); // LOAD_ATTR
            bytecode.push(i as u8);
            bytecode.push(0x00);
        }
        bytecode.push(0x80); // BUILD_TUPLE
        bytecode.push(self.fields.len() as u8);

        // Compare
        bytecode.push(op);

        // RETURN_VALUE
        bytecode.push(0x56);

        bytecode
    }

    /// Get the __slots__ tuple for this dataclass
    pub fn get_slots(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.name.clone()).collect()
    }
}

impl DataclassField {
    /// Create a new field
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_hint: None,
            default: None,
            has_default_factory: false,
        }
    }

    /// Set the type hint
    pub fn with_type(mut self, type_hint: &str) -> Self {
        self.type_hint = Some(type_hint.to_string());
        self
    }

    /// Set the default value
    pub fn with_default(mut self, default: u32) -> Self {
        self.default = Some(default);
        self
    }

    /// Mark as having a default factory
    pub fn with_default_factory(mut self) -> Self {
        self.has_default_factory = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataclass_info() {
        let mut info = DataclassInfo::new("Point");
        info.add_field(DataclassField::new("x").with_type("int"));
        info.add_field(DataclassField::new("y").with_type("int"));

        assert_eq!(info.name, "Point");
        assert_eq!(info.fields.len(), 2);
    }

    #[test]
    fn test_generate_init() {
        let mut info = DataclassInfo::new("Point");
        info.add_field(DataclassField::new("x"));
        info.add_field(DataclassField::new("y"));

        let bytecode = info.generate_init();
        assert!(!bytecode.is_empty());

        // Should end with RETURN_VALUE
        assert_eq!(*bytecode.last().unwrap(), 0x56);
    }

    #[test]
    fn test_get_slots() {
        let mut info = DataclassInfo::new("Point");
        info.add_field(DataclassField::new("x"));
        info.add_field(DataclassField::new("y"));
        info.add_field(DataclassField::new("z"));

        let slots = info.get_slots();
        assert_eq!(slots, vec!["x", "y", "z"]);
    }

    #[test]
    fn test_field_builder() {
        let field = DataclassField::new("value").with_type("int").with_default(42);

        assert_eq!(field.name, "value");
        assert_eq!(field.type_hint, Some("int".to_string()));
        assert_eq!(field.default, Some(42));
    }
}
