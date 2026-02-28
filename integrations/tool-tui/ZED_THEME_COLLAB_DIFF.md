# Zed: Theme Builder, Collaboration UI, and Diff Manager

Comprehensive analysis of three major Zed features: the theme system, real-time collaboration UI, and Git diff manager.

## Overview

| System | Lines of Code | Primary Crates | Purpose |
|--------|--------------|----------------|---------|
| Theme System | ~1,143 | theme_selector, theme_importer, theme_extension | VS Code theme import, theme selection, icon themes |
| Diff Manager | ~3,898 | buffer_diff | Git diff visualization, staging/unstaging hunks, word-level diffs |
| Collaboration UI | ~6,060 | collab_ui | Real-time collaboration, channels, notifications, contact management |

---

## 1. Theme System

### Architecture

The theme system consists of three main components:

1. **Theme Selector** - UI for browsing and selecting themes
2. **Theme Importer** - CLI tool for converting VS Code themes to Zed format
3. **Theme Extension** - Extension system integration for loading custom themes

### Key Features

#### Theme Selection
- Fuzzy search through available themes
- Live preview when hovering over themes
- Separate selectors for color themes and icon themes
- Filters themes by appearance (light/dark)
- Persists selection to settings file

#### VS Code Theme Import
- Converts VS Code `.json` theme files to Zed format
- Maps VS Code color tokens to Zed's theme schema
- Handles both light and dark appearances
- Supports TextMate grammar scopes
- CLI tool: `theme_importer`

#### Icon Theme Support
- Separate icon theme system independent of color themes
- Loads icon themes from extensions
- Icon theme selector with fuzzy search
- Public icon theme documentation link

### Code Structure

```
theme_selector/
├── theme_selector.rs       # Main theme picker UI
└── icon_theme_selector.rs  # Icon theme picker UI

theme_importer/
├── main.rs                 # CLI entry point
├── vscode.rs              # VS Code theme parser
└── color.rs               # Color conversion utilities

theme_extension/
└── theme_extension.rs      # Extension proxy for theme loading
```

### Theme Selector Implementation

```rust
pub struct ThemeSelector {
    picker: Entity<Picker<ThemeSelectorDelegate>>,
}

pub struct ThemeSelectorDelegate {
    themes: Vec<ThemeMeta>,
    matches: Vec<StringMatch>,
    original_theme: ThemeName,
    selected_theme: Option<ThemeName>,
    // ...
}
```

**Key Methods:**
- `show_selected_theme()` - Applies theme preview
- `confirm()` - Persists theme selection to settings
- `dismissed()` - Reverts to original theme if cancelled
- `update_matches()` - Fuzzy search through themes

### Theme Importer Workflow

1. Parse VS Code theme JSON file
2. Extract color tokens and TextMate scopes
3. Map to Zed's theme schema structure
4. Generate Zed-compatible JSON with `$schema` reference
5. Output to file or stdout

```rust
pub struct VsCodeThemeConverter {
    vscode_theme: VsCodeTheme,
    theme_metadata: ThemeMetadata,
    color_overrides: IndexMap<String, String>,
}
```

### Extension Integration

The `theme_extension` crate provides a proxy between the extension system and theme registry:

```rust
impl ExtensionThemeProxy for ThemeRegistryProxy {
    fn list_theme_names(&self, theme_path: PathBuf, fs: Arc<dyn Fs>) -> Task<Result<Vec<String>>>;
    fn load_user_theme(&self, theme_path: PathBuf, fs: Arc<dyn Fs>) -> Task<Result<()>>;
    fn reload_current_theme(&self, cx: &mut App);
    fn list_icon_theme_names(&self, icon_theme_path: PathBuf, fs: Arc<dyn Fs>) -> Task<Result<Vec<String>>>;
    fn load_icon_theme(&self, icon_theme_path: PathBuf, icons_root_dir: PathBuf, fs: Arc<dyn Fs>) -> Task<Result<()>>;
}
```

### Theme Schema

Zed themes follow a structured schema at `https://zed.dev/schema/themes/v0.2.0.json`:

- **Appearance**: Light or Dark
- **Style Colors**: Background, foreground, borders, etc.
- **Syntax Colors**: Keywords, strings, comments, functions, etc.
- **UI Colors**: Editor, panel, status bar, etc.

---

## 2. Diff Manager (buffer_diff)

### Architecture

The diff manager provides Git integration at the buffer level, tracking changes between:
- **HEAD** (committed version)
- **Index** (staged version)
- **Working Tree** (current buffer content)

### Key Features

#### Diff Hunks
- Tracks added, modified, and deleted lines
- Word-level diffs within changed lines
- Hunk ranges in both buffer and base text coordinates
- Secondary diff support for staging workflow

#### Staging/Unstaging
- Stage individual hunks or all hunks
- Unstage hunks back to working tree
- Merge overlapping hunks intelligently
- Update index text on stage/unstage operations

#### Word-Level Diffs
- Configurable per-language via settings
- Uses language-aware tokenization
- Highlights specific changed words within lines
- Respects `max_word_diff_line_count` limit

