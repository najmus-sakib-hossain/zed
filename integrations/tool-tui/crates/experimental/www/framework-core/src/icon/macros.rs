//! Icon macro definitions

/// Compile-time icon resolution macro
///
/// This macro creates an `IconComponent` at compile time. The actual SVG
/// resolution happens during the build process via the IconProcessor.
#[macro_export]
macro_rules! icon {
    ($name:expr) => {
        $crate::icon::IconComponent::new($name)
    };

    ($name:expr, size = $size:expr) => {
        $crate::icon::IconComponent::new($name).with_size($size)
    };

    ($name:expr, color = $color:expr) => {
        $crate::icon::IconComponent::new($name).with_color($color)
    };

    ($name:expr, class = $class:expr) => {
        $crate::icon::IconComponent::new($name).with_class($class)
    };

    ($name:expr, size = $size:expr, color = $color:expr) => {
        $crate::icon::IconComponent::new($name).with_size($size).with_color($color)
    };

    ($name:expr, size = $size:expr, class = $class:expr) => {
        $crate::icon::IconComponent::new($name).with_size($size).with_class($class)
    };

    ($name:expr, color = $color:expr, class = $class:expr) => {
        $crate::icon::IconComponent::new($name).with_color($color).with_class($class)
    };

    ($name:expr, size = $size:expr, color = $color:expr, class = $class:expr) => {
        $crate::icon::IconComponent::new($name)
            .with_size($size)
            .with_color($color)
            .with_class($class)
    };
}
