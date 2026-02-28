use ratatui::style::{Color, Modifier, Style};
// Temporarily disabled - MachineFormat issue
// use serializer::{DxLlmValue, MachineFormat, machine_to_document};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct ChatTheme {
    pub variant: ThemeVariant,
    // Core shadcn-ui/Vercel design system colors
    pub bg: Color,
    pub fg: Color,
    pub card: Color,
    pub card_fg: Color,
    pub popover: Color,
    pub popover_fg: Color,
    pub primary: Color,
    pub primary_fg: Color,
    pub secondary: Color,
    pub secondary_fg: Color,
    pub muted: Color,
    pub muted_fg: Color,
    pub accent: Color,
    pub accent_fg: Color,
    pub destructive: Color,
    pub destructive_fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub input: Color,
    pub ring: Color,
    // Legacy compatibility
    pub user_msg_bg: Color,
    pub ai_msg_bg: Color,
    pub accent_secondary: Color,
    pub shimmer_colors: Vec<Color>,
    pub mode_colors: ModeColors,
}

#[derive(Debug, Clone)]
pub struct ModeColors {
    pub agent: Color,
    pub plan: Color,
    pub ask: Color,
}

impl ChatTheme {
    pub fn new(variant: ThemeVariant) -> Self {
        // Try to load from theme.sr, fallback to hardcoded if it fails
        Self::from_theme_sr(variant).unwrap_or_else(|_| match variant {
            ThemeVariant::Dark => Self::dark_fallback(),
            ThemeVariant::Light => Self::light_fallback(),
        })
    }

