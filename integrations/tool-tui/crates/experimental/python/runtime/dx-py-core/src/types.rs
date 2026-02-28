//! PyType - Type object and type system

use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pylist::PyValue;
use dashmap::DashMap;
use std::sync::Arc;

/// Type object representing a Python type
#[derive(Debug)]
pub struct PyType {
    /// Object header
    pub header: PyObjectHeader,
    /// Type name
    pub name: String,
    /// Qualified name
    pub qualname: String,
    /// Module name
    pub module: Option<String>,
    /// Base types (for MRO)
    pub bases: Vec<Arc<PyType>>,
    /// Method resolution order
    pub mro: Vec<Arc<PyType>>,
    /// Type attributes/methods
    pub dict: DashMap<String, PyValue>,
    /// Type flags
    pub type_flags: TypeFlags,
    /// __slots__ if defined
    pub slots: Option<Vec<String>>,
    /// Metaclass
    pub metaclass: Option<Arc<PyType>>,
}

/// Type-specific flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeFlags {
    pub is_abstract: bool,
    pub is_final: bool,
    pub has_gc: bool,
    pub has_dict: bool,
    pub has_slots: bool,
    pub is_basetype: bool,
    pub is_heaptype: bool,
}

/// Slot in a type's dictionary (legacy, kept for compatibility)
#[derive(Clone)]
pub enum PyTypeSlot {
    /// A method (function)
    Method(Arc<dyn Fn() + Send + Sync>),
    /// A class method
    ClassMethod(Arc<dyn Fn() + Send + Sync>),
    /// A static method
    StaticMethod(Arc<dyn Fn() + Send + Sync>),
    /// A property
    Property {
        getter: Option<Arc<dyn Fn() + Send + Sync>>,
        setter: Option<Arc<dyn Fn() + Send + Sync>>,
        deleter: Option<Arc<dyn Fn() + Send + Sync>>,
    },
    /// A data descriptor
    Data(Arc<dyn std::any::Any + Send + Sync>),
}

/// Descriptor protocol trait
pub trait Descriptor: Send + Sync {
    /// __get__(self, obj, objtype=None)
    fn get(&self, obj: Option<&PyValue>, objtype: Option<&Arc<PyType>>) -> Option<PyValue>;

    /// __set__(self, obj, value) - returns true if this is a data descriptor
    fn set(&self, _obj: &PyValue, _value: PyValue) -> Option<bool> {
        None // Non-data descriptor by default
    }

    /// __delete__(self, obj) - returns true if this is a data descriptor
    fn delete(&self, _obj: &PyValue) -> Option<bool> {
        None // Non-data descriptor by default
    }

    /// Check if this is a data descriptor (has __set__ or __delete__)
    fn is_data_descriptor(&self) -> bool {
        false
    }
}

/// Property descriptor implementation
pub struct PropertyDescriptor {
    pub fget: Option<PyValue>,
    pub fset: Option<PyValue>,
    pub fdel: Option<PyValue>,
    pub doc: Option<String>,
}

impl PropertyDescriptor {
    pub fn new(fget: Option<PyValue>, fset: Option<PyValue>, fdel: Option<PyValue>) -> Self {
        Self {
            fget,
            fset,
            fdel,
            doc: None,
        }
    }

    /// Create a property with just a getter
    pub fn getter(fget: PyValue) -> Self {
        Self {
            fget: Some(fget),
            fset: None,
            fdel: None,
            doc: None,
        }
    }

    /// Create a new property with a setter
    pub fn setter(&self, fset: PyValue) -> Self {
        Self {
            fget: self.fget.clone(),
            fset: Some(fset),
            fdel: self.fdel.clone(),
            doc: self.doc.clone(),
        }
    }

    /// Create a new property with a deleter
    pub fn deleter(&self, fdel: PyValue) -> Self {
        Self {
            fget: self.fget.clone(),
            fset: self.fset.clone(),
            fdel: Some(fdel),
            doc: self.doc.clone(),
        }
    }

    /// Set the docstring
    pub fn with_doc(mut self, doc: String) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Get the docstring
    pub fn get_doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }

    /// Check if this property is read-only
    pub fn is_readonly(&self) -> bool {
        self.fget.is_some() && self.fset.is_none()
    }

    /// Check if this property is write-only
    pub fn is_writeonly(&self) -> bool {
        self.fget.is_none() && self.fset.is_some()
    }
}

impl Descriptor for PropertyDescriptor {
    fn get(&self, obj: Option<&PyValue>, _objtype: Option<&Arc<PyType>>) -> Option<PyValue> {
        obj?;
        // Would call fget here - for now just return None
        self.fget.clone()
    }

    fn set(&self, _obj: &PyValue, _value: PyValue) -> Option<bool> {
        if self.fset.is_some() {
            // Would call fset here
            Some(true)
        } else {
            None
        }
    }

    fn delete(&self, _obj: &PyValue) -> Option<bool> {
        if self.fdel.is_some() {
            // Would call fdel here
            Some(true)
        } else {
            None
        }
    }

    fn is_data_descriptor(&self) -> bool {
        self.fset.is_some() || self.fdel.is_some()
    }
}

/// Method binding types
#[derive(Debug, Clone)]
pub enum BoundMethod {
    /// Instance method (bound to instance)
    Instance {
        method: Box<PyValue>,
        instance: Arc<PyInstance>,
    },
    /// Class method (bound to class)
    Class {
        method: Box<PyValue>,
        class: Arc<PyType>,
    },
    /// Static method (not bound)
    Static { method: Box<PyValue> },
    /// Unbound method (function)
    Unbound { method: Box<PyValue> },
    /// String method (bound to a string value)
    String {
        value: Arc<str>,
        method: String,
    },
    /// List method (bound to a list)
    List {
        value: Arc<crate::pylist::PyList>,
        method: String,
    },
    /// Dict method (bound to a dict)
    Dict {
        value: Arc<crate::PyDict>,
        method: String,
    },
}

impl BoundMethod {
    /// Create an instance method binding
    pub fn bind_instance(method: PyValue, instance: Arc<PyInstance>) -> Self {
        Self::Instance {
            method: Box::new(method),
            instance,
        }
    }

