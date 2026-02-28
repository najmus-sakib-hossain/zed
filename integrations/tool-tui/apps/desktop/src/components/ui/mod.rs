// ─── DX Extended Component Library ──────────────────────────────────────────
// Desktop-grade UI components built with GPUI. Organized into three tiers:
//   1. Foundation  – layout primitives, helpers, progress, misc utilities
//   2. Interactive – alerts, dialogs, forms, selects, switches, popovers
//   3. Desktop     – app shells, docks, editors, toolbars, file explorers…
//
// Re-exports below give consumers a flat import surface:
//   use crate::components::ui::{Modal, Tabs, TreeView, …};
// ─────────────────────────────────────────────────────────────────────────────

// Library re-exports – consumers will pull in what they need.
#![allow(unused_imports)]

// ── Foundation ───────────────────────────────────────────────────────────────
pub mod helpers;
pub mod layout;
pub mod misc;
pub mod progress;

// ── Interactive components ───────────────────────────────────────────────────
pub mod alert;
pub mod form;
pub mod popover;
pub mod select;
pub mod sheet;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod toggle;

// ── Desktop-specific components ──────────────────────────────────────────────
pub mod activity_bar;
pub mod app_shell;
pub mod calendar;
pub mod color_swatch;
pub mod command_bar;
pub mod desktop_extras;
pub mod dock;
pub mod editor;
pub mod file_explorer;
pub mod indicators;
pub mod list_items;
pub mod menu_bar;
pub mod modal;
pub mod nav_menu;
pub mod notification;
pub mod slider_native;
pub mod split_pane;
pub mod status_bar;
pub mod toolbar;
pub mod tree_view;
pub mod widgets;
pub mod workspace_tabs;

// ─── Re-exports ─────────────────────────────────────────────────────────────
// Foundation
pub use helpers::with_alpha;
pub use layout::{
    AspectRatio, Center, Container, HStack, ResizableDirection, ResizablePanel, Spacer, StackAlign,
    VStack,
};
pub use misc::{Breadcrumb, EmptyState, HoverCard, Pagination, Stat, StatTrend};
pub use progress::{Progress, ProgressSize, Skeleton};

// Interactive
pub use alert::{Alert, AlertDialog, AlertVariant, Dialog, Toast, ToastVariant, Toaster};
pub use form::{FormField, FormGroup, SettingsRow};
pub use popover::{
    CommandItem, CommandPalette, ContextMenu, ContextMenuItem, DropdownMenu, DropdownMenuItem,
    Popover, Tooltip, TooltipSide,
};
pub use select::{Select, SelectOption};
pub use sheet::{Collapsible, ScrollArea, Sheet, SheetSide};
pub use switch::{Checkbox, RadioGroup, RadioOrientation, Switch};
pub use table::{DataTable, List, Table};
pub use tabs::{Accordion, TabOrientation, Tabs};
pub use toggle::{Toggle, ToggleRow, ToggleSize};

// Desktop
pub use activity_bar::{ActivityBar, ActivityBarItem};
pub use app_shell::{AppShell, ContentArea, PageHeader};
pub use calendar::{Calendar, DatePicker, TimePicker};
pub use color_swatch::{ColorPalette, ColorSwatch, GradientBar, ThemePreview};
pub use command_bar::{CommandBar, CommandBarAction, CommandBarGroup, Spotlight, SpotlightItem};
pub use desktop_extras::{
    Banner, BannerVariant, Callout, CalloutVariant, CodeBlock, KeyCombo, Spinner,
};
pub use dock::{
    Dock, DockPosition, DockTab, ResizeDirection, ResizeHandle, Splitter, SplitterDirection,
    WindowControls,
};
pub use editor::{Minimap, TermLine, TermLineKind, TerminalOutput};
pub use file_explorer::{FileEntry, FileExplorer, FileKind};
pub use indicators::{Chip, ChipVariant, DotIndicator, DotStatus, KeyboardShortcut, Tag};
pub use list_items::{EmptyList, FileItem, ListItem};
pub use menu_bar::{DataGrid, GridColumn, MenuBar, MenuBarItem, Slider, SortDirection};
pub use modal::{ConfirmDialog, Drawer, DrawerSide, Modal, Popconfirm};
pub use nav_menu::{NavBadge, NavBadgeVariant, NavItem, NavMenu, NavSection};
pub use notification::{
    Notification, NotificationPosition, NotificationStack, NotificationVariant,
};
pub use slider_native::{RangeSlider, SliderNative, SliderSize};
pub use split_pane::{Panel, SplitDirection, SplitPane};
pub use status_bar::{StatusBar, StatusBarItem};
pub use toolbar::{Toolbar, ToolbarButton, ToolbarGroup, ToolbarSeparator};
pub use tree_view::{TreeNode, TreeView};
pub use widgets::{
    StepItem, StepStatus, Stepper, Steps, Timeline, TimelineItem, ToggleGroup, ToggleGroupItem,
};
pub use workspace_tabs::{TabGroup, WorkspaceTab, WorkspaceTabs};