    /// Load theme from embedded theme.machine file
    fn from_theme_sr(variant: ThemeVariant) -> Result<Self, Box<dyn std::error::Error>> {
        // Temporarily disabled - MachineFormat constructor issue
        // Return dark fallback for now
        return Ok(Self::dark_fallback());

        // The rest is commented out until MachineFormat is fixed
        /*
        // Embed the compiled theme.machine file at compile time
        const THEME_MACHINE: &[u8] =
            include_bytes!("../../../../../.dx/serializer/crates/cli/theme.machine");

        // Parse from machine format (binary, LZ4 compressed)
        let machine = MachineFormat {
            data: THEME_MACHINE.to_vec(),
        };
        let doc = machine_to_document(&machine)?;

        // Extract the appropriate theme section
        let (section_name, modes_name) = match variant {
            ThemeVariant::Dark => ("dark", "dark_modes"),
            ThemeVariant::Light => ("light", "light_modes"),
        };

        let theme_section = doc
            .section_names
            .iter()
            .find(|(_, name)| name.as_str() == section_name)
            .and_then(|(id, _)| doc.sections.get(id))
            .ok_or("Theme section not found")?;

        let modes_section = doc
            .section_names
            .iter()
            .find(|(_, name)| name.as_str() == modes_name)
            .and_then(|(id, _)| doc.sections.get(id))
            .ok_or("Modes section not found")?;

        // Extract colors from the theme section
        let colors = if let Some(row) = theme_section.rows.first() {
            if let Some(DxLlmValue::Obj(obj)) = row.first() {
                obj
            } else {
                return Err("Invalid theme data format".into());
            }
        } else {
            return Err("Theme section has no data".into());
        };

        // Extract mode colors
        let mode_colors = if let Some(row) = modes_section.rows.first() {
            if let Some(DxLlmValue::Obj(obj)) = row.first() {
                obj
            } else {
                return Err("Invalid mode colors format".into());
            }
        } else {
            return Err("Modes section has no data".into());
        };

        // Helper to parse RGB color string "r,g,b" -> Color::Rgb(r, g, b)
        let parse_color = |key: &str| -> Result<Color, Box<dyn std::error::Error>> {
            let value = colors.get(key).ok_or(format!("Missing color: {}", key))?;
            if let DxLlmValue::Str(s) = value {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 3 {
                    let r = parts[0].parse::<u8>()?;
                    let g = parts[1].parse::<u8>()?;
                    let b = parts[2].parse::<u8>()?;
                    Ok(Color::Rgb(r, g, b))
                } else {
                    Err(format!("Invalid RGB format: {}", s).into())
                }
            } else {
                Err(format!("Color {} is not a string", key).into())
            }
        };

        let parse_mode_color = |key: &str| -> Result<Color, Box<dyn std::error::Error>> {
            let value = mode_colors.get(key).ok_or(format!("Missing mode color: {}", key))?;
            if let DxLlmValue::Str(s) = value {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 3 {
                    let r = parts[0].parse::<u8>()?;
                    let g = parts[1].parse::<u8>()?;
                    let b = parts[2].parse::<u8>()?;
                    Ok(Color::Rgb(r, g, b))
                } else {
                    Err(format!("Invalid RGB format: {}", s).into())
                }
            } else {
                Err(format!("Mode color {} is not a string", key).into())
            }
        };

        let bg = parse_color("background")?;
        let fg = parse_color("foreground")?;
        let border = parse_color("border")?;
        let accent = parse_color("accent")?;
        let muted_fg = parse_color("muted_foreground")?;

        Ok(Self {
            variant,
            bg,
            fg,
            card: parse_color("card")?,
            card_fg: parse_color("card_foreground")?,
            popover: parse_color("popover")?,
            popover_fg: parse_color("popover_foreground")?,
            primary: parse_color("primary")?,
            primary_fg: parse_color("primary_foreground")?,
            secondary: parse_color("secondary")?,
            secondary_fg: parse_color("secondary_foreground")?,
            muted: parse_color("muted")?,
            muted_fg,
            accent,
            accent_fg: parse_color("accent_foreground")?,
            destructive: parse_color("destructive")?,
            destructive_fg: parse_color("destructive_foreground")?,
            border,
            border_focused: accent, // Use accent for focused borders
            input: parse_color("input")?,
            ring: parse_color("ring")?,
            // Legacy compatibility
            user_msg_bg: parse_color("card")?,
            ai_msg_bg: parse_color("popover")?,
            accent_secondary: accent,
            shimmer_colors: vec![border, accent, muted_fg, accent, border],
            mode_colors: ModeColors {
                agent: parse_mode_color("agent")?,
                plan: parse_mode_color("plan")?,
                ask: parse_mode_color("ask")?,
            },
        })
        */
    }

    fn dark_fallback() -> Self {
        // Dark mode from your CSS theme - oklch values converted to RGB
        // Using --primary (green) as the main accent throughout the UI
        Self {
            variant: ThemeVariant::Dark,
            bg: Color::Rgb(0, 0, 0),                 // --background
            fg: Color::Rgb(255, 255, 255),           // --foreground
            card: Color::Rgb(9, 9, 9),               // --card
            card_fg: Color::Rgb(255, 255, 255),      // --card-foreground
            popover: Color::Rgb(18, 18, 18),         // --popover
            popover_fg: Color::Rgb(255, 255, 255),   // --popover-foreground
            primary: Color::Rgb(0, 201, 80),         // --primary (green)
            primary_fg: Color::Rgb(255, 255, 255),   // --primary-foreground
            secondary: Color::Rgb(34, 34, 34),       // --secondary
            secondary_fg: Color::Rgb(255, 255, 255), // --secondary-foreground
            muted: Color::Rgb(29, 29, 29),           // --muted
            muted_fg: Color::Rgb(164, 164, 164),     // --muted-foreground
            accent: Color::Rgb(0, 201, 80),          // Use primary green as accent
            accent_fg: Color::Rgb(255, 255, 255),    // --accent-foreground
            destructive: Color::Rgb(255, 91, 91),    // --destructive
            destructive_fg: Color::Rgb(0, 0, 0),     // --destructive-foreground
            border: Color::Rgb(36, 36, 36),          // --border
            border_focused: Color::Rgb(0, 201, 80),  // Use primary green for focus
            input: Color::Rgb(51, 51, 51),           // --input
            ring: Color::Rgb(164, 164, 164),         // --ring
            // Legacy compatibility
            user_msg_bg: Color::Rgb(9, 9, 9),         // card
            ai_msg_bg: Color::Rgb(18, 18, 18),        // popover
            accent_secondary: Color::Rgb(0, 201, 80), // primary green
            shimmer_colors: vec![
                Color::Rgb(36, 36, 36),    // border
                Color::Rgb(0, 201, 80),    // primary green
                Color::Rgb(164, 164, 164), // muted_fg
                Color::Rgb(0, 201, 80),    // primary green
                Color::Rgb(36, 36, 36),    // border
            ],
            mode_colors: ModeColors {
                agent: Color::Rgb(0, 201, 80), // primary green
                plan: Color::Rgb(255, 174, 4), // chart-1 yellow
                ask: Color::Rgb(38, 113, 244), // chart-2 blue
            },
        }
    }

