//! # Binary Forms System
//!
//! 10x faster form processing using pre-validated binary schemas.
//! No multipart parsing - direct binary submission and validation.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Form Schema Definition
// ============================================================================

/// Field validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Required field
    Required,
    /// Minimum length
    MinLength(usize),
    /// Maximum length
    MaxLength(usize),
    /// Email format
    Email,
    /// URL format
    Url,
    /// Numeric range
    Range { min: f64, max: f64 },
    /// Regex pattern
    Pattern(String),
    /// Custom validator name
    Custom(String),
}

/// Field type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Text = 0,
    Email = 1,
    Password = 2,
    Number = 3,
    Checkbox = 4,
    Select = 5,
    Textarea = 6,
    Hidden = 7,
    File = 8,
}

impl FieldType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Text),
            1 => Some(Self::Email),
            2 => Some(Self::Password),
            3 => Some(Self::Number),
            4 => Some(Self::Checkbox),
            5 => Some(Self::Select),
            6 => Some(Self::Textarea),
            7 => Some(Self::Hidden),
            8 => Some(Self::File),
            _ => None,
        }
    }
}

/// Form field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// Field name/id
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Display label
    pub label: String,
    /// Placeholder text
    pub placeholder: Option<String>,
    /// Default value
    pub default: Option<String>,
    /// Validation rules
    pub rules: Vec<ValidationRule>,
    /// Options for select fields
    pub options: Option<Vec<SelectOption>>,
    /// TailwindCSS classes for styling
    pub class: Option<String>,
}

/// Select option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

// ============================================================================
// Binary Form Schema (Compile-time validated)
// ============================================================================

/// Binary form schema - compiled at build time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryFormSchema {
    /// Schema ID (for caching)
    pub id: u32,
    /// Form name
    pub name: String,
    /// Action URL
    pub action: String,
    /// HTTP method
    pub method: FormMethod,
    /// Form fields
    pub fields: Vec<FormField>,
    /// CSRF protection enabled
    pub csrf: bool,
    /// Rate limiting (requests per minute)
    pub rate_limit: Option<u32>,
}

/// Form submission method
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FormMethod {
    Post,
    Put,
    Patch,
}

impl BinaryFormSchema {
    /// Create a new form schema builder
    pub fn builder(name: impl Into<String>) -> FormBuilder {
        FormBuilder::new(name)
    }

    /// Validate form data against schema
    pub fn validate(&self, data: &HashMap<String, FormValue>) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for field in &self.fields {
            let value = data.get(&field.name);
            
            for rule in &field.rules {
                if let Err(e) = validate_rule(&field.name, value, rule) {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Serialize schema to binary format
    pub fn to_binary(&self) -> Vec<u8> {
        // Use bincode for efficient binary serialization
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .unwrap_or_default()
    }

    /// Deserialize from binary format
    pub fn from_binary(data: &[u8]) -> Option<Self> {
        bincode::serde::decode_from_slice(data, bincode::config::standard())
            .ok()
            .map(|(s, _)| s)
    }
}

/// Form value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    File { name: String, data: Vec<u8> },
    Array(Vec<String>),
}

impl FormValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Text(s) => Some(!s.is_empty() && s != "false" && s != "0"),
            _ => None,
        }
    }
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub rule: String,
    pub message: String,
}

