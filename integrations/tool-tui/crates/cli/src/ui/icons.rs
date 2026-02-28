/// Icon collection for CLI UI
pub struct Icons;

impl Icons {
    // Status Icons
    pub const SUCCESS: &'static str = "✅";
    pub const ERROR: &'static str = "❌";
    pub const WARNING: &'static str = "⚠️";
    pub const INFO: &'static str = "ℹ️";
    pub const LOADING: &'static str = "⏳";
    pub const DONE: &'static str = "✓";

    // Action Icons
    pub const ROCKET: &'static str = "🚀";
    pub const FIRE: &'static str = "🔥";
    pub const SPARKLES: &'static str = "✨";
    pub const ZAP: &'static str = "⚡";
    pub const STAR: &'static str = "⭐";
    pub const HEART: &'static str = "❤️";
    pub const THUMBS_UP: &'static str = "👍";

    // People & Roles
    pub const USER: &'static str = "👤";
    pub const ROBOT: &'static str = "🤖";
    pub const DEVELOPER: &'static str = "👨‍💻";
    pub const TEAM: &'static str = "👥";

    // Communication
    pub const CHAT: &'static str = "💬";
    pub const MESSAGE: &'static str = "📨";
    pub const MAIL: &'static str = "📧";
    pub const BELL: &'static str = "🔔";
    pub const MEGAPHONE: &'static str = "📣";

    // Documents & Files
    pub const FILE: &'static str = "📄";
    pub const FOLDER: &'static str = "📁";
    pub const CLIPBOARD: &'static str = "📋";
    pub const MEMO: &'static str = "📝";
    pub const BOOK: &'static str = "📚";
    pub const PAGE: &'static str = "📃";

    // Tools & Development
    pub const WRENCH: &'static str = "🔧";
    pub const HAMMER: &'static str = "🔨";
    pub const GEAR: &'static str = "⚙️";
    pub const PACKAGE: &'static str = "📦";
    pub const LOCK: &'static str = "🔒";
    pub const UNLOCK: &'static str = "🔓";
    pub const KEY: &'static str = "🔑";
    pub const SHIELD: &'static str = "🛡️";

    // Navigation
    pub const ARROW_RIGHT: &'static str = "→";
    pub const ARROW_LEFT: &'static str = "←";
    pub const ARROW_UP: &'static str = "↑";
    pub const ARROW_DOWN: &'static str = "↓";
    pub const POINTER: &'static str = "👉";
    pub const BACK: &'static str = "🔙";
    pub const HOME: &'static str = "🏠";

    // Time & Progress
    pub const CLOCK: &'static str = "🕐";
    pub const HOURGLASS: &'static str = "⌛";
    pub const TIMER: &'static str = "⏱️";
    pub const CALENDAR: &'static str = "📅";

    // Data & Analytics
    pub const CHART: &'static str = "📊";
    pub const GRAPH: &'static str = "📈";
    pub const DATABASE: &'static str = "🗄️";
    pub const SEARCH: &'static str = "🔍";
    pub const MAGNIFY: &'static str = "🔎";

    // Nature & Weather
    pub const SUN: &'static str = "☀️";
    pub const MOON: &'static str = "🌙";
    pub const CLOUD: &'static str = "☁️";
    pub const RAIN: &'static str = "🌧️";
    pub const SNOW: &'static str = "❄️";
    pub const TREE: &'static str = "🌲";

    // Symbols
    pub const CHECK: &'static str = "✓";
    pub const CROSS: &'static str = "✗";
    pub const PLUS: &'static str = "+";
    pub const MINUS: &'static str = "-";
    pub const BULLET: &'static str = "•";
    pub const DIAMOND: &'static str = "◆";
    pub const CIRCLE: &'static str = "●";
    pub const SQUARE: &'static str = "■";

