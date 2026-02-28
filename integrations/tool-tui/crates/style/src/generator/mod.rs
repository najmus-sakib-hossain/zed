//! CSS Generator module
//!
//! Generates CSS rules from extracted class names using the style engine.
//! Supports parallel processing for large batches and arena allocation
//! for memory-efficient generation.

pub mod arena;

use cssparser::serialize_identifier;
use rayon::prelude::*;

#[cfg(feature = "arena-alloc")]
use arena::generate_css_batch_arena;

use crate::core::{
    AppState, group::GroupRegistry, properties_layer_present, set_properties_layer_present,
};

#[allow(dead_code)]
pub fn generate_css_into<'a, I>(buf: &mut Vec<u8>, classes: I, groups: &mut GroupRegistry)
where
    I: IntoIterator<Item = &'a String>,
{
    // Use arena allocation for large batches if feature is enabled
    #[cfg(feature = "arena-alloc")]
    {
        let classes_vec: Vec<&String> = classes.into_iter().collect();
        if classes_vec.len() > 50 {
            generate_css_batch_arena(buf, classes_vec, groups);
            return;
        }
        // Fall through to regular path for small batches
        generate_css_into_regular(buf, classes_vec, groups);
        return;
    }

    #[cfg(not(feature = "arena-alloc"))]
    generate_css_into_regular(buf, classes, groups);
}

#[allow(dead_code)]
fn generate_css_into_regular<'a, I>(buf: &mut Vec<u8>, classes: I, groups: &mut GroupRegistry)
where
    I: IntoIterator<Item = &'a String>,
{
    let engine_opt = std::panic::catch_unwind(AppState::engine).ok();
    if let Some(engine) = engine_opt {
        let collected: Vec<&String> = classes.into_iter().collect();
        if buf.is_empty() && !properties_layer_present() {
            let props = engine.property_at_rules();
            if !props.is_empty() {
                buf.extend_from_slice(props.as_bytes());
                set_properties_layer_present();
            }
            let (root_vars, dark_vars) = engine.generate_color_vars_for(collected.iter().copied());
            if !root_vars.is_empty() {
                buf.extend_from_slice(root_vars.as_bytes());
            }
            if !dark_vars.is_empty() {
                buf.extend_from_slice(dark_vars.as_bytes());
            }
        }

        // Parallelize CSS generation when we have enough classes to make it worthwhile
        // Note: We can only parallelize the engine.css_for_class() lookups, not group operations
        if collected.len() > 100 {
            // First, handle group tokens sequentially (these require mutable access)
            let mut non_group_classes = Vec::with_capacity(collected.len());

            for class in &collected {
                if groups.is_internal_token(class) {
                    continue;
                }
                if let Some(alias_css) = groups.generate_css_for(class, engine) {
                    buf.extend_from_slice(alias_css.as_bytes());
                    if !alias_css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                } else {
                    // Not a group token, add to parallel processing queue
                    non_group_classes.push(*class);
                }
            }

            // Now parallelize the engine lookups for non-group classes
            if !non_group_classes.is_empty() {
                let css_chunks: Vec<Vec<u8>> = non_group_classes
                    .par_iter()
                    .map(|class| {
                        let mut chunk = Vec::with_capacity(128);
                        let mut escaped = String::with_capacity(64);

                        if let Some(css) = engine.css_for_class(class) {
                            chunk.extend_from_slice(css.as_bytes());
                            if !css.ends_with('\n') {
                                chunk.push(b'\n');
                            }
                        } else {
                            chunk.push(b'.');
                            serialize_identifier(class, &mut escaped).unwrap();
                            chunk.extend_from_slice(escaped.as_bytes());
                            chunk.extend_from_slice(b" {}\n");
                        }

                        chunk
                    })
                    .collect();

                // Combine results sequentially
                for chunk in css_chunks {
                    buf.extend_from_slice(&chunk);
                }
            }
        } else {
            // For small numbers of classes, use sequential processing
            let mut escaped = String::with_capacity(64);
            for class in collected {
                if groups.is_internal_token(class) {
                    continue;
                }
                if let Some(alias_css) = groups.generate_css_for(class, engine) {
                    buf.extend_from_slice(alias_css.as_bytes());
                    if !alias_css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                    continue;
                }
                if let Some(css) = engine.css_for_class(class) {
                    buf.extend_from_slice(css.as_bytes());
                    if !css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                } else {
                    buf.push(b'.');
                    escaped.clear();
                    serialize_identifier(class, &mut escaped).unwrap();
                    buf.extend_from_slice(escaped.as_bytes());
                    buf.extend_from_slice(b" {}\n");
                }
            }
        }
    } else {
        let mut escaped = String::with_capacity(64);
        for class in classes {
            if groups.is_internal_token(class) {
                continue;
            }
            buf.push(b'.');
            escaped.clear();
            serialize_identifier(class, &mut escaped).unwrap();
            buf.extend_from_slice(escaped.as_bytes());
            buf.extend_from_slice(b" {}\n");
        }
    }
}