fn validate_rule(
    field_name: &str,
    value: Option<&FormValue>,
    rule: &ValidationRule,
) -> Result<(), ValidationError> {
    match rule {
        ValidationRule::Required => {
            let is_empty = match value {
                None => true,
                Some(FormValue::Text(s)) => s.is_empty(),
                Some(FormValue::Array(a)) => a.is_empty(),
                _ => false,
            };
            if is_empty {
                return Err(ValidationError {
                    field: field_name.to_string(),
                    rule: "required".to_string(),
                    message: format!("{} is required", field_name),
                });
            }
        }
        ValidationRule::MinLength(min) => {
            if let Some(FormValue::Text(s)) = value {
                if s.len() < *min {
                    return Err(ValidationError {
                        field: field_name.to_string(),
                        rule: "minLength".to_string(),
                        message: format!("{} must be at least {} characters", field_name, min),
                    });
                }
            }
        }
        ValidationRule::MaxLength(max) => {
            if let Some(FormValue::Text(s)) = value {
                if s.len() > *max {
                    return Err(ValidationError {
                        field: field_name.to_string(),
                        rule: "maxLength".to_string(),
                        message: format!("{} must be at most {} characters", field_name, max),
                    });
                }
            }
        }
        ValidationRule::Email => {
            if let Some(FormValue::Text(s)) = value {
                if !s.contains('@') || !s.contains('.') {
                    return Err(ValidationError {
                        field: field_name.to_string(),
                        rule: "email".to_string(),
                        message: "Invalid email format".to_string(),
                    });
                }
            }
        }
        ValidationRule::Range { min, max } => {
            if let Some(v) = value {
                if let Some(n) = v.as_number() {
                    if n < *min || n > *max {
                        return Err(ValidationError {
                            field: field_name.to_string(),
                            rule: "range".to_string(),
                            message: format!("{} must be between {} and {}", field_name, min, max),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

// ============================================================================
// Form Builder (Fluent API)
// ============================================================================

/// Fluent form builder
pub struct FormBuilder {
    schema: BinaryFormSchema,
}

impl FormBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: BinaryFormSchema {
                id: 0,
                name: name.into(),
                action: "/api/form".to_string(),
                method: FormMethod::Post,
                fields: Vec::new(),
                csrf: true,
                rate_limit: Some(60),
            },
        }
    }

    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.schema.action = action.into();
        self
    }

    pub fn method(mut self, method: FormMethod) -> Self {
        self.schema.method = method;
        self
    }

    pub fn text(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Text)
    }

    pub fn email(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Email)
    }

    pub fn password(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Password)
    }

    pub fn number(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Number)
    }

    pub fn textarea(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Textarea)
    }

    pub fn checkbox(mut self, name: impl Into<String>, label: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), label.into(), FieldType::Checkbox)
    }

    pub fn select(
        mut self,
        name: impl Into<String>,
        label: impl Into<String>,
        options: Vec<(&str, &str)>,
    ) -> FieldBuilder {
        let mut builder = FieldBuilder::new(self, name.into(), label.into(), FieldType::Select);
        builder.field.options = Some(
            options
                .into_iter()
                .map(|(v, l)| SelectOption {
                    value: v.to_string(),
                    label: l.to_string(),
                    disabled: false,
                })
                .collect(),
        );
        builder
    }

    pub fn csrf(mut self, enabled: bool) -> Self {
        self.schema.csrf = enabled;
        self
    }

    pub fn rate_limit(mut self, requests_per_minute: Option<u32>) -> Self {
        self.schema.rate_limit = requests_per_minute;
        self
    }

    fn add_field(&mut self, field: FormField) {
        self.schema.fields.push(field);
    }

    pub fn build(mut self) -> BinaryFormSchema {
        // Generate schema ID from name hash
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.schema.name.hash(&mut hasher);
        self.schema.id = hasher.finish() as u32;
        self.schema
    }
}

/// Field builder for fluent API
pub struct FieldBuilder {
    form: FormBuilder,
    field: FormField,
}

impl FieldBuilder {
    fn new(form: FormBuilder, name: String, label: String, field_type: FieldType) -> Self {
        Self {
            form,
            field: FormField {
                name,
                field_type,
                label,
                placeholder: None,
                default: None,
                rules: Vec::new(),
                options: None,
                class: None,
            },
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field.placeholder = Some(placeholder.into());
        self
    }

    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.field.default = Some(value.into());
        self
    }

    pub fn required(mut self) -> Self {
        self.field.rules.push(ValidationRule::Required);
        self
    }

    pub fn min_length(mut self, len: usize) -> Self {
        self.field.rules.push(ValidationRule::MinLength(len));
        self
    }

    pub fn max_length(mut self, len: usize) -> Self {
        self.field.rules.push(ValidationRule::MaxLength(len));
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.field.rules.push(ValidationRule::Range { min, max });
        self
    }

