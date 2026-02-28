//! Inlineable decorator definitions

/// Decorators that can be inlined at compile time
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineableDecorator {
    /// @staticmethod - zero overhead, just sets a flag
    StaticMethod,

    /// @classmethod - injects cls as first argument
    ClassMethod,

    /// @property - generates getter descriptor
    Property,

    /// @lru_cache(maxsize=N) - inlines cache lookup/store
    LruCache { maxsize: Option<usize> },

    /// @dataclass - generates __init__, __repr__, __eq__, etc.
    Dataclass {
        frozen: bool,
        slots: bool,
        eq: bool,
        order: bool,
        hash: bool,
    },

    /// @jit - marks for immediate JIT compilation
    Jit,

    /// @parallel - marks for automatic parallelization
    Parallel,

    /// Custom decorator (not inlined, but tracked)
    Custom(String),
}

impl InlineableDecorator {
    /// Parse a decorator from its name and arguments
    pub fn parse(name: &str, args: &[(&str, &str)]) -> Option<Self> {
        match name {
            "staticmethod" => Some(Self::StaticMethod),
            "classmethod" => Some(Self::ClassMethod),
            "property" => Some(Self::Property),
            "lru_cache" | "functools.lru_cache" => {
                let maxsize = args.iter().find(|(k, _)| *k == "maxsize").and_then(|(_, v)| {
                    if *v == "None" {
                        None
                    } else {
                        v.parse().ok()
                    }
                });
                Some(Self::LruCache { maxsize })
            }
            "dataclass" | "dataclasses.dataclass" => {
                let frozen = args
                    .iter()
                    .find(|(k, _)| *k == "frozen")
                    .map(|(_, v)| *v == "True")
                    .unwrap_or(false);
                let slots = args
                    .iter()
                    .find(|(k, _)| *k == "slots")
                    .map(|(_, v)| *v == "True")
                    .unwrap_or(false);
                let eq = args
                    .iter()
                    .find(|(k, _)| *k == "eq")
                    .map(|(_, v)| *v == "True")
                    .unwrap_or(true);
                let order = args
                    .iter()
                    .find(|(k, _)| *k == "order")
                    .map(|(_, v)| *v == "True")
                    .unwrap_or(false);
                let hash = args
                    .iter()
                    .find(|(k, _)| *k == "hash")
                    .map(|(_, v)| *v == "True")
                    .unwrap_or(false);

                Some(Self::Dataclass {
                    frozen,
                    slots,
                    eq,
                    order,
                    hash,
                })
            }
            "jit" | "numba.jit" => Some(Self::Jit),
            "parallel" => Some(Self::Parallel),
            _ => Some(Self::Custom(name.to_string())),
        }
    }

    /// Check if this decorator can be fully inlined
    pub fn is_inlineable(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }

    /// Get the function flags to set for this decorator
    pub fn get_flags(&self) -> FunctionFlags {
        match self {
            Self::StaticMethod => FunctionFlags::STATIC_METHOD,
            Self::ClassMethod => FunctionFlags::CLASS_METHOD,
            Self::Property => FunctionFlags::PROPERTY_GETTER,
            Self::LruCache { .. } => FunctionFlags::HAS_CACHE,
            Self::Dataclass { .. } => FunctionFlags::empty(),
            Self::Jit => FunctionFlags::IMMEDIATE_JIT,
            Self::Parallel => FunctionFlags::AUTO_PARALLEL,
            Self::Custom(_) => FunctionFlags::empty(),
        }
    }
}

bitflags::bitflags! {
    /// Flags for function objects
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FunctionFlags: u32 {
        /// Function is a static method
        const STATIC_METHOD = 0x0001;
        /// Function is a class method
        const CLASS_METHOD = 0x0002;
        /// Function is a property getter
        const PROPERTY_GETTER = 0x0004;
        /// Function is a property setter
        const PROPERTY_SETTER = 0x0008;
        /// Function has an LRU cache
        const HAS_CACHE = 0x0010;
        /// Function should be JIT compiled immediately
        const IMMEDIATE_JIT = 0x0020;
        /// Function can be auto-parallelized
        const AUTO_PARALLEL = 0x0040;
        /// Function is a generator
        const GENERATOR = 0x0080;
        /// Function is async
        const ASYNC = 0x0100;
        /// Function has type hints
        const HAS_TYPES = 0x0200;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_staticmethod() {
        let dec = InlineableDecorator::parse("staticmethod", &[]).unwrap();
        assert_eq!(dec, InlineableDecorator::StaticMethod);
        assert!(dec.is_inlineable());
    }

    #[test]
    fn test_parse_lru_cache() {
        let dec = InlineableDecorator::parse("lru_cache", &[("maxsize", "128")]).unwrap();
        assert_eq!(dec, InlineableDecorator::LruCache { maxsize: Some(128) });
    }

    #[test]
    fn test_parse_dataclass() {
        let dec = InlineableDecorator::parse("dataclass", &[("frozen", "True"), ("slots", "True")])
            .unwrap();

        match dec {
            InlineableDecorator::Dataclass { frozen, slots, .. } => {
                assert!(frozen);
                assert!(slots);
            }
            _ => panic!("Expected Dataclass"),
        }
    }

    #[test]
    fn test_parse_custom() {
        let dec = InlineableDecorator::parse("my_decorator", &[]).unwrap();
        assert_eq!(dec, InlineableDecorator::Custom("my_decorator".to_string()));
        assert!(!dec.is_inlineable());
    }

    #[test]
    fn test_flags() {
        assert_eq!(InlineableDecorator::StaticMethod.get_flags(), FunctionFlags::STATIC_METHOD);
        assert_eq!(InlineableDecorator::Jit.get_flags(), FunctionFlags::IMMEDIATE_JIT);
    }
}