#### Real-Time Updates
- Incremental diff computation on buffer edits
- Efficient hunk comparison algorithm
- Emits events for UI updates
- Tracks extended ranges for smooth scrolling

### Core Data Structures

```rust
pub struct BufferDiff {
    buffer_id: BufferId,
    inner: BufferDiffInner<language::BufferSnapshot>,
    secondary_diff: Option<Entity<BufferDiff>>,
}

pub struct BufferDiffInner<T> {
    base_text: T,
    hunks: SumTree<InternalDiffHunk>,
    pending_hunks: SumTree<InternalDiffHunk>,
    base_text_exists: bool,
    buffer_snapshot: text::BufferSnapshot,
}

pub struct DiffHunk {
    pub range: Range<Point>,
    pub diff_base_byte_range: Range<usize>,
    pub buffer_range: Range<Anchor>,
    pub base_word_diffs: Vec<Range<usize>>,
    pub buffer_word_diffs: Vec<Range<Anchor>>,
    pub secondary_status: DiffHunkSecondaryStatus,
}
```

### Hunk Status Types

```rust
pub enum DiffHunkStatusKind {
    Added,    // New lines in buffer
    Modified, // Changed lines
    Deleted,  // Removed lines
}

pub enum DiffHunkSecondaryStatus {
    NoSecondaryHunk,                    // No staging info
    HasSecondaryHunk,                   // Staged version exists
    OverlapsWithSecondaryHunk,          // Partial staging
    SecondaryHunkAdditionPending,       // Pending stage
    SecondaryHunkRemovalPending,        // Pending unstage
}
```

### Staging Workflow

```rust
impl BufferDiff {
    pub fn stage_or_unstage_hunks(
        &mut self,
        stage: bool,
        hunks: &[DiffHunk],
        buffer: &text::BufferSnapshot,
        file_exists: bool,
        cx: &mut Context<Self>,
    ) -> Option<Rope>;
}
```

**Staging Process:**
1. Identify hunks to stage from working tree
2. Compute new index text by replacing hunk ranges
3. Merge overlapping hunks intelligently
4. Update secondary diff (index vs HEAD)
5. Emit events for UI updates

**Unstaging Process:**
1. Identify hunks to unstage from index
2. Replace with HEAD version of the text
3. Update pending hunks tracking
4. Recompute diffs

### Diff Computation

```rust
fn compute_hunks(
    diff_base: Option<(Arc<str>, Rope)>,
    buffer: &text::BufferSnapshot,
    diff_options: Option<DiffOptions>,
) -> SumTree<InternalDiffHunk>;
```

**Algorithm:**
1. Use libgit2's `GitPatch` for line-level diff
2. Process each hunk from the patch
3. Track buffer row divergence for accurate positioning
4. Compute word-level diffs if enabled
5. Build SumTree for efficient range queries

### Word Diff Algorithm

```rust
pub struct DiffOptions {
    pub language_scope: Option<language::LanguageScope>,
    pub max_word_diff_line_count: usize,
}
```

- Only computed for hunks with matching line counts
- Uses language-specific tokenization
- Highlights changed tokens within lines
- Configurable via `word_diff_enabled` setting

### Events

```rust
pub enum BufferDiffEvent {
    DiffChanged(DiffChanged),
    LanguageChanged,
    HunksStagedOrUnstaged(Option<Rope>),
}

pub struct DiffChanged {
    pub changed_range: Option<Range<text::Anchor>>,
    pub base_text_changed_range: Option<Range<usize>>,
    pub extended_range: Option<Range<text::Anchor>>,
}
```

### Performance Optimizations

- **SumTree**: O(log n) hunk lookups by range
- **Incremental Updates**: Only recompute changed regions
- **Anchor-Based Ranges**: Stable across buffer edits
- **Cursor-Based Iteration**: Efficient sequential access
- **Lazy Word Diffs**: Only computed when needed

---

## 3. Collaboration UI (collab_ui)

### Architecture

Real-time collaboration system built on Zed's multiplayer infrastructure, providing:
- Channel-based communication
- Contact management
- Presence indicators
- Notifications
- Screen sharing

### Key Features

#### Channel System
- Create public or private channels
- Invite members with role-based permissions (Admin, Member, Guest)
- Channel notes for shared documentation
- Channel links for easy sharing
- Nested channel hierarchy

#### Contact Management
- Search and add contacts by username
- Send/accept contact requests
- View contact online status
- Contact request status tracking

#### Collaboration Panel
- Tree view of channels and contacts
- Expandable/collapsible sections
- Drag-and-drop channel organization
- Context menus for actions
- Real-time presence updates

#### Notifications
- In-app notification panel
- Contact request notifications
- Channel invite notifications
- Mention notifications
- Dismissible notification items

### Code Structure

