use gpui::Hsla;

/// Adjust the alpha channel of an HSLA colour.
///
/// Useful for creating semi-transparent variants of theme colours.
///
/// # Examples
/// ```
/// let translucent = with_alpha(theme.primary, 0.5);
/// ```
pub fn with_alpha(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: alpha, ..color }
}