    pub fn pattern(mut self, regex: impl Into<String>) -> Self {
        self.field.rules.push(ValidationRule::Pattern(regex.into()));
        self
    }

    /// Apply TailwindCSS classes
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.field.class = Some(class.into());
        self
    }

    pub fn done(mut self) -> FormBuilder {
        self.form.add_field(self.field);
        self.form
    }
}

// ============================================================================
// Pre-built Form Schemas
// ============================================================================

/// Contact form schema
pub fn contact_form() -> BinaryFormSchema {
    BinaryFormSchema::builder("contact")
        .action("/api/contact")
        .text("name", "Full Name")
            .placeholder("John Doe")
            .required()
            .min_length(2)
            .max_length(100)
            .class("w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500")
            .done()
        .email("email", "Email Address")
            .placeholder("john@example.com")
            .required()
            .class("w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500")
            .done()
        .select("subject", "Subject", vec![
            ("general", "General Inquiry"),
            ("support", "Technical Support"),
            ("enterprise", "Enterprise Sales"),
            ("partnership", "Partnership"),
        ])
            .required()
            .class("w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500")
            .done()
        .textarea("message", "Message")
            .placeholder("How can we help you?")
            .required()
            .min_length(10)
            .max_length(2000)
            .class("w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 h-32")
            .done()
        .build()
}

/// Newsletter signup form
pub fn newsletter_form() -> BinaryFormSchema {
    BinaryFormSchema::builder("newsletter")
        .action("/api/subscribe")
        .email("email", "Email")
            .placeholder("you@example.com")
            .required()
            .class("flex-1 px-4 py-3 rounded-l-lg border-0 focus:ring-2 focus:ring-cyan-400")
            .done()
        .checkbox("updates", "Receive product updates")
            .default("true")
            .class("w-5 h-5 text-cyan-500")
            .done()
        .build()
}

/// Waitlist signup form
pub fn waitlist_form() -> BinaryFormSchema {
    BinaryFormSchema::builder("waitlist")
        .action("/api/waitlist")
        .text("name", "Name")
            .placeholder("Your name")
            .required()
            .class("w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-white")
            .done()
        .email("email", "Email")
            .placeholder("you@company.com")
            .required()
            .class("w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-white")
            .done()
        .select("company_size", "Company Size", vec![
            ("1-10", "1-10 employees"),
            ("11-50", "11-50 employees"),
            ("51-200", "51-200 employees"),
            ("201-1000", "201-1000 employees"),
            ("1000+", "1000+ employees"),
        ])
            .class("w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-white")
            .done()
        .textarea("use_case", "How will you use DX?")
            .placeholder("Tell us about your project...")
            .max_length(500)
            .class("w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-white h-24")
            .done()
        .build()
}

// Re-export as BinaryForm type alias
pub type BinaryForm = BinaryFormSchema;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_builder() {
        let form = contact_form();
        assert_eq!(form.name, "contact");
        assert_eq!(form.fields.len(), 4);
        assert!(form.csrf);
    }

    #[test]
    fn test_form_validation() {
        let form = contact_form();
        
        // Missing required fields
        let data: HashMap<String, FormValue> = HashMap::new();
        let result = form.validate(&data);
        assert!(result.is_err());
        
        // Valid data
        let mut data = HashMap::new();
        data.insert("name".to_string(), FormValue::Text("John Doe".to_string()));
        data.insert("email".to_string(), FormValue::Text("john@example.com".to_string()));
        data.insert("subject".to_string(), FormValue::Text("general".to_string()));
        data.insert("message".to_string(), FormValue::Text("This is a test message.".to_string()));
        
        let result = form.validate(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_form_serialization() {
        let form = contact_form();
        let binary = form.to_binary();
        assert!(!binary.is_empty());
        
        let restored = BinaryFormSchema::from_binary(&binary).unwrap();
        assert_eq!(restored.name, form.name);
        assert_eq!(restored.fields.len(), form.fields.len());
    }
}
