# DX UI Overhaul — Master TODO

## Part 1: Center AI Panel + Rounded Input [IN PROGRESS]
- [ ] Add `center_ai_mode` state to `Workspace` struct
- [ ] Modify `Workspace::render()` to show AgentPanel centered when no files open
- [ ] Add `is_centered` prop to `AgentPanel` render path
- [ ] Style chat input: `max_w(680px)`, `rounded_xl()`, `border_1()`, `shadow_md()`, `mx_auto()`
- [ ] Wire file open/close events to toggle `center_ai_mode`
- [ ] Build and verify

## Part 2: Six AI Profiles [QUEUED]
- [ ] Add PLAN, STUDY, DEEP_RESEARCH, SEARCH profile IDs
- [ ] Create `PlanView` component
- [ ] Create `StudyView` component (3-column: sources/chat/studio)
- [ ] Create `ComingSoonView` stub for Deep Research & Search
- [ ] Profile switcher UI with 6 entries + distinct icons
- [ ] Wire profile switch to transform entire panel content

## Part 3: Notion-Style Left Sidebar [QUEUED]
- [ ] Create `DxSidebar` panel struct
- [ ] Top zone: Home, Search, + New buttons
- [ ] Center zone: Notion-style page tree with sections
- [ ] Bottom zone: Dot-nav workspace switcher
- [ ] Register as default left dock panel (expanded)
- [ ] Embed ProjectPanel as collapsible section

## Part 4: Mood/Media Toggle System [QUEUED]
- [ ] Define `MoodActionSet` per mood (Text/Image/Audio/Video/Live/3D/PDF)
- [ ] Create `MoodActionBar` component
- [ ] Wire mood toggle to swap input action buttons
- [ ] Change send button label per mood

## Part 5: Session History Rail [QUEUED]
- [ ] Create `SessionHistoryRail` component
- [ ] Group sessions by date
- [ ] Show in center mode on right side
- [ ] Click to load session

## Part 6: Social Sharing (GPUI) [QUEUED]
- [ ] Create `social_sharing` crate
- [ ] Port REST implementations from integrations/agent/src/channels/
- [ ] Create `SocialShareService` GPUI Global
- [ ] Connect Accounts settings page
- [ ] Wire share popover to actual send logic

## Part 7: Voice Overlay (Wispr Flow) [QUEUED]
- [ ] Bottom-center rounded voice input overlay
- [ ] Hotkey trigger system
- [ ] Local STT model integration
- [ ] Waveform/orb visualization in GPUI

## Part 8: AI Face Widget [QUEUED]
- [ ] Port SVG face from www-forge-token to GPUI
- [ ] Emotion system (happy/thinking/listening etc.)
- [ ] Eye tracking (mouse follow)
- [ ] Click to open mini AI panel
- [ ] Bottom-center always-visible placement

## Part 9: Background Agent Spawning [QUEUED]
- [ ] Integrate agent daemon from integrations/agent
- [ ] Local + VPS spawn support
- [ ] 24/7 background Ollama model
- [ ] Agent management UI

## Part 10: Computer Use Integration [QUEUED]
- [ ] Port computer-use sidecar from integrations/agent
- [ ] Mouse/keyboard/screenshot control
- [ ] Safety boundaries and allowlists

## Part 11: Visual Polish Pass [QUEUED]
- [ ] Spacing refinements
- [ ] Typography hierarchy
- [ ] New theme color tokens
- [ ] Animation transitions (150ms ease-out)
