

 I'll research the current UI architecture thoroughly before drafting a plan. Let me start with deep discovery. 

Research current UI architecture

Read [](file:///workspaces/zed/MODE.md)

Read [](file:///workspaces/zed/DX.md#1-1), lines 1 to 100

Read [](file:///workspaces/zed/essence)

Read [](file:///workspaces/zed/crates/agent)

Read [](file:///workspaces/zed/crates/agent_ui)

Read [](file:///workspaces/zed/crates/agent_settings)

Read [](file:///workspaces/zed/crates/agent/src)

Read [](file:///workspaces/zed/crates/agent_ui/src)

Read [](file:///workspaces/zed/crates/agent_settings/src)

Read [](file:///workspaces/zed/crates/workspace)

Read [](file:///workspaces/zed/crates/theme)

Read [](file:///workspaces/zed/essence/README.md#1-1), lines 1 to 50

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#1-1), lines 1 to 120

Read [](file:///workspaces/zed/crates/agent_ui/src/message_editor.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/crates/agent_ui/src/mode_selector.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/crates/agent_ui/src/profile_selector.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/crates/agent_settings/src/agent_profile.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/crates/workspace/src)

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#120-120), lines 120 to 250

Searched for text `struct AgentPanel` (`**/crates/agent_ui/src/agent_panel.rs`), 1 result

Searched for text `impl Render for AgentPanel` (`**/crates/agent_ui/src/agent_panel.rs`), 1 result

Searched for text `impl Panel for AgentPanel` (`**/crates/agent_ui/src/agent_panel.rs`), 1 result

Searched for text `fn render` (`**/crates/agent_ui/src/message_editor.rs`), 2 results

Searched for text `fn render` (`**/crates/agent_ui/src/mode_selector.rs`), 1 result

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#497-497), lines 497 to 600

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#3154-3154), lines 3154 to 3300

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#1908-1908), lines 1908 to 2000

Read [](file:///workspaces/zed/crates/agent_ui/src/message_editor.rs#1346-1346), lines 1346 to 1500

Read [](file:///workspaces/zed/crates/agent_ui/src/mode_selector.rs#100-100), lines 100 to 218

Read [](file:///workspaces/zed/crates/agent_settings/src/agent_profile.rs#100-100), lines 100 to 200

Read [](file:///workspaces/zed/crates/workspace/src/dock.rs#1-1), lines 1 to 100

Searched for regex `pub trait Panel|pub enum DockPosition|struct Dock` (`**/crates/workspace/src/dock.rs`), 4 results

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view)

Read [](file:///workspaces/zed/crates/agent_ui/src/ui)

Read [](file:///workspaces/zed/crates/theme/src)

Searched for text `render_toolbar` (`**/crates/agent_ui/src/agent_panel.rs`), 4 results

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#2334-2334), lines 2334 to 2550

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#1-1), lines 1 to 100

Searched for regex `impl Render for|struct ThreadView|fn render` (`**/crates/agent_ui/src/connection_view/thread_view.rs`), 20 results

Read [](file:///workspaces/zed/crates/theme/src/theme.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/crates/workspace/src/dock.rs#200-200), lines 200 to 250

Searched for text `enum ActiveView` (`**/crates/agent_ui/src/agent_panel.rs`), 1 result

Read [](file:///workspaces/zed/crates/agent_ui/src/agent_panel.rs#339-339), lines 339 to 400

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#260-260), lines 260 to 400

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#2594-2594), lines 2594 to 2900

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#3486-3486), lines 3486 to 3620

Searched for regex `left_dock|right_dock|bottom_dock|struct Workspace` (`**/crates/workspace/src/workspace.rs`), 15 results

Searched for regex `enum AgentType` (`**/crates/agent_ui/src/agent_panel.rs`), 1 result

Searched for text `impl Render for Workspace` (`**/crates/workspace/src/workspace.rs`), 1 result

Searched for text `CHAT_INPUT_MOOD_ICONS` (`**/crates/agent_ui/src/connection_view/thread_view.rs`), 3 results

Searched for regex `struct ThemeColors|pub fn colors` (`**/crates/theme/src/styles.rs`), no results

Searched for regex `struct AgentSettings|pub struct AgentProfileId|struct AgentProfileConfig` (`**/crates/agent_settings/src/agent_settings.rs`), 2 results

Searched for text `impl Render for ThreadView` (`**/crates/agent_ui/src/connection_view/thread_view.rs`), 1 result

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#180-180), lines 180 to 260

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#7626-7626), lines 7626 to 7800

Read [](file:///workspaces/zed/crates/workspace/src/workspace.rs#7429-7429), lines 7429 to 7550

Read [](file:///workspaces/zed/crates/agent_settings/src/agent_settings.rs#1-1), lines 1 to 120

Read [](file:///workspaces/zed/crates/theme/src/styles.rs)

Read [](file:///workspaces/zed/crates/workspace/src/workspace.rs#7550-7550), lines 7550 to 7700

Read [](file:///workspaces/zed/crates/theme/src/styles/colors.rs#1-1), lines 1 to 100

Searched for regex `enum ChatTarget` (`**/crates/agent_ui/src/connection_view/thread_view.rs`), 1 result

Read [](file:///workspaces/zed/crates/agent_ui/src/connection_view/thread_view.rs#155-155), lines 155 to 195

Read [](file:///workspaces/zed/crates/theme/src/styles/colors.rs#100-100), lines 100 to 300

Read [](file:///workspaces/zed/crates/workspace/src/workspace.rs#1239-1239), lines 1239 to 1310

Searched for regex `project_panel|ProjectPanel` (`**/crates/workspace/src/workspace.rs`), no results

Searched for files matching `**/project_panel*`, 3 matches

Read [](file:///workspaces/zed/HOMEPAGE_TEXT.md#1-1), lines 1 to 50

Searched for regex `impl Panel for ProjectPanel|impl Render for ProjectPanel|struct ProjectPanel` (`**/crates/project_panel/src/project_panel.rs`), 3 results

Read [](file:///workspaces/zed/crates/project_panel/src/project_panel.rs#128-128), lines 128 to 200

Read [](file:///workspaces/zed/crates/project_panel/src/project_panel.rs#6393-6393), lines 6393 to 6440

Read [](file:///workspaces/zed/crates/project_panel/src/project_panel.rs#7040-7040), lines 7040 to 7090

Read [](file:///home/codespace/.vscode-remote/data/User/workspaceStorage/5a1cdf1-1/GitHub.copilot-chat/chat-session-resources/a50f080d-9a82-40e3-bc97-1b21463e66f2/toolu_bdrk_014WsB2owyda7ps2MfV7w4GW__vscode-1772309599208/content.txt)

Great, I have a thorough understanding of the codebase. Let me now clarify a few things about scope and priorities before drafting the plan. 

Asked 4 questions (Scope, AI Profiles, Center Panel, Social Share)

Let me research the existing social media integration code you mentioned. 

Research social integrations code

Read [](file:///workspaces/zed/integrations)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent)

Read [](file:///workspaces/zed/integrations/agent/src)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/integrations)

Read [](file:///workspaces/zed/integrations/agent/src/channels)

Read [](file:///workspaces/zed/integrations/agent/src/integrations)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/integrations/src)

Read [](file:///workspaces/zed/integrations/agent/src/channels/mod.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/integrations/agent/src/channels/traits.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/integrations/agent/src/channels/discord.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/src/channels/whatsapp.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/src/channels/telegram.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/src/channels/slack.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/src/channels/signal.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/Cargo.toml#1-1), lines 1 to 100

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/Cargo.toml)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/lib.rs)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/discord.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/telegram.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/whatsapp.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/slack.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/traits.rs)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/signal.rs)

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/message.rs#1-1), lines 1 to 120

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/matrix.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/teams.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/agent/Cargo.toml#100-100), lines 100 to 271

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/integrations/src/messaging.rs#1-1), lines 1 to 200

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/integrations/src/messaging.rs#200-200), lines 200 to 336

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/integrations/src/lib.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/integrations/tool-tui/crates/agent/channels/src/google_chat.rs#1-1), lines 1 to 150

Read [](file:///workspaces/zed/integrations/agent/src/channels/whatsapp_web.rs#1-1), lines 1 to 100

Read [](file:///workspaces/zed/integrations/agent/src/channels/irc.rs#1-1), lines 1 to 50

Read [](file:///workspaces/zed/integrations/agent/src/channels/nostr.rs#1-1), lines 1 to 50

Read [](file:///workspaces/zed/integrations/agent/src/channels/dingtalk.rs#1-1), lines 1 to 50

Read [](file:///workspaces/zed/integrations/agent/src/channels/qq.rs#1-1), lines 1 to 50

Read [](file:///workspaces/zed/integrations/agent/src/channels/mattermost.rs#1-1), lines 1 to 50

Read [](file:///workspaces/zed/integrations/agent/src/channels/imessage.rs#1-1), lines 1 to 50

Read [](file:///home/codespace/.vscode-remote/data/User/workspaceStorage/5a1cdf1-1/GitHub.copilot-chat/chat-session-resources/a50f080d-9a82-40e3-bc97-1b21463e66f2/toolu_bdrk_01VWWFM9JuAffDPWc724EECx__vscode-1772309599282/content.txt)

Now I have full context on both the UI architecture and the social integrations. Here's the comprehensive plan:

---

## Plan: DX UI Overhaul — Full Redesign

**TL;DR:** Transform Zed's editor-first layout into DX's AI-first layout across 7 major workstreams. The AI panel becomes the default center view (moving to right dock only when files are open). A Notion-style sidebar replaces the collapsed left icon. Six AI profiles each transform the panel's content. Mood toggles swap input action buttons per media type. Session history lives on the right edge. Social sharing adapts the existing 15+ platform integrations from integrations into GPUI. Design direction: modern & polished with cleaner spacing, refined typography, and rounded surfaces.

---

### Step 1: AI Panel — Center-First Layout with Dock Fallback

**What changes:** The `AgentPanel` currently lives exclusively in the right `Dock`. It needs a dual-mode existence: when no files are open, it renders as the **center workspace content** (full-width, input centered with `max-w-2xl`, rounded borders). When the user opens a file, it transitions to the right dock as it does today.

- Modify workspace.rs `Workspace::render()` (around line 7429) — add a condition: if no editor panes have open files AND the agent panel exists, render the agent panel *in the center PaneGroup area* instead of inside the right dock.
- Add a new `WorkspaceLayout` enum or flag on `Workspace` to track `CenterMode::AiDefault` vs `CenterMode::Editor` — toggled when the first file tab opens or the last file tab closes.
- In the center-mode rendering path, wrap the `AgentPanel` content in a centered container: `div().size_full().flex().items_center().justify_center()` → inner `div().max_w(px(720)).w_full().rounded_xl().border_1().border_color(cx.theme().colors().border)`.
- The `render_message_editor()` in thread_view.rs (line 2594) needs a `centered: bool` prop that adds `rounded_xl()`, `mx_auto()`, `max_w(px(680))`, and `shadow_md()` to the input container when in center mode.
- The transition animation between center and dock can use GPUI's `AnimationExt` for a subtle fade/slide.

### Step 2: Six AI Profiles — Profile-Driven Panel Transformation

**What changes:** Currently there are 3 builtin profiles (`WRITE`, `ASK`, `MINIMAL`) in agent_profile.rs (line 17). The profile system needs to expand to 6, and each profile must transform the entire panel content — not just the model/tools, but the UI layout itself.

- Add new `AgentProfileId` constants: `PLAN`, `STUDY`, `DEEP_RESEARCH`, `SEARCH` alongside existing `WRITE` (→ rename to `AGENT`), `ASK`, `MINIMAL` (→ deprecate or keep as internal).
- Create a new enum `ProfileUiMode` in agent_panel.rs:
  ```
  Agent → standard ThreadView (current behavior)
  Ask → simplified ThreadView (no tool output, just Q&A)
  Plan → new PlanView with step-by-step checklist rendering (inspired by Copilot Plan screenshot)
  Study → new StudyView with sources sidebar + chat + studio column (inspired by NotebookLM screenshot)
  DeepResearch → stub panel with "Coming Soon" + description
  Search → stub panel with "Coming Soon" + description
  ```
- **PlanView** (new file `crates/agent_ui/src/plan_view.rs`): Renders a vertical list of plan steps with checkboxes, expand/collapse per step, and a diff-preview pane. The message editor at bottom stays but action buttons change to "Generate Plan" / "Refine" / "Execute".
- **StudyView** (new file `crates/agent_ui/src/study_view.rs`): 3-column layout within the panel — left column for Sources (list of documents/URLs with checkboxes), center for Chat, right for Studio outputs (generated summaries, study guides, audio overviews). Modeled after the NotebookLM screenshot.
- **Stub panels** (new file `crates/agent_ui/src/coming_soon_view.rs`): Simple centered card with icon, title ("Deep Research" / "Search"), description text, and a muted "Coming Soon" badge.
- Modify the profile selector in profile_selector.rs to display all 6 profiles with distinct icons.
- In `AgentPanel::render()` (line 3154), match on the active profile's `ProfileUiMode` to choose which view to render.

### Step 3: Notion-Style Left Sidebar

**What changes:** The current left dock hosts `ProjectPanel` (file explorer) and collapses to a thin icon strip. It needs to become an always-expanded **navigation sidebar** with three zones: top nav bar, center page tree (Notion-style), and bottom dot-nav.

- Create a new `DxSidebar` panel struct (new file `crates/workspace/src/dx_sidebar.rs`) implementing the `Panel` trait. It replaces or wraps `ProjectPanel` as the default left dock content.
- **Top zone** (~48px): Home button, Search button, + New button — horizontal icon row. Inspired by the Zen browser sidebar top (screenshot reference).
- **Center zone** (flex-grow, scrollable): Notion-like tree structure with:
  - Section headers: "Recents", "Favorites", "Agents" (with Beta badge), "Private"
  - Tree items: icon + label + optional badge (e.g., notification count on Inbox)
  - Nested pages with indent levels
  - "New page" and "New agent" action items with `+` icon
  - Each item: `h_flex().gap_2().px_2().py_1().rounded_md().hover(bg: ghost_element_hover)`
- **Bottom zone** (~40px): Dot-navigation strip — a row of small colored dots representing open spaces/workspaces. Clicking a dot switches context. Inspired by Firefox's "Space" sidebar dots and the sidebar.png reference.
- Register `DxSidebar` in `Workspace::new()` alongside or replacing `ProjectPanel`. Set `is_open: true` by default with a reasonable default width (~240px).
- The `ProjectPanel` file tree can be embedded *within* the DxSidebar center zone as a collapsible section, so file navigation still works.

### Step 4: Mood/Media Toggle System

**What changes:** The mood icons (Text, Image, Audio, Video, Live, 3D, PDF) at thread_view.rs line 192 (`CHAT_INPUT_MOOD_ICONS`) already exist as toggles. They need to **transform the entire input area's action buttons** when switched.

- Define a `MoodActionSet` struct that maps each mood to its specific action buttons:
  ```
  Text → model selector, profile selector, thinking effort, enhance prompt
  Image → image model selector (DALL-E, Midjourney, Flux, etc.), aspect ratio, style preset, negative prompt toggle
  Audio → audio model selector, voice selector, duration, format (speech/music/sfx)
  Video → video model selector, duration, resolution, fps, aspect ratio
  Live → microphone selector, voice mode toggle, conversation history
  3D/AR/VR → 3D model selector, format (GLB/USDZ/OBJ), scene type
  PDF/Docs → doc type selector (PDF/DOCX/slides), template, page count
  ```
- Create `crates/agent_ui/src/mood_action_bar.rs` — a new component that renders the appropriate action buttons based on `selected_mood`.
- In `ThreadView::render_message_editor()` (line 2594), replace the current hardcoded left-side selectors with `MoodActionBar::new(self.selected_mood)`. The bar dynamically renders only the controls relevant to the active mood.
- The send button label/icon should also change per mood: "Send" for text, "Generate" for image/video/audio/3D, "Start" for live.
- Store `selected_mood: ChatInputMood` on `ThreadView` (already partially exists as the mood icon toggle state).

### Step 5: Session History — Right-Side Scrollable Panel

**What changes:** When the AI panel is in center mode, there's empty space on the right. Add a **session history rail** — a narrow vertical strip showing past conversation sessions, inspired by the Grok session-history screenshot.

- Create `crates/agent_ui/src/session_history_rail.rs` — a slim (200-280px) vertical panel that appears on the right side *within* the center AI view (not as a separate dock panel).
- Renders a `UniformList` of session cards, each showing: title (first message truncated), timestamp, mood icon badge, message count.
- Clicking a session loads it in the center chat.
- Group sessions by date: "Today", "Yesterday", "This Week", "Older".
- In center mode, the layout becomes: `h_flex()` → `[center chat (flex-grow)]` + `[session_history_rail (fixed width)]`.
- In dock mode (right panel), session history is accessible via a toggle button in the toolbar (already partially exists as `HistoryKind` in `ActiveView`).

### Step 6: Social Sharing — GPUI Integration

**What changes:** Adapt the existing CLI-based social integrations into the GPUI desktop app. The share popover at thread_view.rs line 2692 already renders platform icons. It needs to wire up to actual send logic.

- Create `crates/social_sharing/` — a new crate that provides a GPUI-compatible social sharing service. This adapts the `Channel` trait from traits.rs into a GPUI `Global` service.
- **Phase 1 platforms** (highest user demand): WhatsApp, Telegram, Discord, Slack, X/Twitter (new), Facebook Messenger (new).
- Port the `reqwest`-based REST implementations from channels — these are the most portable since they don't depend on framework-specific runtimes. Key files:
  - whatsapp.rs (WhatsApp Cloud API)
  - telegram.rs (Bot API)
  - discord.rs (REST API)
  - slack.rs (Web API)
- Create `SocialShareService` as a GPUI `Global`: stores authenticated channel configs, exposes `async fn share(platform, content) -> Result<DeliveryStatus>`.
- Add a **Connect Accounts** settings page (new file `crates/agent_ui/src/social_settings_view.rs`) — lists platforms with connect/disconnect buttons, OAuth flows via `cx.open_url()` + local callback server.
- The share popover in `ThreadView` calls `SocialShareService::share()` with the AI response content formatted per-platform (Markdown → platform-native formatting).
- Use `gpui::http_client` or the existing `HttpClientWithUrl` from the client crate rather than adding raw `reqwest` — keeps dependencies aligned with Zed's HTTP stack.

### Step 7: Visual Polish — Modern & Polished Direction

**What changes:** Apply the "modern & polished" design direction across all new and existing components.

- **Spacing**: Increase base padding from `p_1`/`p_2` to `p_2`/`p_3` in panel containers. Add `gap_2` minimum between toolbar items.
- **Typography**: Use `text_ui_sm` for secondary labels, `text_ui` (not `text_ui_xs`) for primary interactive elements. Add `font_weight(FontWeight::MEDIUM)` to section headers.
- **Borders & Surfaces**: Replace sharp-cornered containers with `rounded_lg()` or `rounded_xl()`. Add subtle `shadow_sm()` to floating elements (popovers, the centered chat input). Use `surface_background` color for elevated cards.
- **Input styling**: Centered chat input gets `rounded_xl().border_1().shadow_md()` with `p_3` internal padding. Placeholder text uses `text_placeholder` color.
- **Icon sizing**: Standardize toolbar icons to `IconSize::Small` (16px), action bar icons to `IconSize::Medium` (20px). Add consistent `gap_1` between icon groups.
- **Transitions**: Add `AnimationExt` fade-in for panel switches (profile changes, mood toggles). Duration: 150ms ease-out.
- **Theme tokens**: Add new tokens to colors.rs `ThemeColors`: `chat_input_background`, `chat_input_border`, `session_card_background`, `sidebar_section_header`, `mood_icon_active`.

---

**Verification**
1. Build with `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked`
2. Launch DX → confirm AI panel appears centered with rounded input when no files are open
3. Open a file → confirm AI panel transitions to right dock
4. Switch between all 6 profiles → confirm panel content changes (Agent/Ask show chat, Plan shows checklist, Study shows 3-column, Deep Research/Search show "Coming Soon")
5. Toggle each mood icon → confirm action buttons change accordingly
6. Verify left sidebar is expanded by default with Home/nav/dot-nav zones
7. Test social share → confirm at least one platform (Telegram) delivers a message
8. Run clippy to verify no warnings

**Decisions**
- Center panel uses a `WorkspaceLayout` flag on `Workspace`, not a new dock position — avoids breaking the existing 3-dock architecture
- Social sharing reuses channels REST implementations (not the `serenity`/`teloxide` SDK versions from tool-tui) — REST is simpler to port and has fewer dependencies
- Deep Research and Search get stub UI now — full implementation deferred
- `PlanView` and `StudyView` are new files in src, not modifications to `ThreadView` — keeps concerns separated
- Sidebar is a new `DxSidebar` panel that *contains* `ProjectPanel`, not a replacement — preserves existing file-tree functionality