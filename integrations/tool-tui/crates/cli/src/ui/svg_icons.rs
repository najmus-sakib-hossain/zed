/// SVG-like icons using Unicode box-drawing and Braille characters
pub struct SvgIcon;

impl SvgIcon {
    /// Robot/AI icon (3x3)
    pub fn robot() -> Vec<&'static str> {
        vec![" ┌─┐ ", "┌┤●├┐", "└─┴─┘"]
    }

    /// User icon (3x3)
    pub fn user() -> Vec<&'static str> {
        vec![" ╭─╮ ", " │●│ ", "╰───╯"]
    }

    /// Lightning/Zap icon (3x3)
    pub fn lightning() -> Vec<&'static str> {
        vec!["╱╲  ", " ╱╲ ", "  ╲ "]
    }

    /// Clipboard/Plan icon (3x3)
    pub fn clipboard() -> Vec<&'static str> {
        vec!["┌─┬─┐", "│ ║ │", "└───┘"]
    }

    /// Chat bubble icon (3x3)
    pub fn chat() -> Vec<&'static str> {
        vec!["╭───╮", "│   │", "╰─╯ ╰"]
    }

    /// Checkmark icon (3x3)
    pub fn check() -> Vec<&'static str> {
        vec!["    ╱", "   ╱ ", "╲ ╱  "]
    }

    /// Cross/X icon (3x3)
    pub fn cross() -> Vec<&'static str> {
        vec!["╲   ╱", " ╲ ╱ ", " ╱ ╲ "]
    }

    /// Gear/Settings icon (3x3)
    pub fn gear() -> Vec<&'static str> {
        vec!["┬─┬─┬", "│ ● │", "┴─┴─┴"]
    }

    /// File icon (3x3)
    pub fn file() -> Vec<&'static str> {
        vec!["┌──╮", "│  │", "└──┘"]
    }

    /// Folder icon (3x3)
    pub fn folder() -> Vec<&'static str> {
        vec!["┌┬──╮", "├┴──┤", "└───┘"]
    }

    /// Package/Box icon (3x3)
    pub fn package() -> Vec<&'static str> {
        vec!["┌─┬─┐", "├─┼─┤", "└─┴─┘"]
    }

    /// Lock icon (3x3)
    pub fn lock() -> Vec<&'static str> {
        vec![" ╭─╮ ", "┌┴─┴┐", "└───┘"]
    }

    /// Unlock icon (3x3)
    pub fn unlock() -> Vec<&'static str> {
        vec!["╭─╮  ", "┌┴─┴┐", "└───┘"]
    }

    /// Search/Magnifying glass icon (3x3)
    pub fn search() -> Vec<&'static str> {
        vec![" ╭─╮ ", " │ │╱", " ╰─╯ "]
    }

    /// Star icon (3x3)
    pub fn star() -> Vec<&'static str> {
        vec!["  ╱╲ ", "╱╲  ╱", "  ╲╱ "]
    }

    /// Heart icon (3x3)
    pub fn heart() -> Vec<&'static str> {
        vec!["╱╲ ╱╲", "│   │", "╰───╯"]
    }

    /// Rocket icon (4x4)
    pub fn rocket() -> Vec<&'static str> {
        vec!["  ╱╲  ", " ╱  ╲ ", "│ ●● │", "╰────╯"]
    }

    /// Database icon (3x3)
    pub fn database() -> Vec<&'static str> {
        vec!["╭───╮", "├───┤", "╰───╯"]
    }

    /// Code/Terminal icon (3x3)
    pub fn code() -> Vec<&'static str> {
        vec!["< ╱ >", " ╱   ", "╱    "]
    }

    /// Bug icon (3x3)
    pub fn bug() -> Vec<&'static str> {
        vec!["┬ ╭╮ ┬", "│ ││ │", "┴ ╰╯ ┴"]
    }

    /// Warning triangle (3x3)
    pub fn warning() -> Vec<&'static str> {
        vec!["  ╱╲  ", " ╱ !╲ ", "╱────╲"]
    }

    /// Info circle (3x3)
    pub fn info() -> Vec<&'static str> {
        vec![" ╭─╮ ", " │i│ ", " ╰─╯ "]
    }

    /// Loading spinner frames (3x3 each)
    pub fn spinner_frames() -> Vec<Vec<&'static str>> {
        vec![
            vec!["│    ", "│    ", "│    "],
            vec!["╱    ", "     ", "     "],
            vec!["─    ", "     ", "     "],
            vec!["╲    ", "     ", "     "],
            vec!["│    ", "│    ", "│    "],
            vec!["     ", "     ", "╱    "],
            vec!["     ", "     ", "─    "],
            vec!["     ", "     ", "╲    "],
        ]
    }

    /// Arrow right (3x3)
    pub fn arrow_right() -> Vec<&'static str> {
        vec!["    ╱", "────╱", "    ╲"]
    }

    /// Arrow left (3x3)
    pub fn arrow_left() -> Vec<&'static str> {
        vec!["╲    ", "╲────", "╱    "]
    }

    /// Download icon (3x3)
    pub fn download() -> Vec<&'static str> {
        vec!["  │  ", "╲ │ ╱", " ╲│╱ "]
    }

    /// Upload icon (3x3)
    pub fn upload() -> Vec<&'static str> {
        vec![" ╱│╲ ", "╱ │ ╲", "  │  "]
    }

    /// Render icon with color
    pub fn render(lines: Vec<&str>, color: owo_colors::Style) -> String {
        lines
            .iter()
            .map(|line| format!("{}", color.style(line)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render icon inline (single line)
    pub fn render_inline(lines: Vec<&str>) -> String {
        lines.join(" ")
    }
}

/// Demo function to show all icons
pub fn show_svg_icon_gallery() {
    use owo_colors::OwoColorize;

    println!("\n{}", "═".repeat(70).bright_cyan());
    println!("{}", "  DX CLI SVG-Style Icon Gallery".bright_white().bold());
    println!("{}", "  Using Unicode Box Drawing Characters".bright_black());
    println!("{}\n", "═".repeat(70).bright_cyan());

    let icons = vec![
        ("Robot/AI", SvgIcon::robot(), "cyan"),
        ("User", SvgIcon::user(), "blue"),
        ("Lightning", SvgIcon::lightning(), "yellow"),
        ("Clipboard", SvgIcon::clipboard(), "green"),
        ("Chat", SvgIcon::chat(), "magenta"),
        ("Check", SvgIcon::check(), "green"),
        ("Cross", SvgIcon::cross(), "red"),
        ("Gear", SvgIcon::gear(), "white"),
        ("File", SvgIcon::file(), "blue"),
        ("Folder", SvgIcon::folder(), "yellow"),
        ("Package", SvgIcon::package(), "cyan"),
        ("Lock", SvgIcon::lock(), "red"),
        ("Unlock", SvgIcon::unlock(), "green"),
        ("Search", SvgIcon::search(), "blue"),
        ("Star", SvgIcon::star(), "yellow"),
        ("Heart", SvgIcon::heart(), "red"),
        ("Database", SvgIcon::database(), "cyan"),
        ("Code", SvgIcon::code(), "green"),
        ("Bug", SvgIcon::bug(), "red"),
        ("Warning", SvgIcon::warning(), "yellow"),
        ("Info", SvgIcon::info(), "blue"),
        ("Arrow Right", SvgIcon::arrow_right(), "white"),
        ("Arrow Left", SvgIcon::arrow_left(), "white"),
        ("Download", SvgIcon::download(), "green"),
        ("Upload", SvgIcon::upload(), "blue"),
    ];

    // Display in grid format
    for chunk in icons.chunks(3) {
        // Print names
        print!("  ");
        for (name, _, _) in chunk {
            print!("{:20}", name.bright_white().bold());
        }
        println!();

        // Print icons (3 lines each)
        for line_idx in 0..3 {
            print!("  ");
            for (_, lines, color_name) in chunk {
                let colored = match *color_name {
                    "cyan" => lines[line_idx].bright_cyan().to_string(),
                    "blue" => lines[line_idx].bright_blue().to_string(),
                    "yellow" => lines[line_idx].bright_yellow().to_string(),
                    "green" => lines[line_idx].bright_green().to_string(),
                    "magenta" => lines[line_idx].bright_magenta().to_string(),
                    "red" => lines[line_idx].bright_red().to_string(),
                    _ => lines[line_idx].bright_white().to_string(),
                };
                print!("{:20}", colored);
            }
            println!();
        }
        println!();
    }

    // Show rocket separately (it's 4 lines)
    println!("  {}", "Rocket".bright_white().bold());
    for line in SvgIcon::rocket() {
        println!("  {}", line.bright_yellow());
    }

    println!("\n{}", "═".repeat(70).bright_cyan());
    println!("  {} Pure Unicode - No external dependencies!", "✓".bright_green());
    println!("{}\n", "═".repeat(70).bright_cyan());
}