    // Box Drawing
    pub const BOX_VERTICAL: &'static str = "│";
    pub const BOX_HORIZONTAL: &'static str = "─";
    pub const BOX_TOP_LEFT: &'static str = "┌";
    pub const BOX_TOP_RIGHT: &'static str = "┐";
    pub const BOX_BOTTOM_LEFT: &'static str = "└";
    pub const BOX_BOTTOM_RIGHT: &'static str = "┘";
    pub const BOX_CROSS: &'static str = "┼";
    pub const BOX_T_DOWN: &'static str = "┬";
    pub const BOX_T_UP: &'static str = "┴";
    pub const BOX_T_RIGHT: &'static str = "├";
    pub const BOX_T_LEFT: &'static str = "┤";

    // Programming
    pub const CODE: &'static str = "💻";
    pub const BUG: &'static str = "🐛";
    pub const TERMINAL: &'static str = "⌨️";
    pub const BINARY: &'static str = "🔢";

    // Misc
    pub const GIFT: &'static str = "🎁";
    pub const TROPHY: &'static str = "🏆";
    pub const TARGET: &'static str = "🎯";
    pub const LIGHT_BULB: &'static str = "💡";
    pub const CRYSTAL_BALL: &'static str = "🔮";
    pub const MAGIC_WAND: &'static str = "🪄";
}

/// Print all available icons with descriptions
pub fn show_icon_gallery() {
    use owo_colors::OwoColorize;

    println!("\n{}", "═".repeat(60).bright_cyan());
    println!("{}", "  DX CLI Icon Gallery".bright_white().bold());
    println!("{}\n", "═".repeat(60).bright_cyan());

    let categories = vec![
        (
            "Status Icons",
            vec![
                (Icons::SUCCESS, "Success"),
                (Icons::ERROR, "Error"),
                (Icons::WARNING, "Warning"),
                (Icons::INFO, "Info"),
                (Icons::LOADING, "Loading"),
                (Icons::DONE, "Done"),
            ],
        ),
        (
            "Action Icons",
            vec![
                (Icons::ROCKET, "Rocket"),
                (Icons::FIRE, "Fire"),
                (Icons::SPARKLES, "Sparkles"),
                (Icons::ZAP, "Zap"),
                (Icons::STAR, "Star"),
                (Icons::HEART, "Heart"),
            ],
        ),
        (
            "People & Roles",
            vec![
                (Icons::USER, "User"),
                (Icons::ROBOT, "Robot/AI"),
                (Icons::DEVELOPER, "Developer"),
                (Icons::TEAM, "Team"),
            ],
        ),
        (
            "Communication",
            vec![
                (Icons::CHAT, "Chat"),
                (Icons::MESSAGE, "Message"),
                (Icons::MAIL, "Mail"),
                (Icons::BELL, "Bell"),
            ],
        ),
        (
            "Documents",
            vec![
                (Icons::FILE, "File"),
                (Icons::FOLDER, "Folder"),
                (Icons::CLIPBOARD, "Clipboard"),
                (Icons::MEMO, "Memo"),
                (Icons::BOOK, "Book"),
            ],
        ),
        (
            "Tools",
            vec![
                (Icons::WRENCH, "Wrench"),
                (Icons::HAMMER, "Hammer"),
                (Icons::GEAR, "Gear"),
                (Icons::PACKAGE, "Package"),
                (Icons::LOCK, "Lock"),
                (Icons::KEY, "Key"),
            ],
        ),
        (
            "Programming",
            vec![
                (Icons::CODE, "Code"),
                (Icons::BUG, "Bug"),
                (Icons::TERMINAL, "Terminal"),
                (Icons::BINARY, "Binary"),
            ],
        ),
        (
            "Misc",
            vec![
                (Icons::GIFT, "Gift"),
                (Icons::TROPHY, "Trophy"),
                (Icons::TARGET, "Target"),
                (Icons::LIGHT_BULB, "Light Bulb"),
                (Icons::MAGIC_WAND, "Magic Wand"),
            ],
        ),
    ];

    for (category, icons) in categories {
        println!("  {}", category.bright_yellow().bold());
        println!("  {}", "─".repeat(40).bright_black());

        for (icon, name) in icons {
            println!("    {}  {}", icon, name.bright_white());
        }
        println!();
    }

    println!("{}", "═".repeat(60).bright_cyan());
    println!("  {} Use these icons in your CLI apps!", Icons::SPARKLES);
    println!("{}\n", "═".repeat(60).bright_cyan());
}