    /// Create a class method binding
    pub fn bind_class(method: PyValue, class: Arc<PyType>) -> Self {
        Self::Class {
            method: Box::new(method),
            class,
        }
    }

    /// Create a static method (no binding)
    pub fn static_method(method: PyValue) -> Self {
        Self::Static {
            method: Box::new(method),
        }
    }

    /// Create an unbound method
    pub fn unbound(method: PyValue) -> Self {
        Self::Unbound {
            method: Box::new(method),
        }
    }

    /// Create a string method binding
    pub fn bind_string(value: Arc<str>, method: impl Into<String>) -> Self {
        Self::String {
            value,
            method: method.into(),
        }
    }

    /// Create a list method binding
    pub fn bind_list(value: Arc<crate::pylist::PyList>, method: impl Into<String>) -> Self {
        Self::List {
            value,
            method: method.into(),
        }
    }

    /// Create a dict method binding
    pub fn bind_dict(value: Arc<crate::PyDict>, method: impl Into<String>) -> Self {
        Self::Dict {
            value,
            method: method.into(),
        }
    }

    /// Get the method name for builtin type methods
    pub fn method_name(&self) -> Option<&str> {
        match self {
            Self::String { method, .. } => Some(method),
            Self::List { method, .. } => Some(method),
            Self::Dict { method, .. } => Some(method),
            _ => None,
        }
    }

    /// Check if this is a string method
    pub fn is_string_method(&self) -> bool {
        matches!(self, Self::String { .. })
    }

    /// Check if this is a list method
    pub fn is_list_method(&self) -> bool {
        matches!(self, Self::List { .. })
    }

    /// Check if this is a dict method
    pub fn is_dict_method(&self) -> bool {
        matches!(self, Self::Dict { .. })
    }

    /// Get the underlying method
    pub fn method(&self) -> &PyValue {
        match self {
            Self::Instance { method, .. } => method,
            Self::Class { method, .. } => method,
            Self::Static { method } => method,
            Self::Unbound { method } => method,
            // For builtin type methods, there's no PyValue method - return a placeholder
            Self::String { .. } | Self::List { .. } | Self::Dict { .. } => {
                // These are handled specially in the interpreter
                // Return a static None as a placeholder
                static NONE: PyValue = PyValue::None;
                &NONE
            }
        }
    }

    /// Get the bound object (instance or class)
    pub fn bound_object(&self) -> Option<PyValue> {
        match self {
            Self::Instance { instance, .. } => Some(PyValue::Instance(Arc::clone(instance))),
            Self::Class { class, .. } => Some(PyValue::Type(Arc::clone(class))),
            Self::Static { .. } | Self::Unbound { .. } => None,
            Self::String { value, .. } => Some(PyValue::Str(Arc::clone(value))),
            Self::List { value, .. } => Some(PyValue::List(Arc::clone(value))),
            Self::Dict { value, .. } => Some(PyValue::Dict(Arc::clone(value))),
        }
    }

    /// Call the bound method with the given arguments.
    /// For instance methods, prepends the instance to the args.
    /// For class methods, prepends the class to the args.
    /// For static methods, passes args unchanged.
    ///
    /// Returns the full argument list to be passed to the underlying function.
    pub fn prepare_call_args(&self, args: Vec<PyValue>) -> Vec<PyValue> {
        match self {
            Self::Instance { instance, .. } => {
                // Prepend instance (self) to args
                let mut full_args = vec![PyValue::Instance(Arc::clone(instance))];
                full_args.extend(args);
                full_args
            }
            Self::Class { class, .. } => {
                // Prepend class (cls) to args
                let mut full_args = vec![PyValue::Type(Arc::clone(class))];
                full_args.extend(args);
                full_args
            }
            Self::Static { .. } | Self::Unbound { .. } => {
                // No binding, pass args as-is
                args
            }
            // For builtin type methods, args are passed as-is
            // The bound value is accessed via the BoundMethod variant
            Self::String { .. } | Self::List { .. } | Self::Dict { .. } => args,
        }
    }

    /// Get the function to call (for use with prepare_call_args)
    pub fn get_function(&self) -> &PyValue {
        self.method()
    }

    /// Check if this is an instance method
    pub fn is_instance_method(&self) -> bool {
        matches!(self, Self::Instance { .. })
    }

    /// Check if this is a class method
    pub fn is_class_method(&self) -> bool {
        matches!(self, Self::Class { .. })
    }

    /// Check if this is a static method
    pub fn is_static_method(&self) -> bool {
        matches!(self, Self::Static { .. })
    }

    /// Check if this is an unbound method
    pub fn is_unbound(&self) -> bool {
        matches!(self, Self::Unbound { .. })
    }

    /// Get the instance if this is an instance method
    pub fn get_instance(&self) -> Option<&Arc<PyInstance>> {
        match self {
            Self::Instance { instance, .. } => Some(instance),
            _ => None,
        }
    }

    /// Get the class if this is a class method
    pub fn get_class(&self) -> Option<&Arc<PyType>> {
        match self {
            Self::Class { class, .. } => Some(class),
            _ => None,
        }
    }
}

/// Method descriptor for instance methods
#[derive(Debug, Clone)]
pub struct MethodDescriptor {
    pub function: Box<PyValue>,
}

impl MethodDescriptor {
    pub fn new(function: PyValue) -> Self {
        Self {
            function: Box::new(function),
        }
    }
}

impl Descriptor for MethodDescriptor {
    fn get(&self, obj: Option<&PyValue>, _objtype: Option<&Arc<PyType>>) -> Option<PyValue> {
        match obj {
            Some(PyValue::Instance(instance)) => {
                // Bind to instance
                Some(PyValue::BoundMethod(BoundMethod::bind_instance(
                    *self.function.clone(),
                    Arc::clone(instance),
                )))
            }
            None => {
                // Unbound access (e.g., Class.method)
                Some(PyValue::BoundMethod(BoundMethod::unbound(*self.function.clone())))
            }
            _ => Some(*self.function.clone()),
        }
    }

    fn is_data_descriptor(&self) -> bool {
        false // Methods are non-data descriptors
    }
}

/// Class method descriptor
#[derive(Debug, Clone)]
pub struct ClassMethodDescriptor {
    pub function: Box<PyValue>,
}

