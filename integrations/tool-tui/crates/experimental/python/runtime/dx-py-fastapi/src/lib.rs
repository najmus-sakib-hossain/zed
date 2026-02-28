//! DX-Py FastAPI Compatibility Layer
//!
//! This crate provides compatibility implementations for FastAPI-required
//! components, including:
//! - Pydantic model validation (Rust core compatibility)
//! - Async/await runtime integration
//! - ASGI protocol support (Starlette compatibility)
//! - Type hint validation for endpoints

pub mod asgi;
pub mod async_runtime;
pub mod pydantic;
pub mod starlette;
pub mod validation;

pub use asgi::{AsgiApp, AsgiError, AsgiMessage, AsgiScope};
pub use async_runtime::{
    AsyncContextManager, AsyncError, AsyncGenerator, AsyncIterator, Coroutine, CoroutineResult,
    CoroutineState, EventLoop, EventLoopRegistry, Task,
};
pub use pydantic::{
    FieldType, JsonSchema, ModelValidator, PydanticField, PydanticModel, SchemaGenerator,
    ValidationError,
};
pub use starlette::{
    Request as StarletteRequest, Response as StarletteResponse, Route as StarletteRoute,
    StarletteApp,
};
pub use validation::{EndpointValidator, TypeValidator, ValidationResult};
