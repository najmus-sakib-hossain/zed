//! Translation module
//!
//! This module provides translation functionality through multiple providers.

pub mod base;
pub mod constants;
pub mod google;
pub mod microsoft;

pub use base::Translator;
pub use google::GoogleTranslator;
pub use microsoft::MicrosoftTranslator;