impl ClassMethodDescriptor {
    pub fn new(function: PyValue) -> Self {
        Self {
            function: Box::new(function),
        }
    }
}

impl Descriptor for ClassMethodDescriptor {
    fn get(&self, _obj: Option<&PyValue>, objtype: Option<&Arc<PyType>>) -> Option<PyValue> {
        if let Some(class) = objtype {
            // Always bind to the class
            Some(PyValue::BoundMethod(BoundMethod::bind_class(
                *self.function.clone(),
                Arc::clone(class),
            )))
        } else {
            Some(*self.function.clone())
        }
    }

    fn is_data_descriptor(&self) -> bool {
        false // Class methods are non-data descriptors
    }
}

/// Static method descriptor
#[derive(Debug, Clone)]
pub struct StaticMethodDescriptor {
    pub function: Box<PyValue>,
}

impl StaticMethodDescriptor {
    pub fn new(function: PyValue) -> Self {
        Self {
            function: Box::new(function),
        }
    }
}

impl Descriptor for StaticMethodDescriptor {
    fn get(&self, _obj: Option<&PyValue>, _objtype: Option<&Arc<PyType>>) -> Option<PyValue> {
        // Static methods are never bound
        Some(PyValue::BoundMethod(BoundMethod::static_method(*self.function.clone())))
    }

    fn is_data_descriptor(&self) -> bool {
        false // Static methods are non-data descriptors
    }
}
pub struct SlotDescriptor {
    pub name: String,
    pub offset: usize,
}

impl SlotDescriptor {
    pub fn new(name: impl Into<String>, offset: usize) -> Self {
        Self {
            name: name.into(),
            offset,
        }
    }
}

impl Descriptor for SlotDescriptor {
    fn get(&self, _obj: Option<&PyValue>, _objtype: Option<&Arc<PyType>>) -> Option<PyValue> {
        // Would access the slot storage here
        None
    }

    fn set(&self, _obj: &PyValue, _value: PyValue) -> Option<bool> {
        // Would set the slot storage here
        Some(true)
    }

    fn delete(&self, _obj: &PyValue) -> Option<bool> {
        // Would delete from slot storage here
        Some(true)
    }

    fn is_data_descriptor(&self) -> bool {
        true
    }
}

/// Python class instance
#[derive(Debug, Clone)]
pub struct PyInstance {
    /// Object header
    pub header: PyObjectHeader,
    /// The class/type of this instance
    pub class: Arc<PyType>,
    /// Instance attributes (__dict__)
    pub dict: DashMap<String, PyValue>,
    /// Slot storage (for __slots__)
    pub slots_storage: Option<Vec<Option<PyValue>>>,
}

impl PyInstance {
    /// Create a new instance of a class
    pub fn new(class: Arc<PyType>) -> Self {
        let slots_storage = class.slots.as_ref().map(|slots| vec![None; slots.len()]);

        Self {
            header: PyObjectHeader::new(TypeTag::Object, ObjectFlags::NONE),
            class,
            dict: DashMap::new(),
            slots_storage,
        }
    }

    /// Get an attribute from the instance using descriptor protocol
    pub fn get_attr(&self, name: &str) -> Option<PyValue> {
        // Python attribute lookup order:
        // 1. Data descriptors from type's MRO
        // 2. Instance __dict__ or __slots__
        // 3. Non-data descriptors and other class attributes from MRO

        // Check for data descriptor in class
        if let Some(type_attr) = self.class.get_attr_from_mro(name) {
            // Check if it's a data descriptor (has __set__ or __delete__)
            if Self::is_data_descriptor(&type_attr) {
                // Call descriptor's __get__ method
                return Self::call_descriptor_get(&type_attr, Some(self), Some(&self.class));
            }
        }

        // Check __slots__ first if they exist
        if let Some(slot_value) = self.get_slot_attr(name) {
            return Some(slot_value);
        }

        // Check instance dict (only if __slots__ allows it or doesn't exist)
        if self.is_slot_allowed(name) {
            if let Some(value) = self.dict.get(name) {
                return Some(value.clone());
            }
        }

        // Check for non-data descriptors and other class attributes
        if let Some(type_attr) = self.class.get_attr_from_mro(name) {
            // Check if it's a descriptor
            if Self::is_descriptor(&type_attr) {
                return Self::call_descriptor_get(&type_attr, Some(self), Some(&self.class));
            }
            // Regular class attribute
            return Some(type_attr);
        }

        None
    }

    /// Check if a value is a data descriptor
    fn is_data_descriptor(value: &PyValue) -> bool {
        // In a full implementation, we'd check for __set__ or __delete__
        // For now, we check specific types
        matches!(value, PyValue::BoundMethod(_)) // Methods are descriptors but not data descriptors
    }

    /// Check if a value is a descriptor
    fn is_descriptor(value: &PyValue) -> bool {
        // Check if it has __get__ method
        matches!(value, PyValue::BoundMethod(_))
    }

    /// Call a descriptor's __get__ method
    fn call_descriptor_get(
        descriptor: &PyValue,
        obj: Option<&PyInstance>,
        _objtype: Option<&Arc<PyType>>,
    ) -> Option<PyValue> {
        // For now, handle built-in descriptor types
        match descriptor {
            // Regular functions become bound methods when accessed on instances
            PyValue::BoundMethod(BoundMethod::Unbound { method }) => {
                if let Some(instance) = obj {
                    Some(PyValue::BoundMethod(BoundMethod::bind_instance(
                        *method.clone(),
                        Arc::new(instance.clone()), // This is a bit inefficient, but works for now
                    )))
                } else {
                    Some(descriptor.clone())
                }
            }
            _ => Some(descriptor.clone()),
        }
    }

    /// Set an attribute on the instance
    pub fn set_attr(&self, name: impl Into<String>, value: PyValue) {
        let name = name.into();

        // Check if __slots__ restricts this
        if let Some(ref slots) = self.class.slots {
            if !slots.contains(&name) && !self.class.type_flags.has_dict {
                // In a real implementation, this would raise AttributeError
                // For now, we'll just ignore the assignment
                return;
            }

            // If the attribute is in __slots__, store it in slot storage
            if let Some(_slot_index) = slots.iter().position(|s| s == &name) {
                // We need mutable access to slots_storage, but we can't get it through &self
                // In a real implementation, this would use interior mutability
                // For now, we'll fall back to dict storage
            }
        }

        self.dict.insert(name, value);
    }

