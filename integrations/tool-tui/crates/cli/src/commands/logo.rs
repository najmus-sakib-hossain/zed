//! Display ASCII art logos from various programming languages and tools

use anyhow::Result;
use onefetch_ascii::AsciiArt;
use owo_colors::{AnsiColors, DynColors, OwoColorize};

use crate::ui::theme::Theme;

pub fn run_logo(_theme: &Theme) -> Result<()> {
    println!();
    println!("  {} DX ASCII Art Logo Gallery", "◆".cyan().bold());
    println!();

    // Display all available logos
    display_rust_logo();
    display_python_logo();
    display_javascript_logo();
    display_go_logo();
    display_java_logo();
    display_cpp_logo();
    display_ruby_logo();
    display_php_logo();
    display_swift_logo();
    display_kotlin_logo();

    println!();
    println!("  {} Gallery complete!", "✓".green().bold());
    println!();

    Ok(())
}

fn display_rust_logo() {
    println!("  {} {}", "▸".cyan(), "Rust".bright_white().bold());

    let ascii = r#"
{0}            _~^~^~_
{0}        \) /  o o  \ (/
{0}          '_   -   _'
{0}          / '-----' \
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightRed)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_python_logo() {
    println!("  {} {}", "▸".cyan(), "Python".bright_white().bold());

    let ascii = r#"
{0}         .--.
{0}        |o_o |
{0}        |:_/ |
{0}       //   \ \
{0}      (|     | )
{0}     /'\_   _/`\
{0}     \___)=(___/
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightBlue)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_javascript_logo() {
    println!("  {} {}", "▸".cyan(), "JavaScript".bright_white().bold());

    let ascii = r#"
{0}     ___  ___
{0}    |   ||   |
{0}    | | || | |
{0}  __|_|_||_|_|__
{0} |  JavaScript  |
{0}  --------------
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightYellow)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_go_logo() {
    println!("  {} {}", "▸".cyan(), "Go".bright_white().bold());

    let ascii = r#"
{0}     ,_---~~~~~----._
{0}  _,,_,*^____      _____``*g*\"*,
{0} / __/ /'     ^.  /      \ ^@q   f
{0}[  @f | @))    |  | @))   l  0 _/
{0} \`/   \~____ / __ \_____/    \
{0}  |           _l__l_           I
{0}  }          [______]           I
{0}  ]            | | |            |
{0}  ]             ~ ~             |
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightCyan)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_java_logo() {
    println!("  {} {}", "▸".cyan(), "Java".bright_white().bold());

    let ascii = r#"
{0}       _
{0}      | |
{0}      | | __ ___   ____ _
{0}  _   | |/ _` \ \ / / _` |
{0} | |__| | (_| |\ V / (_| |
{0}  \____/ \__,_| \_/ \__,_|
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightRed)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_cpp_logo() {
    println!("  {} {}", "▸".cyan(), "C++".bright_white().bold());

    let ascii = r#"
{0}    _____
{0}   / ____|_     _
{0}  | |   _| |_ _| |_
{0}  | |  |_   _|_   _|
{0}  | |____|_|   |_|
{0}   \_____|
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightBlue)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_ruby_logo() {
    println!("  {} {}", "▸".cyan(), "Ruby".bright_white().bold());

    let ascii = r#"
{0}        .---.
{0}       /     \
{0}       \.@-@./
{0}       /`\_/`\
{0}      //  _  \\
{0}     | \     )|_
{0}    /`\_`>  <_/ \
{0}    \__/'---'\__/
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightRed)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_php_logo() {
    println!("  {} {}", "▸".cyan(), "PHP".bright_white().bold());

    let ascii = r#"
{0}   ____  _   _ ____
{0}  |  _ \| | | |  _ \
{0}  | |_) | |_| | |_) |
{0}  |  __/|  _  |  __/
{0}  |_|   |_| |_|_|
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightMagenta)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_swift_logo() {
    println!("  {} {}", "▸".cyan(), "Swift".bright_white().bold());

    let ascii = r#"
{0}    .-.
{0}   (o.o)
{0}    |=|
{0}   __|__
{0}  //.=|=.\\
{0} // .=|=. \\
{0} \\ .=|=. //
{0}  \\(_=_)//
{0}   (:| |:)
{0}    || ||
{0}    () ()
{0}    || ||
{0}    || ||
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightRed)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}

fn display_kotlin_logo() {
    println!("  {} {}", "▸".cyan(), "Kotlin".bright_white().bold());

    let ascii = r#"
{0}   |\  /|
{0}   | \/ |
{0}   |    |
{0}   | /\ |
{0}   |/  \|
"#;

    let colors = vec![DynColors::Ansi(AnsiColors::BrightMagenta)];
    let art = AsciiArt::new(ascii, &colors, true);

    for line in art {
        println!("    {}", line);
    }
    println!();
}
