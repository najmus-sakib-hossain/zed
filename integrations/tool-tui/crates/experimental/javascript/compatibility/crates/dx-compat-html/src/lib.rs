//! # dx-compat-html
//!
//! HTML Rewriter compatibility layer using lol_html.
//!
//! Provides streaming HTML transformation similar to Cloudflare's HTMLRewriter API.
//!
//! # Example
//! ```ignore
//! use dx_compat_html::{HTMLRewriter, ContentType};
//!
//! let mut rewriter = HTMLRewriter::new();
//! rewriter.on("a[href]", |el| {
//!     if let Some(href) = el.get_attribute("href") {
//!         el.set_attribute("target", "_blank");
//!     }
//! });
//!
//! let result = rewriter.transform("<a href='example.com'>Link</a>")?;
//! ```

#![warn(missing_docs)]

mod element;
mod error;
mod rewriter;

pub use element::{ContentType, DocumentProxy, ElementProxy};
pub use error::{HtmlError, HtmlResult};
pub use rewriter::{transform_html, HTMLRewriter};