    /// Set an attribute by slot index (for __slots__)
    pub fn set_slot_attr(&mut self, slot_name: &str, value: PyValue) -> Result<(), String> {
        if let Some(ref slots) = self.class.slots {
            if let Some(slot_index) = slots.iter().position(|s| s == slot_name) {
                return if self.set_slot(slot_index, value) {
                    Ok(())
                } else {
                    Err(format!("Failed to set slot '{}'", slot_name))
                };
            }
        }
        Err(format!("'{}' object has no attribute '{}'", self.class.name, slot_name))
    }

    /// Get an attribute by slot index (for __slots__)
    pub fn get_slot_attr(&self, slot_name: &str) -> Option<PyValue> {
        if let Some(ref slots) = self.class.slots {
            if let Some(slot_index) = slots.iter().position(|s| s == slot_name) {
                return self.get_slot(slot_index);
            }
        }
        None
    }

    /// Check if an attribute name is allowed by __slots__
    pub fn is_slot_allowed(&self, name: &str) -> bool {
        match &self.class.slots {
            Some(slots) => slots.contains(&name.to_string()) || self.class.type_flags.has_dict,
            None => true, // No __slots__ restriction
        }
    }

    /// Get all slot names
    pub fn get_slot_names(&self) -> Option<&Vec<String>> {
        self.class.slots.as_ref()
    }

    /// Check if this instance uses __slots__
    pub fn has_slots(&self) -> bool {
        self.class.slots.is_some()
    }

    /// Delete an attribute from the instance
    pub fn del_attr(&self, name: &str) -> bool {
        self.dict.remove(name).is_some()
    }

    /// Check if instance has an attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.dict.contains_key(name) || self.class.get_attr_from_mro(name).is_some()
    }

    /// Get the class name
    pub fn class_name(&self) -> &str {
        &self.class.name
    }

    /// Get slot value by index
    pub fn get_slot(&self, index: usize) -> Option<PyValue> {
        self.slots_storage.as_ref()?.get(index)?.clone()
    }

    /// Set slot value by index
    pub fn set_slot(&mut self, index: usize, value: PyValue) -> bool {
        if let Some(ref mut storage) = self.slots_storage {
            if index < storage.len() {
                storage[index] = Some(value);
                return true;
            }
        }
        false
    }

    /// Delete slot value by index
    pub fn del_slot(&mut self, index: usize) -> bool {
        if let Some(ref mut storage) = self.slots_storage {
            if index < storage.len() {
                storage[index] = None;
                return true;
            }
        }
        false
    }
}

