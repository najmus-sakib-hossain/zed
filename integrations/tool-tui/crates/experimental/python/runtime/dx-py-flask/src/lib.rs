//! DX-Py Flask Compatibility Layer
//!
//! This crate provides compatibility implementations for Flask-required
//! components, including:
//! - Werkzeug C extension compatibility (URL routing, request/response handling)
//! - Jinja2 template compatibility
//! - WSGI protocol support

pub mod jinja;
pub mod werkzeug;
pub mod wsgi;

pub use jinja::{JinjaContext, JinjaEngine, JinjaError, JinjaTemplate};
pub use werkzeug::{
    Request, RequestError, Response, ResponseBuilder, Route, RouteMatch, RoutingError, UrlRouter,
};
pub use wsgi::{WsgiApp, WsgiEnviron, WsgiError, WsgiResponse};
