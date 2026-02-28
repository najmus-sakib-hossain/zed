//! Derive macros for dx-serializer
//!
//! This crate provides procedural macros for generating compile-time quantum layouts
//! and compile-time serialization.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, Type, parse_macro_input};

/// Derive macro for QuantumLayout
///
/// Generates compile-time field offsets and accessor methods for zero-copy deserialization.
#[proc_macro_derive(QuantumLayout)]
pub fn derive_quantum_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "QuantumLayout only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "QuantumLayout only supports structs")
                .to_compile_error()
                .into();
        }
    };

    // Calculate field offsets and categorize fields
    let mut fixed_fields = Vec::new();
    let mut slot_fields = Vec::new();
    let mut current_offset = 0usize;
    let header_size = 4usize; // Magic + version + flags

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Determine if this is a fixed-size or variable-size field
        if is_fixed_size_type(field_type) {
            let size = get_type_size_from_type(field_type);
            fixed_fields.push((field_name.clone(), field_type.clone(), current_offset, size));
            current_offset += size;
        } else {
            // Variable-size field (String, Vec, etc.) - uses a slot
            slot_fields.push((field_name.clone(), field_type.clone()));
        }
    }

    let fixed_size = current_offset;
    let slot_count = slot_fields.len();

    // Generate offset constants for fixed fields
    let fixed_offset_consts = fixed_fields.iter().map(|(name, _ty, offset, _size)| {
        let const_name =
            syn::Ident::new(&format!("{}_OFFSET", name.to_string().to_uppercase()), name.span());
        let absolute_offset = header_size + offset;
        quote! {
            pub const #const_name: usize = #absolute_offset;
        }
    });

    // Generate slot index constants for variable fields
    let slot_index_consts = slot_fields.iter().enumerate().map(|(idx, (name, _ty))| {
        let const_name =
            syn::Ident::new(&format!("{}_SLOT", name.to_string().to_uppercase()), name.span());
        let slot_offset = header_size + fixed_size + (idx * 16);
        quote! {
            pub const #const_name: usize = #slot_offset;
        }
    });

    // Generate accessor methods for fixed fields
    let fixed_accessors = fixed_fields.iter().map(|(name, field_type, _offset, _size)| {
        let const_name =
            syn::Ident::new(&format!("{}_OFFSET", name.to_string().to_uppercase()), name.span());
        let reader_method = get_reader_method_from_type(field_type);

        quote! {
            #[inline(always)]
            pub fn #name(reader: &::serializer::machine::quantum::QuantumReader) -> #field_type {
                reader.#reader_method::<{Self::#const_name}>()
            }
        }
    });

    // Generate accessor methods for slot fields
    let slot_accessors = slot_fields.iter().map(|(name, field_type)| {
        let const_name = syn::Ident::new(
            &format!("{}_SLOT", name.to_string().to_uppercase()),
            name.span(),
        );

        // Determine accessor based on type
        if is_string_type(field_type) {
            quote! {
                #[inline(always)]
                pub fn #name(reader: &::serializer::machine::quantum::QuantumReader) -> Option<&str> {
                    reader.read_inline_str::<{Self::#const_name}>()
                }
            }
        } else if is_bytes_type(field_type) {
            quote! {
                #[inline(always)]
                pub fn #name(reader: &::serializer::machine::quantum::QuantumReader) -> Option<&[u8]> {
                    reader.read_inline_bytes::<{Self::#const_name}>()
                }
            }
        } else {
            // Generic slot accessor
            quote! {
                #[inline(always)]
                pub fn #name(reader: &::serializer::machine::quantum::QuantumReader) -> Option<&str> {
                    reader.read_inline_str::<{Self::#const_name}>()
                }
            }
        }
    });

    let heap_offset = header_size + fixed_size + (slot_count * 16);

    let expanded = quote! {
        impl #name {
            /// Header size (magic + version + flags)
            pub const HEADER_SIZE: usize = #header_size;

            /// Total size of fixed (primitive) fields
            pub const FIXED_SIZE: usize = #fixed_size;

            /// Number of variable-length slots
            pub const SLOT_COUNT: usize = #slot_count;

            /// Heap offset (where variable data begins)
            pub const HEAP_OFFSET: usize = #heap_offset;

            /// Minimum buffer size required
            pub const MIN_SIZE: usize = #heap_offset;

            // Field offset constants
            #(#fixed_offset_consts)*

            // Slot index constants
            #(#slot_index_consts)*

            // Accessor methods for fixed fields
            #(#fixed_accessors)*

            // Accessor methods for slot fields
            #(#slot_accessors)*
        }

        impl ::serializer::machine::quantum::QuantumType for #name {
            const FIXED_SIZE: usize = #fixed_size;
            const SLOT_COUNT: usize = #slot_count;
        }
    };

    TokenStream::from(expanded)
}

/// Check if a type is fixed-size (primitive)
fn is_fixed_size_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;
            return matches!(
                ident.to_string().as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "f32"
                    | "f64"
                    | "bool"
            );
        }
    }
    false
}