impl PyType {
    /// Create a new type
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            header: PyObjectHeader::new(TypeTag::Type, ObjectFlags::NONE),
            qualname: name.clone(),
            name,
            module: None,
            bases: Vec::new(),
            mro: Vec::new(),
            dict: DashMap::new(),
            type_flags: TypeFlags::default(),
            slots: None,
            metaclass: None,
        }
    }

    /// Create a new type with bases
    pub fn with_bases(name: impl Into<String>, bases: Vec<Arc<PyType>>) -> Self {
        let mut ty = Self::new(name);
        ty.bases = bases.clone();
        ty.compute_mro();
        ty
    }

    /// Create a new type with metaclass
    pub fn with_metaclass(name: impl Into<String>, metaclass: Arc<PyType>) -> Self {
        let mut ty = Self::new(name);
        ty.metaclass = Some(metaclass);
        ty
    }

    /// Set the module name
    pub fn with_module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Set __slots__
    pub fn with_slots(mut self, slots: Vec<String>) -> Self {
        self.slots = Some(slots);
        self.type_flags.has_slots = true;
        self
    }

    /// Get a dunder method from the type's MRO
    pub fn get_dunder(&self, name: &str) -> Option<PyValue> {
        let dunder_name = format!("__{}__", name);
        self.get_attr_from_mro(&dunder_name)
    }

    /// Check if type has a specific dunder method
    pub fn has_dunder(&self, name: &str) -> bool {
        self.get_dunder(name).is_some()
    }

    /// Get __add__ method
    pub fn get_add(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__add__")
    }

    /// Get __radd__ method
    pub fn get_radd(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__radd__")
    }

    /// Get __sub__ method
    pub fn get_sub(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__sub__")
    }

    /// Get __mul__ method
    pub fn get_mul(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__mul__")
    }

    /// Get __truediv__ method
    pub fn get_truediv(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__truediv__")
    }

    /// Get __floordiv__ method
    pub fn get_floordiv(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__floordiv__")
    }

    /// Get __mod__ method
    pub fn get_mod(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__mod__")
    }

    /// Get __pow__ method
    pub fn get_pow(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__pow__")
    }

    /// Get __neg__ method
    pub fn get_neg(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__neg__")
    }

    /// Get __pos__ method
    pub fn get_pos(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__pos__")
    }

    /// Get __invert__ method
    pub fn get_invert(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__invert__")
    }

    /// Get __eq__ method
    pub fn get_eq(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__eq__")
    }

    /// Get __ne__ method
    pub fn get_ne(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__ne__")
    }

    /// Get __lt__ method
    pub fn get_lt(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__lt__")
    }

    /// Get __le__ method
    pub fn get_le(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__le__")
    }

    /// Get __gt__ method
    pub fn get_gt(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__gt__")
    }

    /// Get __ge__ method
    pub fn get_ge(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__ge__")
    }

    /// Get __getitem__ method
    pub fn get_getitem(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__getitem__")
    }

    /// Get __setitem__ method
    pub fn get_setitem(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__setitem__")
    }

    /// Get __delitem__ method
    pub fn get_delitem(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__delitem__")
    }

    /// Get __contains__ method
    pub fn get_contains(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__contains__")
    }

    /// Get __len__ method
    pub fn get_len(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__len__")
    }

    /// Get __iter__ method
    pub fn get_iter(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__iter__")
    }

    /// Get __next__ method
    pub fn get_next(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__next__")
    }

    /// Get __call__ method
    pub fn get_call(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__call__")
    }

    /// Get __str__ method
    pub fn get_str(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__str__")
    }

    /// Get __repr__ method
    pub fn get_repr(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__repr__")
    }

    /// Get __hash__ method
    pub fn get_hash(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__hash__")
    }

    /// Get __bool__ method
    pub fn get_bool(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__bool__")
    }

    /// Get __getattr__ method
    pub fn get_getattr(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__getattr__")
    }

    /// Get __getattribute__ method
    pub fn get_getattribute(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__getattribute__")
    }

    /// Get __setattr__ method
    pub fn get_setattr(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__setattr__")
    }

    /// Get __delattr__ method
    pub fn get_delattr(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__delattr__")
    }

    /// Get __enter__ method (context manager)
    pub fn get_enter(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__enter__")
    }

    /// Get __exit__ method (context manager)
    pub fn get_exit(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__exit__")
    }

    /// Get __init__ method
    pub fn get_init(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__init__")
    }

    /// Get __new__ method
    pub fn get_new(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__new__")
    }

    /// Get __del__ method
    pub fn get_del(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__del__")
    }

    /// Get __init_subclass__ method
    pub fn get_init_subclass(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__init_subclass__")
    }

    /// Get __class_getitem__ method
    pub fn get_class_getitem(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__class_getitem__")
    }

    /// Get __get__ method (descriptor protocol)
    pub fn get_get(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__get__")
    }

    /// Get __set__ method (descriptor protocol)
    pub fn get_set(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__set__")
    }

    /// Get __delete__ method (descriptor protocol)
    pub fn get_delete(&self) -> Option<PyValue> {
        self.get_attr_from_mro("__delete__")
    }

    /// Check if this type is a descriptor
    pub fn is_descriptor(&self) -> bool {
        self.has_dunder("get")
    }

    /// Check if this type is a data descriptor
    pub fn is_data_descriptor(&self) -> bool {
        self.has_dunder("set") || self.has_dunder("delete")
    }

    /// Compute method resolution order using C3 linearization
    pub fn compute_mro(&mut self) {
        // C3 linearization algorithm
        // MRO = [C] + merge(MRO(B1), MRO(B2), ..., [B1, B2, ...])

        if self.bases.is_empty() {
            // No bases, MRO is just self (represented by empty vec since we don't have Arc<Self>)
            self.mro.clear();
            return;
        }

        // Collect base MROs
        let mut to_merge: Vec<Vec<Arc<PyType>>> = Vec::new();
        for base in &self.bases {
            let mut base_mro = vec![Arc::clone(base)];
            base_mro.extend(base.mro.iter().cloned());
            to_merge.push(base_mro);
        }

        // Add the list of bases
        to_merge.push(self.bases.clone());

        // Merge using C3
        self.mro = Self::c3_merge(to_merge);
    }

    /// C3 merge algorithm
    fn c3_merge(mut lists: Vec<Vec<Arc<PyType>>>) -> Vec<Arc<PyType>> {
        let mut result = Vec::new();

        loop {
            // Remove empty lists
            lists.retain(|l| !l.is_empty());

            if lists.is_empty() {
                break;
            }

            // Find a good head (not in tail of any list)
            let mut found = None;
            for list in &lists {
                let head = &list[0];
                let in_tail = lists
                    .iter()
                    .any(|l| l.len() > 1 && l[1..].iter().any(|t| Arc::ptr_eq(t, head)));

                if !in_tail {
                    found = Some(Arc::clone(head));
                    break;
                }
            }

            match found {
                Some(head) => {
                    result.push(Arc::clone(&head));
                    // Remove head from all lists
                    for list in &mut lists {
                        if !list.is_empty() && Arc::ptr_eq(&list[0], &head) {
                            list.remove(0);
                        }
                    }
                }
                None => {
                    // Inconsistent MRO - this shouldn't happen with valid Python classes
                    break;
                }
            }
        }

        result
    }

    /// Check if this type is a subtype of another
    pub fn is_subtype(&self, other: &PyType) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        self.mro.iter().any(|t| std::ptr::eq(t.as_ref(), other))
    }

    /// Get an attribute from the type's MRO
    pub fn get_attr_from_mro(&self, name: &str) -> Option<PyValue> {
        // Check own dict first
        if let Some(value) = self.dict.get(name) {
            return Some(value.clone());
        }

        // Check MRO
        for base in &self.mro {
            if let Some(value) = base.dict.get(name) {
                return Some(value.clone());
            }
        }

        None
    }

    /// Get an attribute from the type (legacy method)
    pub fn get_attr(&self, _name: &str) -> Option<PyTypeSlot> {
        // This is kept for backward compatibility
        None
    }

    /// Set an attribute on the type
    pub fn set_attr(&self, name: impl Into<String>, value: PyValue) {
        self.dict.insert(name.into(), value);
    }

    /// Create an instance of this type using __new__ and __init__ protocol
    pub fn create_instance_with_protocol(
        self: &Arc<Self>,
        _args: &[PyValue],
    ) -> crate::RuntimeResult<PyValue> {
        // Step 1: Call __new__ to create the instance
        let instance = if let Some(_new_method) = self.get_new() {
            // Call __new__(cls, *args)
            // For now, we'll just create a basic instance since we don't have full callable support
            // In a full implementation, this would:
            // 1. Call new_method with (cls, *args)
            // 2. Check if the returned object is an instance of cls
            // 3. If not, return it directly without calling __init__
            Arc::new(PyInstance::new(Arc::clone(self)))
        } else {
            // Default __new__ behavior - create instance of this type
            Arc::new(PyInstance::new(Arc::clone(self)))
        };

        // Step 2: Call __init__ to initialize the instance (if it exists and instance is of correct type)
        // Only call __init__ if the instance is actually of this type
        if instance.class.name == self.name {
            if let Some(_init_method) = self.get_init() {
                // Call __init__(self, *args)
                // For now, we'll skip the actual call since we don't have full callable support
                // In a full implementation, this would call the __init__ method with the instance and args
                // The return value of __init__ is ignored (should be None)
            }
        }

        Ok(PyValue::Instance(instance))
    }

    /// Handle __new__ returning different type
    /// This is used when __new__ returns an instance of a different class
    pub fn handle_new_different_type(
        new_result: PyValue,
        expected_type: &Arc<PyType>,
        _args: &[PyValue],
    ) -> crate::RuntimeResult<PyValue> {
        match &new_result {
            PyValue::Instance(instance) => {
                // Check if the instance is of the expected type
                if Arc::ptr_eq(&instance.class, expected_type) {
                    // Same type - call __init__ if it exists
                    if let Some(_init_method) = expected_type.get_init() {
                        // Call __init__(instance, *args)
                        // For now, skip the actual call
                    }
                } else {
                    // Different type - don't call __init__
                    // This is the correct Python behavior
                }
                Ok(new_result)
            }
            _ => {
                // __new__ returned a non-instance - return as-is
                // This is valid Python behavior (e.g., int.__new__ can return existing int objects)
                Ok(new_result)
            }
        }
    }

    /// Create an instance of this type (legacy method)
    pub fn create_instance(self: &Arc<Self>) -> PyInstance {
        PyInstance::new(Arc::clone(self))
    }
}

