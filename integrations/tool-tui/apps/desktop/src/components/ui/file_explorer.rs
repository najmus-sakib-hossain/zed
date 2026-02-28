use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::Theme;

// ─── FileExplorer ───────────────────────────────────────────────────────────
// A full file-browser sidebar component combining TreeView with file metadata
// and common file-type icons. Designed for IDE and desktop file manager UIs.
//
// Usage:
//   FileExplorer::new("Project")
//       .entry(FileEntry::directory("src")
//           .child(FileEntry::file("main.rs").size("4.2 KB"))
//           .child(FileEntry::file("lib.rs"))
//           .expanded(true)
//       )
//       .entry(FileEntry::file("Cargo.toml").size("1.1 KB"))
//       .show_hidden(true)
//       .render(&theme)

pub struct FileExplorer {
    root_label: String,
    entries: Vec<FileEntry>,
    show_hidden: bool,
    show_sizes: bool,
    indent: f32,
    selected_path: Option<String>,
}

#[allow(dead_code)]
impl FileExplorer {
    pub fn new(root_label: impl Into<String>) -> Self {
        Self {
            root_label: root_label.into(),
            entries: Vec::new(),
            show_hidden: false,
            show_sizes: true,
            indent: 16.0,
            selected_path: None,
        }
    }

    pub fn entry(mut self, entry: FileEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn entries(mut self, entries: Vec<FileEntry>) -> Self {
        self.entries = entries;
        self
    }

    pub fn show_hidden(mut self, v: bool) -> Self {
        self.show_hidden = v;
        self
    }

    pub fn show_sizes(mut self, v: bool) -> Self {
        self.show_sizes = v;
        self
    }

    pub fn indent(mut self, px_val: f32) -> Self {
        self.indent = px_val;
        self
    }

    pub fn selected(mut self, path: impl Into<String>) -> Self {
        self.selected_path = Some(path.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut col = div().flex().flex_col().h_full().overflow_hidden();

        // Header
        col = col.child(
            div().flex().items_center().justify_between().px(px(12.0)).py(px(8.0)).child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(self.root_label.to_uppercase()),
            ),
        );

        // Entries
        for entry in self.entries {
            Self::render_entry_into(
                &mut col,
                entry,
                0,
                self.indent,
                self.show_sizes,
                &self.selected_path,
                theme,
            );
        }

        col
    }

    fn render_entry_into(
        parent: &mut gpui::Div,
        entry: FileEntry,
        depth: usize,
        indent: f32,
        show_sizes: bool,
        selected_path: &Option<String>,
        theme: &Theme,
    ) {
        let left_pad = px(12.0 + depth as f32 * indent);
        let is_selected = selected_path.as_ref().is_some_and(|p| *p == entry.name);

        let bg = if is_selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if is_selected {
            theme.accent_foreground
        } else {
            theme.foreground
        };

        let icon = match entry.kind {
            FileKind::File => file_icon_for(&entry.name),
            FileKind::Directory => {
                if entry.expanded {
                    "📂"
                } else {
                    "📁"
                }
            }
        };

        let hover_bg = theme.ghost_hover;

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(left_pad)
            .pr(px(12.0))
            .py(px(3.0))
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg))
            .child(div().text_xs().w(px(16.0)).text_center().child(icon))
            .child(div().flex_1().truncate().child(entry.name.clone()));

        if show_sizes {
            if let Some(size) = &entry.size {
                row = row
                    .child(div().text_xs().text_color(theme.muted_foreground).child(size.clone()));
            }
        }

        // We can't take `parent` mutably in a loop and also pass entries, so we
        // use a workaround: collect children into a vec and extend the parent.
        // However, GPUI Div doesn't implement Extend. Instead we chain .child()
        // calls. Since we can't easily return from this recursive helper with
        // the parent, we use an unsafe trick: we wrap the parent in a &mut and
        // mutate in place.
        //
        // Actually, since this is a static layout (not truly interactive tree),
        // we just use child() calls directly.
        *parent = std::mem::replace(parent, div()).child(row);

        if entry.kind == FileKind::Directory && entry.expanded {
            for child_entry in entry.children {
                Self::render_entry_into(
                    parent,
                    child_entry,
                    depth + 1,
                    indent,
                    show_sizes,
                    selected_path,
                    theme,
                );
            }
        }
    }
}

// ── Types ──

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    File,
    Directory,
}

pub struct FileEntry {
    name: String,
    kind: FileKind,
    children: Vec<FileEntry>,
    expanded: bool,
    size: Option<String>,
    modified: Option<String>,
}

#[allow(dead_code)]
impl FileEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FileKind::File,
            children: Vec::new(),
            expanded: false,
            size: None,
            modified: None,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FileKind::Directory,
            children: Vec::new(),
            expanded: false,
            size: None,
            modified: None,
        }
    }

    pub fn child(mut self, entry: FileEntry) -> Self {
        self.children.push(entry);
        self
    }

    pub fn expanded(mut self, v: bool) -> Self {
        self.expanded = v;
        self
    }

    pub fn size(mut self, s: impl Into<String>) -> Self {
        self.size = Some(s.into());
        self
    }

    pub fn modified(mut self, m: impl Into<String>) -> Self {
        self.modified = Some(m.into());
        self
    }
}

/// Simple heuristic: pick an emoji icon based on file extension.
fn file_icon_for(name: &str) -> &'static str {
    if let Some(ext) = name.rsplit('.').next() {
        match ext {
            "rs" => "🦀",
            "toml" | "yaml" | "yml" | "json" | "json5" => "⚙",
            "md" | "txt" | "rst" => "📝",
            "ts" | "tsx" | "js" | "jsx" => "🟨",
            "css" | "scss" | "less" => "🎨",
            "html" | "htm" => "🌐",
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => "🖼",
            "lock" => "🔒",
            "sh" | "bash" | "zsh" | "fish" | "ps1" => "💲",
            "py" => "🐍",
            "go" => "🐹",
            "c" | "cpp" | "h" | "hpp" => "⚡",
            "java" | "kt" | "kts" => "☕",
            "wasm" => "🔮",
            _ => "📄",
        }
    } else {
        "📄"
    }
}