```
collab_ui/src/
├── collab_ui.rs              # Main UI coordinator
├── collab_panel.rs           # Left panel with channels/contacts
├── channel_modal.rs          # Channel member management
├── contact_finder.rs         # Contact search and invite
├── notification_panel.rs     # Notification center
└── collab_panel/
    ├── channel_modal.rs      # Channel settings modal
    └── contact_finder.rs     # Contact search modal
```

### Channel Modal

```rust
pub struct ChannelModal {
    picker: Entity<Picker<ChannelModalDelegate>>,
    channel_store: Entity<ChannelStore>,
    channel_id: ChannelId,
}

pub enum Mode {
    ManageMembers,  // View and manage existing members
    InviteMembers,  // Search and invite new members
}
```

**Features:**
- Toggle between manage and invite modes
- Fuzzy search for users
- Role management (Admin, Member, Guest)
- Remove members
- Public/private channel toggle
- Copy channel link

**Member Actions:**
- Promote to Admin
- Demote to Member
- Demote to Guest
- Remove from Channel

### Contact Finder

```rust
pub struct ContactFinder {
    picker: Entity<Picker<ContactFinderDelegate>>,
}

pub struct ContactFinderDelegate {
    potential_contacts: Arc<[Arc<User>]>,
    user_store: Entity<UserStore>,
    selected_index: usize,
}
```

**Contact Request Status:**
- `None` - No relationship
- `RequestSent` - Pending outgoing request
- `RequestReceived` - Pending incoming request
- `RequestAccepted` - Active contact

### Collaboration Panel

The main panel provides a tree view of:
- **Channels Section**: All channels user has access to
- **Contacts Section**: Online and offline contacts
- **Direct Messages**: 1-on-1 conversations

**Interactions:**
- Click to join channel
- Right-click for context menu
- Drag to reorder channels
- Expand/collapse sections
- View member count and online status

### Channel Permissions

```rust
pub enum ChannelRole {
    Admin,   // Full control: manage members, settings
    Member,  // Can post and read
    Guest,   // Read-only access
}

pub enum ChannelVisibility {
    Public,  // Anyone can join
    Members, // Invite-only
}
```

### Real-Time Updates

The collaboration UI subscribes to:
- `ChannelStore` - Channel membership changes
- `UserStore` - Contact status updates
- `NotificationStore` - New notifications
- Presence updates via multiplayer protocol

### Notification System

```rust
pub struct NotificationPanel {
    notification_list: Entity<UniformList>,
    notification_store: Entity<NotificationStore>,
}
```

**Notification Types:**
- Contact requests
- Channel invites
- Mentions in channels
- Project shares
- Call invitations

**Actions:**
- Accept/Decline requests
- Dismiss notifications
- Navigate to source (channel, project, etc.)

### UI Components

**Channel List Item:**
- Channel icon (#)
- Channel name
- Member count
- Online indicator
- Unread badge

**Contact List Item:**
- User avatar
- Username
- Online status indicator
- Busy/Away status

**Notification Item:**
- Notification icon
- Message text
- Timestamp
- Action buttons (Accept/Decline)
- Dismiss button

---

## Integration Points

### Theme System ↔ Settings
- Persists theme selection to `settings.json`
- Watches for theme file changes
- Reloads theme on settings update

### Diff Manager ↔ Editor
- Provides gutter indicators for changed lines
- Enables inline diff view
- Supports "go to next/previous change"
- Integrates with Git panel for staging

### Collaboration UI ↔ Workspace
- Shares workspace state across collaborators
- Syncs cursor positions and selections
- Broadcasts file edits in real-time
- Manages shared project access

---

## Performance Considerations

### Theme System
- Lazy loading of theme files
- Caches parsed themes in memory
- Async theme switching to avoid UI blocking

### Diff Manager
- Incremental diff computation
- SumTree for O(log n) range queries
- Word diffs only for small hunks
- Debounced updates on rapid edits

### Collaboration UI
- Virtual scrolling for large channel lists
- Batched presence updates
- Efficient tree diffing for UI updates
- WebSocket connection pooling

---

## Testing

### Theme System Tests
- Theme conversion accuracy
- Color mapping correctness
- Schema validation

### Diff Manager Tests
- Hunk computation accuracy
- Staging/unstaging correctness
- Word diff algorithm
- Edge cases (empty files, large files)

### Collaboration UI Tests
- Channel creation/deletion
- Member management
- Contact request flow
- Notification delivery

---

## Future Enhancements

### Theme System
- Theme marketplace integration
- Live theme editing
- Theme inheritance/composition
- Automatic light/dark mode switching

### Diff Manager
- Semantic diff (function-level changes)
- Conflict resolution UI
- Blame annotations
- Diff statistics

### Collaboration UI
- Video/audio calls
- Screen sharing improvements
- Threaded conversations
- Rich message formatting
- File sharing in channels

---

## Summary

These three systems showcase Zed's commitment to developer experience:

1. **Theme System** - Flexible, extensible theming with VS Code compatibility
2. **Diff Manager** - Sophisticated Git integration with staging workflow
3. **Collaboration UI** - Real-time multiplayer editing with rich social features

Total: ~11,101 lines of Rust code implementing production-ready features with GPUI.