/// Super object for super() calls
#[derive(Debug)]
pub struct PySuper {
    /// The type to start searching from
    pub type_: Arc<PyType>,
    /// The object (instance) to bind to
    pub obj: Option<Arc<PyInstance>>,
    /// The type of the object
    pub obj_type: Option<Arc<PyType>>,
}

impl PySuper {
    /// Create a new super object
    pub fn new(type_: Arc<PyType>, obj: Option<Arc<PyInstance>>) -> Self {
        let obj_type = obj.as_ref().map(|o| Arc::clone(&o.class));
        Self {
            type_,
            obj,
            obj_type,
        }
    }

    /// Create a super object with explicit type and object type (for super(type, obj) form)
    pub fn new_with_types(
        type_: Arc<PyType>,
        obj: Option<Arc<PyInstance>>,
        obj_type: Option<Arc<PyType>>,
    ) -> Self {
        Self {
            type_,
            obj,
            obj_type,
        }
    }

    /// Create a super object for super() with no arguments
    /// This would normally be resolved at compile time to use the current class and instance
    pub fn new_no_args(current_class: Arc<PyType>, current_instance: Arc<PyInstance>) -> Self {
        Self {
            type_: current_class,
            obj: Some(current_instance.clone()),
            obj_type: Some(Arc::clone(&current_instance.class)),
        }
    }

    /// Get an attribute via super
    /// super(type_, obj) searches the MRO of obj's type, starting AFTER type_
    pub fn get_attr(&self, name: &str) -> Option<PyValue> {
        let obj_type = self.obj_type.as_ref()?;

        // Build the complete MRO including obj_type itself
        let mut complete_mro = vec![Arc::clone(obj_type)];
        complete_mro.extend(obj_type.mro.iter().cloned());

        // Find type_ in the complete MRO and search after it
        let mut found = false;
        for base in &complete_mro {
            if found {
                if let Some(value) = base.dict.get(name) {
                    // If we have an instance and the value is a method, bind it
                    if let Some(ref instance) = self.obj {
                        return Some(self.bind_method_if_needed(value.clone(), instance));
                    }
                    return Some(value.clone());
                }
            }
            if Arc::ptr_eq(base, &self.type_) {
                found = true;
            }
        }

        None
    }

    /// Bind method to instance if it's a method descriptor
    fn bind_method_if_needed(&self, value: PyValue, _instance: &Arc<PyInstance>) -> PyValue {
        // For now, just return the value as-is
        // In a full implementation, this would check if the value is a method descriptor
        // and bind it to the instance
        value
    }

    /// Check if this super object is valid
    pub fn is_valid(&self) -> bool {
        // super() requires either both type_ and obj_type, or neither
        match (&self.obj_type, &self.obj) {
            (Some(obj_type), Some(obj)) => {
                // Verify that obj is an instance of obj_type or a subtype
                Arc::ptr_eq(&obj.class, obj_type) || obj.class.is_subtype(obj_type)
            }
            (None, None) => true, // Unbound super
            _ => false,
        }
    }

    /// Get the next class in the MRO after the current type
    pub fn get_next_class(&self) -> Option<Arc<PyType>> {
        let obj_type = self.obj_type.as_ref()?;

        // Build the complete MRO including obj_type itself
        let mut complete_mro = vec![Arc::clone(obj_type)];
        complete_mro.extend(obj_type.mro.iter().cloned());

        // Find type_ in the complete MRO and return the next one
        for (i, base) in complete_mro.iter().enumerate() {
            if Arc::ptr_eq(base, &self.type_) {
                return complete_mro.get(i + 1).cloned();
            }
        }

        None
    }
}

/// Built-in type singletons
pub mod builtin_types {
    use super::*;
    use std::sync::OnceLock;

    static TYPE_NONE: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_BOOL: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_INT: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_FLOAT: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_STR: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_LIST: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_TUPLE: OnceLock<Arc<PyType>> = OnceLock::new();
    static TYPE_DICT: OnceLock<Arc<PyType>> = OnceLock::new();

    pub fn none_type() -> Arc<PyType> {
        TYPE_NONE.get_or_init(|| Arc::new(PyType::new("NoneType"))).clone()
    }

    pub fn bool_type() -> Arc<PyType> {
        TYPE_BOOL.get_or_init(|| Arc::new(PyType::new("bool"))).clone()
    }

    pub fn int_type() -> Arc<PyType> {
        TYPE_INT.get_or_init(|| Arc::new(PyType::new("int"))).clone()
    }

    pub fn float_type() -> Arc<PyType> {
        TYPE_FLOAT.get_or_init(|| Arc::new(PyType::new("float"))).clone()
    }

    pub fn str_type() -> Arc<PyType> {
        TYPE_STR.get_or_init(|| Arc::new(PyType::new("str"))).clone()
    }

    pub fn list_type() -> Arc<PyType> {
        TYPE_LIST.get_or_init(|| Arc::new(PyType::new("list"))).clone()
    }

    pub fn tuple_type() -> Arc<PyType> {
        TYPE_TUPLE.get_or_init(|| Arc::new(PyType::new("tuple"))).clone()
    }

