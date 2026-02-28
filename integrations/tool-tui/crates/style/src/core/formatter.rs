use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

fn add_leading_zero_to_fractions(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + 8);
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'.' {
            let prev = out.last().copied();
            let next = bytes.get(i + 1).copied();
            let prev_is_digit = prev.is_some_and(|ch| ch.is_ascii_digit());
            let next_is_digit = next.is_some_and(|ch| ch.is_ascii_digit());
            if !prev_is_digit && next_is_digit {
                out.push(b'0');
            }
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| code.to_string())
}

pub fn format_css_pretty(input: &str) -> Option<String> {
    let sheet = StyleSheet::parse(
        input,
        ParserOptions {
            error_recovery: true,
            ..ParserOptions::default()
        },
    )
    .ok()?;
    let printed = sheet
        .to_css(PrinterOptions {
            minify: false,
            ..PrinterOptions::default()
        })
        .ok()?;
    let mut out = printed.code;
    out = out.replace("}\n.", "}\n\n.");
    out = add_leading_zero_to_fractions(&out);
    Some(out)
}