    fn light_fallback() -> Self {
        // Light mode from theme.css - shadcn-ui/Vercel design system
        Self {
            variant: ThemeVariant::Light,
            bg: Color::Rgb(252, 252, 252),             // --background
            fg: Color::Rgb(0, 0, 0),                   // --foreground
            card: Color::Rgb(255, 255, 255),           // --card
            card_fg: Color::Rgb(0, 0, 0),              // --card-foreground
            popover: Color::Rgb(252, 252, 252),        // --popover
            popover_fg: Color::Rgb(0, 0, 0),           // --popover-foreground
            primary: Color::Rgb(0, 0, 0),              // --primary
            primary_fg: Color::Rgb(255, 255, 255),     // --primary-foreground
            secondary: Color::Rgb(235, 235, 235),      // --secondary
            secondary_fg: Color::Rgb(0, 0, 0),         // --secondary-foreground
            muted: Color::Rgb(245, 245, 245),          // --muted
            muted_fg: Color::Rgb(82, 82, 82),          // --muted-foreground
            accent: Color::Rgb(235, 235, 235),         // --accent
            accent_fg: Color::Rgb(0, 0, 0),            // --accent-foreground
            destructive: Color::Rgb(229, 75, 79),      // --destructive
            destructive_fg: Color::Rgb(255, 255, 255), // --destructive-foreground
            border: Color::Rgb(228, 228, 228),         // --border
            border_focused: Color::Rgb(0, 0, 0),       // --ring
            input: Color::Rgb(235, 235, 235),          // --input
            ring: Color::Rgb(0, 0, 0),                 // --ring
            // Legacy compatibility
            user_msg_bg: Color::Rgb(255, 255, 255), // card
            ai_msg_bg: Color::Rgb(252, 252, 252),   // popover
            accent_secondary: Color::Rgb(235, 235, 235), // accent
            shimmer_colors: vec![
                Color::Rgb(228, 228, 228), // border
                Color::Rgb(235, 235, 235), // accent
                Color::Rgb(82, 82, 82),    // muted_fg
                Color::Rgb(235, 235, 235), // accent
                Color::Rgb(228, 228, 228), // border
            ],
            mode_colors: ModeColors {
                agent: Color::Rgb(0, 160, 60), // darker green for light mode
                plan: Color::Rgb(200, 130, 0), // darker yellow for light mode
                ask: Color::Rgb(30, 90, 200),  // darker blue for light mode
            },
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.fg).add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.border_focused
        } else {
            self.border
        })
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.primary).add_modifier(Modifier::BOLD)
    }

    pub fn mode_style(&self, mode: &crate::ui::chat::modes::ChatMode) -> Style {
        let color = match mode {
            crate::ui::chat::modes::ChatMode::Agent => self.mode_colors.agent,
            crate::ui::chat::modes::ChatMode::Plan => self.mode_colors.plan,
            crate::ui::chat::modes::ChatMode::Ask => self.mode_colors.ask,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }
}