pub fn generate_class_rules_only<'a, I>(buf: &mut Vec<u8>, classes: I, groups: &mut GroupRegistry)
where
    I: IntoIterator<Item = &'a String>,
{
    // Use arena allocation for large batches if feature is enabled
    #[cfg(feature = "arena-alloc")]
    {
        let classes_vec: Vec<&String> = classes.into_iter().collect();
        if classes_vec.len() > 50 {
            // Arena path doesn't have separate "rules only" - just use regular arena
            generate_css_batch_arena(buf, classes_vec, groups);
            return;
        }
        // Fall through to regular path for small batches
        generate_class_rules_only_regular(buf, classes_vec, groups);
        return;
    }

    #[cfg(not(feature = "arena-alloc"))]
    generate_class_rules_only_regular(buf, classes, groups);
}

fn generate_class_rules_only_regular<'a, I>(
    buf: &mut Vec<u8>,
    classes: I,
    groups: &mut GroupRegistry,
) where
    I: IntoIterator<Item = &'a String>,
{
    use cssparser::serialize_identifier;
    let engine_opt = std::panic::catch_unwind(AppState::engine).ok();
    if let Some(engine) = engine_opt {
        let collected: Vec<&String> = classes.into_iter().collect();

        // Parallelize CSS generation when we have enough classes to make it worthwhile
        // Note: We can only parallelize the engine.css_for_class() lookups, not group operations
        if collected.len() > 100 {
            // First, handle group tokens sequentially (these require mutable access)
            let mut non_group_classes = Vec::with_capacity(collected.len());

            for class in &collected {
                if groups.is_internal_token(class) {
                    continue;
                }
                if groups.is_util_member(class) {
                    continue;
                }
                if let Some(alias_css) = groups.generate_css_for(class, engine) {
                    buf.extend_from_slice(alias_css.as_bytes());
                    if !alias_css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                } else {
                    // Not a group token, add to parallel processing queue
                    non_group_classes.push(*class);
                }
            }

            // Now parallelize the engine lookups for non-group classes
            if !non_group_classes.is_empty() {
                let css_chunks: Vec<Vec<u8>> = non_group_classes
                    .par_iter()
                    .map(|class| {
                        let mut chunk = Vec::with_capacity(128);
                        let mut escaped = String::with_capacity(64);

                        if let Some(css) = engine.css_for_class(class) {
                            chunk.extend_from_slice(css.as_bytes());
                            if !css.ends_with('\n') {
                                chunk.push(b'\n');
                            }
                        } else {
                            chunk.push(b'.');
                            serialize_identifier(class, &mut escaped).unwrap();
                            chunk.extend_from_slice(escaped.as_bytes());
                            chunk.extend_from_slice(b" {}\n");
                        }

                        chunk
                    })
                    .collect();

                // Combine results sequentially
                for chunk in css_chunks {
                    buf.extend_from_slice(&chunk);
                }
            }
        } else {
            // For small numbers of classes, use sequential processing
            let mut escaped = String::with_capacity(64);
            for class in collected {
                if groups.is_internal_token(class) {
                    continue;
                }
                if groups.is_util_member(class) {
                    continue;
                }
                if let Some(alias_css) = groups.generate_css_for(class, engine) {
                    buf.extend_from_slice(alias_css.as_bytes());
                    if !alias_css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                    continue;
                }
                if let Some(css) = engine.css_for_class(class) {
                    buf.extend_from_slice(css.as_bytes());
                    if !css.ends_with('\n') {
                        buf.push(b'\n');
                    }
                } else {
                    buf.push(b'.');
                    escaped.clear();
                    serialize_identifier(class, &mut escaped).unwrap();
                    buf.extend_from_slice(escaped.as_bytes());
                    buf.extend_from_slice(b" {}\n");
                }
            }
        }
    } else {
        let mut escaped = String::with_capacity(64);
        for class in classes {
            if groups.is_internal_token(class) {
                continue;
            }
            buf.push(b'.');
            escaped.clear();
            serialize_identifier(class, &mut escaped).unwrap();
            buf.extend_from_slice(escaped.as_bytes());
            buf.extend_from_slice(b" {}\n");
        }
    }
}
