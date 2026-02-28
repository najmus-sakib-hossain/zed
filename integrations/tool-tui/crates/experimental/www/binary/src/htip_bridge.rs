//! # HTIP Bridge: Deserializer → DOM
//!
//! Connects dx-binary operations to dx-dom rendering.
//! This is the critical path: Binary → WASM → Browser.

#[cfg(target_arch = "wasm32")]
use crate::deserializer::HtipStream;
#[cfg(target_arch = "wasm32")]
use crate::opcodes::{Operation, PropertyValue};
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, Element, HtmlElement, HtmlTemplateElement, Node, Text};

/// The HTIP Application Engine
///
/// Manages template cache and live instances
#[cfg(target_arch = "wasm32")]
pub struct HtipEngine {
    /// Template cache: template_id -> HtmlTemplateElement
    templates: HashMap<u16, HtmlTemplateElement>,
    /// Instance cache: instance_id -> HtmlElement
    instances: HashMap<u32, HtmlElement>,
    /// String table from HTIP payload
    strings: Vec<String>,
    /// Document reference
    document: Document,
}

#[cfg(target_arch = "wasm32")]
impl HtipEngine {
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        Ok(Self {
            templates: HashMap::new(),
            instances: HashMap::new(),
            strings: Vec::new(),
            document,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            templates: HashMap::new(),
            instances: HashMap::new(),
            strings: Vec::new(),
        })
    }

    /// Process an HTIP stream and apply all operations to the DOM
    pub fn process_stream(
        &mut self,
        stream: &HtipStream,
        root: &HtmlElement,
    ) -> Result<(), String> {
        // Load string table
        self.strings.clear();
        for i in 0.. {
            match stream.get_string(i) {
                Some(s) => self.strings.push(s.to_string()),
                None => break,
            }
        }

        // Process each operation
        for op in stream.operations() {
            self.apply_operation(op, root)?;
        }

        Ok(())
    }

    /// Apply a single HTIP operation
    fn apply_operation(&mut self, op: &Operation, root: &HtmlElement) -> Result<(), String> {
        match op {
            Operation::TemplateDef(def) => {
                self.register_template(def.id, def.html_string_id)?;
            }
            Operation::Instantiate(inst) => {
                self.instantiate_template(
                    inst.instance_id,
                    inst.template_id,
                    inst.parent_id,
                    root,
                )?;
            }
            Operation::PatchText(patch) => {
                self.patch_text(patch.instance_id, patch.slot_id, patch.string_id)?;
            }
            Operation::PatchAttr(patch) => {
                self.patch_attr(
                    patch.instance_id,
                    patch.slot_id,
                    patch.attr_name_id,
                    patch.value_id,
                )?;
            }
            Operation::PatchClassToggle(toggle) => {
                self.toggle_class(toggle.instance_id, toggle.class_name_id, toggle.enabled)?;
            }
            Operation::AttachEvent(event) => {
                // Event handling requires JS callbacks - skip for now
                web_sys::console::warn_1(
                    &format!("AttachEvent not yet implemented: {:?}", event).into(),
                );
            }
            Operation::RemoveNode(remove) => {
                self.remove_node(remove.instance_id)?;
            }
            Operation::BatchStart(_) => {
                // Batching is transparent - no action needed
            }
            Operation::BatchCommit(_) => {
                // Batching is transparent - no action needed
            }
            Operation::SetProperty(prop) => {
                self.set_property(prop.instance_id, prop.prop_name_id, &prop.value)?;
            }
            Operation::AppendChild(append) => {
                self.append_child(append.parent_id, append.child_id)?;
            }
        }
        Ok(())
    }

    /// Register a template in the cache
    #[cfg(target_arch = "wasm32")]
    fn register_template(&mut self, template_id: u16, html_string_id: u32) -> Result<(), String> {
        let html = self.get_string(html_string_id)?;

        // Create a <template> element
        let template = self
            .document
            .create_element("template")
            .map_err(|e| format!("Failed to create template: {:?}", e))?;

        let template: HtmlTemplateElement =
            template.dyn_into().map_err(|_| "Failed to cast to HtmlTemplateElement")?;

        // Set innerHTML
        template.set_inner_html(html);

        // Cache it
        self.templates.insert(template_id, template);

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn register_template(&mut self, template_id: u16, _html_string_id: u32) -> Result<(), String> {
        self.templates.insert(template_id, ()); // Mock
        Ok(())
    }

    /// Instantiate a template using cloneNode
    #[cfg(target_arch = "wasm32")]
    fn instantiate_template(
        &mut self,
        instance_id: u32,
        template_id: u16,
        parent_id: u32,
        root: &HtmlElement,
    ) -> Result<(), String> {
        let template = self
            .templates
            .get(&template_id)
            .ok_or_else(|| format!("Template {} not found", template_id))?;

        // Clone the template content
        let content = template.content();
        let clone = content
            .clone_node_with_deep(true)
            .map_err(|e| format!("Failed to clone: {:?}", e))?;

        // Get the first element child
        let element = content
            .first_element_child()
            .ok_or("Template has no element child")?
            .clone_node_with_deep(true)
            .map_err(|e| format!("Failed to clone element: {:?}", e))?;

        // Cast to HtmlElement
        let element =
            element.dyn_into::<HtmlElement>().map_err(|_| "Failed to cast to HtmlElement")?;

        // Find parent (or use root)
        let parent = if parent_id == 0 {
            root.clone()
        } else {
            self.instances
                .get(&parent_id)
                .ok_or_else(|| format!("Parent {} not found", parent_id))?
                .clone()
        };

        // Append to parent
        parent
            .append_child(&element)
            .map_err(|e| format!("Failed to append: {:?}", e))?;

        // Cache the instance
        self.instances.insert(instance_id, element);

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn instantiate_template(
        &mut self,
        instance_id: u32,
        _template_id: u16,
        _parent_id: u32,
        _root: &(),
    ) -> Result<(), String> {
        self.instances.insert(instance_id, ()); // Mock
        Ok(())
    }

    /// Patch text content at a slot
    #[cfg(target_arch = "wasm32")]
    fn patch_text(&mut self, instance_id: u32, slot_id: u16, string_id: u32) -> Result<(), String> {
        let instance = self
            .instances
            .get(&instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let text = self.get_string(string_id)?;

        // Find the slot (comment node with <!--SLOT_N-->)
        let slot_marker = format!("SLOT_{}", slot_id);
        if let Some(text_node) = find_slot_text_node(instance, &slot_marker) {
            text_node.set_text_content(Some(text));
        } else {
            // Fallback: set text content of the element itself
            instance.set_text_content(Some(text));
        }

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn patch_text(
        &mut self,
        _instance_id: u32,
        _slot_id: u16,
        _string_id: u32,
    ) -> Result<(), String> {
        Ok(()) // Mock
    }

    /// Patch an attribute
    #[cfg(target_arch = "wasm32")]
    fn patch_attr(
        &mut self,
        instance_id: u32,
        _slot_id: u16,
        attr_name_id: u32,
        value_id: u32,
    ) -> Result<(), String> {
        let instance = self
            .instances
            .get(&instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let attr_name = self.get_string(attr_name_id)?;
        let value = self.get_string(value_id)?;

        instance
            .set_attribute(attr_name, value)
            .map_err(|e| format!("Failed to set attribute: {:?}", e))?;

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn patch_attr(&mut self, _: u32, _: u16, _: u32, _: u32) -> Result<(), String> {
        Ok(()) // Mock
    }

    /// Toggle a CSS class
    #[cfg(target_arch = "wasm32")]
    fn toggle_class(
        &mut self,
        instance_id: u32,
        class_name_id: u32,
        enabled: bool,
    ) -> Result<(), String> {
        let instance = self
            .instances
            .get(&instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let class_name = self.get_string(class_name_id)?;

        let class_list = instance.class_list();
        if enabled {
            class_list
                .add_1(class_name)
                .map_err(|e| format!("Failed to add class: {:?}", e))?;
        } else {
            class_list
                .remove_1(class_name)
                .map_err(|e| format!("Failed to remove class: {:?}", e))?;
        }

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn toggle_class(&mut self, _: u32, _: u32, _: bool) -> Result<(), String> {
        Ok(()) // Mock
    }

    /// Remove a node
    #[cfg(target_arch = "wasm32")]
    fn remove_node(&mut self, instance_id: u32) -> Result<(), String> {
        if let Some(instance) = self.instances.remove(&instance_id) {
            if let Some(parent) = instance.parent_node() {
                parent
                    .remove_child(&instance)
                    .map_err(|e| format!("Failed to remove: {:?}", e))?;
            }
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remove_node(&mut self, instance_id: u32) -> Result<(), String> {
        self.instances.remove(&instance_id);
        Ok(())
    }

    /// Set a property
    #[cfg(target_arch = "wasm32")]
    fn set_property(
        &mut self,
        instance_id: u32,
        prop_name_id: u32,
        value: &PropertyValue,
    ) -> Result<(), String> {
        let instance = self
            .instances
            .get(&instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let prop_name = self.get_string(prop_name_id)?;

        // Use js_sys::Reflect to set property
        let js_value = match value {
            PropertyValue::String(id) => {
                let s = self.get_string(*id)?;
                JsValue::from_str(s)
            }
            PropertyValue::Number(n) => JsValue::from_f64(*n),
            PropertyValue::Boolean(b) => JsValue::from_bool(*b),
            PropertyValue::Null => JsValue::NULL,
        };

        js_sys::Reflect::set(instance, &JsValue::from_str(prop_name), &js_value)
            .map_err(|e| format!("Failed to set property: {:?}", e))?;

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_property(&mut self, _: u32, _: u32, _: &PropertyValue) -> Result<(), String> {
        Ok(()) // Mock
    }

    /// Append child
    #[cfg(target_arch = "wasm32")]
    fn append_child(&mut self, parent_id: u32, child_id: u32) -> Result<(), String> {
        let parent = self
            .instances
            .get(&parent_id)
            .ok_or_else(|| format!("Parent {} not found", parent_id))?
            .clone();

        let child = self
            .instances
            .get(&child_id)
            .ok_or_else(|| format!("Child {} not found", child_id))?;

        parent
            .append_child(child)
            .map_err(|e| format!("Failed to append child: {:?}", e))?;

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn append_child(&mut self, _: u32, _: u32) -> Result<(), String> {
        Ok(()) // Mock
    }

    /// Helper: Get string from table
    fn get_string(&self, id: u32) -> Result<&str, String> {
        self.strings
            .get(id as usize)
            .map(|s| s.as_str())
            .ok_or_else(|| format!("String {} not found", id))
    }
}

/// Helper: Find text node after slot marker comment
#[cfg(target_arch = "wasm32")]
fn find_slot_text_node(element: &Element, slot_marker: &str) -> Option<Text> {
    let children = element.child_nodes();
    for i in 0..children.length() {
        if let Some(node) = children.get(i) {
            // Check if it's a comment node with our marker
            if node.node_type() == Node::COMMENT_NODE {
                if let Some(comment_text) = node.text_content() {
                    if comment_text.contains(slot_marker) {
                        // Return next sibling if it's a text node
                        if let Some(next) = node.next_sibling() {
                            if next.node_type() == Node::TEXT_NODE {
                                return next.dyn_into::<Text>().ok();
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// WASM Exports
// ============================================================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_htip_engine() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"dx-morph: HTIP Bridge Initialized".into());
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // HtipEngine tests require WASM environment
        // Use wasm-bindgen-test for browser-based testing
    }
}