    pub fn dict_type() -> Arc<PyType> {
        TYPE_DICT.get_or_init(|| Arc::new(PyType::new("dict"))).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_creation() {
        let ty = PyType::new("MyClass");
        assert_eq!(ty.name, "MyClass");
        assert_eq!(ty.header.type_tag(), TypeTag::Type);
    }

    #[test]
    fn test_builtin_types() {
        let int_type = builtin_types::int_type();
        assert_eq!(int_type.name, "int");

        let str_type = builtin_types::str_type();
        assert_eq!(str_type.name, "str");
    }

    #[test]
    fn test_instance_creation() {
        let class = Arc::new(PyType::new("MyClass"));
        let instance = class.create_instance();

        assert_eq!(instance.class_name(), "MyClass");
    }

    #[test]
    fn test_instance_attributes() {
        let class = Arc::new(PyType::new("MyClass"));
        let instance = PyInstance::new(Arc::clone(&class));

        instance.set_attr("x", PyValue::Int(42));

        let value = instance.get_attr("x").unwrap();
        if let PyValue::Int(v) = value {
            assert_eq!(v, 42);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_class_attribute_inheritance() {
        let class = Arc::new(PyType::new("MyClass"));
        class.set_attr("class_var", PyValue::Int(100));

        let instance = PyInstance::new(Arc::clone(&class));

        // Instance should see class attribute
        let value = instance.get_attr("class_var").unwrap();
        if let PyValue::Int(v) = value {
            assert_eq!(v, 100);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_mro_simple() {
        let base = Arc::new(PyType::new("Base"));
        let derived = PyType::with_bases("Derived", vec![Arc::clone(&base)]);

        assert_eq!(derived.mro.len(), 1);
        assert!(Arc::ptr_eq(&derived.mro[0], &base));
    }

    #[test]
    fn test_mro_diamond() {
        // Diamond inheritance: D -> B, C -> A
        let a = Arc::new(PyType::new("A"));
        let b = Arc::new(PyType::with_bases("B", vec![Arc::clone(&a)]));
        let c = Arc::new(PyType::with_bases("C", vec![Arc::clone(&a)]));
        let d = PyType::with_bases("D", vec![Arc::clone(&b), Arc::clone(&c)]);

        // MRO should be: B, C, A (C3 linearization)
        assert_eq!(d.mro.len(), 3);
        assert_eq!(d.mro[0].name, "B");
        assert_eq!(d.mro[1].name, "C");
        assert_eq!(d.mro[2].name, "A");
    }

    #[test]
    fn test_new_init_protocol() {
        let class = Arc::new(PyType::new("TestClass"));

        // Test basic instance creation
        let result = class.create_instance_with_protocol(&[]).unwrap();
        match result {
            PyValue::Instance(instance) => {
                assert_eq!(instance.class.name, "TestClass");
            }
            _ => panic!("Expected instance"),
        }
    }

    #[test]
    fn test_new_init_with_args() {
        let class = Arc::new(PyType::new("TestClass"));

        // Test instance creation with arguments
        let args = vec![PyValue::Int(42), PyValue::Str(Arc::from("test"))];
        let result = class.create_instance_with_protocol(&args).unwrap();
        match result {
            PyValue::Instance(instance) => {
                assert_eq!(instance.class.name, "TestClass");
            }
            _ => panic!("Expected instance"),
        }
    }

    #[test]
    fn test_new_different_type() {
        let class = Arc::new(PyType::new("TestClass"));
        let other_class = Arc::new(PyType::new("OtherClass"));
        let other_instance = Arc::new(PyInstance::new(Arc::clone(&other_class)));

        // Test when __new__ returns different type
        let result =
            PyType::handle_new_different_type(PyValue::Instance(other_instance), &class, &[])
                .unwrap();

        match result {
            PyValue::Instance(instance) => {
                assert_eq!(instance.class.name, "OtherClass");
            }
            _ => panic!("Expected instance"),
        }
    }

    #[test]
    fn test_is_subtype() {
        let base = Arc::new(PyType::new("Base"));
        let derived = Arc::new(PyType::with_bases("Derived", vec![Arc::clone(&base)]));

        assert!(derived.is_subtype(&base));
        assert!(!base.is_subtype(&derived));
    }

    #[test]
    fn test_super_resolution() {
        let base = Arc::new(PyType::new("Base"));
        base.set_attr("method", PyValue::Str(Arc::from("base_method")));

        let derived = Arc::new(PyType::with_bases("Derived", vec![Arc::clone(&base)]));
        derived.set_attr("method", PyValue::Str(Arc::from("derived_method")));

        let instance = Arc::new(PyInstance::new(Arc::clone(&derived)));

        // super(Derived, instance) should find base's method
        // because it searches AFTER Derived in the MRO
        let super_obj = PySuper::new(Arc::clone(&derived), Some(Arc::clone(&instance)));
        let value = super_obj.get_attr("method").unwrap();

        if let PyValue::Str(s) = value {
            assert_eq!(&*s, "base_method");
        } else {
            panic!("Expected Str");
        }
    }

    #[test]
    fn test_super_no_args() {
        let base = Arc::new(PyType::new("Base"));
        base.set_attr("method", PyValue::Str(Arc::from("base_method")));

        let derived = Arc::new(PyType::with_bases("Derived", vec![Arc::clone(&base)]));
        derived.set_attr("method", PyValue::Str(Arc::from("derived_method")));

        let instance = Arc::new(PyInstance::new(Arc::clone(&derived)));

        // super() with no args (simulating current class and instance)
        let super_obj = PySuper::new_no_args(Arc::clone(&derived), Arc::clone(&instance));
        let value = super_obj.get_attr("method").unwrap();

        if let PyValue::Str(s) = value {
            assert_eq!(&*s, "base_method");
        } else {
            panic!("Expected Str");
        }
    }

    #[test]
    fn test_super_validity() {
        let base = Arc::new(PyType::new("Base"));
        let derived = Arc::new(PyType::with_bases("Derived", vec![Arc::clone(&base)]));
        let instance = Arc::new(PyInstance::new(Arc::clone(&derived)));

        // Valid super object
        let super_obj = PySuper::new(Arc::clone(&derived), Some(Arc::clone(&instance)));
        assert!(super_obj.is_valid());

        // Valid unbound super
        let unbound_super = PySuper::new_with_types(Arc::clone(&derived), None, None);
        assert!(unbound_super.is_valid());
    }

    #[test]
    fn test_super_next_class() {
        let a = Arc::new(PyType::new("A"));
        let b = Arc::new(PyType::with_bases("B", vec![Arc::clone(&a)]));
        let c = Arc::new(PyType::with_bases("C", vec![Arc::clone(&b)]));

        let instance = Arc::new(PyInstance::new(Arc::clone(&c)));
        let super_obj = PySuper::new(Arc::clone(&c), Some(Arc::clone(&instance)));

        // Next class after C should be B
        let next = super_obj.get_next_class().unwrap();
        assert_eq!(next.name, "B");

        // Test super from B
        let super_b = PySuper::new(Arc::clone(&b), Some(Arc::clone(&instance)));
        let next_b = super_b.get_next_class().unwrap();
        assert_eq!(next_b.name, "A");
    }

    #[test]
    fn test_property_descriptor() {
        use crate::types::PropertyDescriptor;

        // Create a getter function
        let getter = PyValue::Str(Arc::from("getter_function"));
        let setter = PyValue::Str(Arc::from("setter_function"));
        let deleter = PyValue::Str(Arc::from("deleter_function"));

        // Test property creation
        let prop = PropertyDescriptor::new(Some(getter.clone()), None, None);
        assert!(prop.is_readonly());
        assert!(!prop.is_writeonly());
        assert!(!prop.is_data_descriptor()); // No setter or deleter

        // Test property with setter
        let prop_with_setter = prop.setter(setter.clone());
        assert!(!prop_with_setter.is_readonly());
        assert!(prop_with_setter.is_data_descriptor()); // Has setter

        // Test property with deleter
        let prop_with_deleter = prop_with_setter.deleter(deleter.clone());
        assert!(prop_with_deleter.is_data_descriptor()); // Has setter and deleter

        // Test docstring
        let prop_with_doc = prop.with_doc("Test property".to_string());
        assert_eq!(prop_with_doc.get_doc(), Some("Test property"));
    }

    #[test]
    fn test_property_getter_only() {
        use crate::types::PropertyDescriptor;

        let getter = PyValue::Str(Arc::from("getter"));
        let prop = PropertyDescriptor::getter(getter.clone());

        assert!(prop.is_readonly());
        assert!(prop.fget.is_some());
        assert!(prop.fset.is_none());
        assert!(prop.fdel.is_none());
    }

    #[test]
    fn test_property_descriptor_protocol() {
        use crate::types::{Descriptor, PropertyDescriptor};

        let getter = PyValue::Str(Arc::from("getter"));
        let setter = PyValue::Str(Arc::from("setter"));

        let prop = PropertyDescriptor::new(Some(getter.clone()), Some(setter.clone()), None);

        // Test descriptor protocol
        let class = Arc::new(PyType::new("TestClass"));
        let instance = Arc::new(PyInstance::new(Arc::clone(&class)));

        // Test get
        let result = prop.get(Some(&PyValue::Instance(instance.clone())), Some(&class));
        assert!(result.is_some());

        // Test set
        let set_result = prop.set(&PyValue::Instance(instance), PyValue::Int(42));
        assert_eq!(set_result, Some(true)); // Has setter

        // Test is_data_descriptor
        assert!(prop.is_data_descriptor()); // Has setter
    }

    #[test]
    fn test_slots_basic() {
        let slots = vec!["x".to_string(), "y".to_string()];
        let class = Arc::new(PyType::new("Point").with_slots(slots));
        let instance = PyInstance::new(Arc::clone(&class));

        // Test that instance has slots
        assert!(instance.has_slots());
        assert_eq!(instance.get_slot_names().unwrap().len(), 2);

        // Test slot attribute access
        assert!(instance.is_slot_allowed("x"));
        assert!(instance.is_slot_allowed("y"));
        assert!(!instance.is_slot_allowed("z")); // Not in __slots__
    }

    #[test]
    fn test_slots_attribute_restriction() {
        let slots = vec!["allowed".to_string()];
        let class = Arc::new(PyType::new("RestrictedClass").with_slots(slots));
        let instance = PyInstance::new(Arc::clone(&class));

        // Setting allowed attribute should work
        instance.set_attr("allowed", PyValue::Int(42));

        // Setting disallowed attribute should be ignored (in real Python, this would raise AttributeError)
        instance.set_attr("disallowed", PyValue::Int(99));

        // Check that allowed attribute was set
        let allowed_value = instance.get_attr("allowed");
        assert!(allowed_value.is_some());

        // Check that disallowed attribute was not set
        let disallowed_value = instance.get_attr("disallowed");
        assert!(disallowed_value.is_none());
    }

    #[test]
    fn test_slots_vs_dict() {
        // Class without __slots__ (uses __dict__)
        let normal_class = Arc::new(PyType::new("NormalClass"));
        let normal_instance = PyInstance::new(Arc::clone(&normal_class));

        assert!(!normal_instance.has_slots());
        assert!(normal_instance.is_slot_allowed("any_name"));

        // Class with __slots__
        let slots_class = Arc::new(PyType::new("SlotsClass").with_slots(vec!["x".to_string()]));
        let slots_instance = PyInstance::new(Arc::clone(&slots_class));

        assert!(slots_instance.has_slots());
        assert!(slots_instance.is_slot_allowed("x"));
        assert!(!slots_instance.is_slot_allowed("y"));
    }

    #[test]
    fn test_slots_storage() {
        let slots = vec!["x".to_string(), "y".to_string()];
        let class = Arc::new(PyType::new("Point").with_slots(slots));
        let mut instance = PyInstance::new(Arc::clone(&class));

        // Test setting slot attributes
        assert!(instance.set_slot_attr("x", PyValue::Int(10)).is_ok());
        assert!(instance.set_slot_attr("y", PyValue::Int(20)).is_ok());

        // Test getting slot attributes
        let x_value = instance.get_slot_attr("x");
        assert!(x_value.is_some());
        if let Some(PyValue::Int(x)) = x_value {
            assert_eq!(x, 10);
        }

        // Test setting invalid slot
        assert!(instance.set_slot_attr("z", PyValue::Int(30)).is_err());
    }
}