/// Get the size in bytes of a fixed-size type
fn get_type_size_from_type(ty: &Type) -> usize {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;
            return match ident.to_string().as_str() {
                "u8" | "i8" | "bool" => 1,
                "u16" | "i16" => 2,
                "u32" | "i32" | "f32" => 4,
                "u64" | "i64" | "f64" => 8,
                "u128" | "i128" => 16,
                _ => 0,
            };
        }
    }
    0
}

/// Get the appropriate QuantumReader method for a type
fn get_reader_method_from_type(ty: &Type) -> syn::Ident {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;
            let method_name = match ident.to_string().as_str() {
                "u8" => "read_u8",
                "u16" => "read_u16",
                "u32" => "read_u32",
                "u64" => "read_u64",
                "i8" => "read_i8",
                "i16" => "read_i16",
                "i32" => "read_i32",
                "i64" => "read_i64",
                "f32" => "read_f32",
                "f64" => "read_f64",
                "bool" => "read_bool",
                _ => "read_u8",
            };
            return syn::Ident::new(method_name, proc_macro2::Span::call_site());
        }
    }
    syn::Ident::new("read_u8", proc_macro2::Span::call_site())
}

/// Check if type is String
fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

/// Check if type is Vec<u8> or &[u8]
fn is_bytes_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                return true;
            }
        }
    }
    false
}

/// Attribute macro for compile-time serialization
#[proc_macro_attribute]
pub fn dx_static_serialize(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemConst);

    let const_name = &input.ident;
    let const_type = &input.ty;
    let const_expr = &input.expr;
    let vis = &input.vis;

    // Generate a bytes constant name
    let bytes_name = syn::Ident::new(
        &format!("{}_BYTES", const_name.to_string().to_uppercase()),
        const_name.span(),
    );

    let expanded = quote! {
        // Keep the original const
        #vis const #const_name: #const_type = #const_expr;

        // Generate a lazy static for the serialized bytes
        #vis static #bytes_name: ::std::sync::LazyLock<Vec<u8>> =
            ::std::sync::LazyLock::new(|| {
                ::serializer::machine::rkyv_compat::to_bytes(&#const_name)
                    .expect("Failed to serialize static data")
            });
    };

    TokenStream::from(expanded)
}

/// Macro to include and serialize a file at compile time
///
/// Reads a file at compile time, parses it (JSON/TOML/YAML), and embeds serialized bytes.
///
/// # Example
///
/// ```ignore
/// const CONFIG: &[u8] = include_serialized!("config.toml");
/// ```
#[proc_macro]
pub fn include_serialized(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    // Get the file path relative to the crate root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let file_path = std::path::Path::new(&manifest_dir).join(&path_str);

    // Read the file at compile time
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return syn::Error::new_spanned(
                path_lit,
                format!("Failed to read file '{}': {}", file_path.display(), e),
            )
            .to_compile_error()
            .into();
        }
    };

    // Determine file format from extension
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Parse the content based on format
    let json_value: serde_json::Value = match extension {
        "json" => match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                return syn::Error::new_spanned(path_lit, format!("Failed to parse JSON: {}", e))
                    .to_compile_error()
                    .into();
            }
        },
        "toml" => match toml::from_str::<toml::Value>(&content) {
            Ok(toml_val) => match serde_json::to_value(toml_val) {
                Ok(v) => v,
                Err(e) => {
                    return syn::Error::new_spanned(
                        path_lit,
                        format!("Failed to convert TOML to JSON: {}", e),
                    )
                    .to_compile_error()
                    .into();
                }
            },
            Err(e) => {
                return syn::Error::new_spanned(path_lit, format!("Failed to parse TOML: {}", e))
                    .to_compile_error()
                    .into();
            }
        },
        "yaml" | "yml" => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(yaml_val) => match serde_json::to_value(yaml_val) {
                Ok(v) => v,
                Err(e) => {
                    return syn::Error::new_spanned(
                        path_lit,
                        format!("Failed to convert YAML to JSON: {}", e),
                    )
                    .to_compile_error()
                    .into();
                }
            },
            Err(e) => {
                return syn::Error::new_spanned(path_lit, format!("Failed to parse YAML: {}", e))
                    .to_compile_error()
                    .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                path_lit,
                format!(
                    "Unsupported file format: '{}'. Supported: .json, .toml, .yaml, .yml",
                    extension
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    // Serialize to JSON bytes (placeholder for DX-Machine format)
    let serialized = match serde_json::to_vec(&json_value) {
        Ok(bytes) => bytes,
        Err(e) => {
            return syn::Error::new_spanned(path_lit, format!("Failed to serialize: {}", e))
                .to_compile_error()
                .into();
        }
    };

    // Generate byte array literal
    let byte_literals = serialized.iter().map(|b| quote! { #b });

    // Add compile-time dependency tracking
    let path_str_for_dep = file_path.display().to_string();

    let expanded = quote! {
        {
            // Tell cargo to recompile if the file changes
            const _: &[u8] = include_bytes!(#path_str_for_dep);

            // Embed the serialized bytes
            &[#(#byte_literals),*]
        }
    };

    TokenStream::from(expanded)
}
