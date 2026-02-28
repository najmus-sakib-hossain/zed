# DX Integrations: Universal Control Plane

> **"Connect to Everything. Control Anything. Zero Dependencies."**

DX provides native Rust integrations for all major services, protocols, and platforms. Each integration is built with token efficiency and performance in mind.

---

## Integration Categories

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DX INTEGRATION ECOSYSTEM                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         PROTOCOLS & STANDARDS                         │   │
│  │  LSP • MCP • Webhooks • WebSocket • HTTP/2 • gRPC                    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                              VOICE & AUDIO                            │   │
│  │  ElevenLabs • Voice Wake • Talk Mode • Shazam • Spotify • Sonos     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                             AUTOMATION                                │   │
│  │  Zapier • N8N • Cron • Gmail Pub/Sub • Webhooks • Answer Call       │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                          BROWSER & CANVAS                             │   │
│  │  Chrome/Chromium CDP • Canvas A2UI • Screen Capture                  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                            PRODUCTIVITY                               │   │
│  │  Notion • GitHub • Obsidian • Trello • Things 3 • Bear • Notes      │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                            COMMUNICATION                              │   │
│  │  Email • Gmail • Twitter/X • Answer Call                             │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                           MEDIA & CAPTURE                             │   │
│  │  Photo/Video • Screen Capture • GIF Finder • Weather                 │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                          SECURITY & HOME                              │   │
│  │  1Password • IoT • Smart Home • Apple Reminders                      │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Protocols & Standards

### 1.1 LSP (Language Server Protocol)

DX includes a full LSP server for IDE integration.

```sr
# ~/.dx/config/lsp.sr

[lsp]
enabled = true
port = 8790

[lsp.capabilities]
completion = true
hover = true
definition = true
references = true
diagnostics = true
formatting = true
code_actions = true

[lsp.languages]
sr = { analyzer = "dx-serializer" }
md = { analyzer = "dx-markdown" }
```

**CLI Commands:**
```bash
# Start LSP server
dx lsp --port 8790

# Connect from VS Code
# Extension auto-connects to running LSP
```

### 1.2 MCP (Model Context Protocol)

Native MCP server for AI tool integration.

```sr
# ~/.dx/config/mcp.sr

[mcp]
enabled = true
transport = "stdio"  # stdio, http, ws

[mcp.server]
name = "dx-agent"
version = "1.0.0"
capabilities = ["tools", "resources", "prompts"]

[mcp.tools]
browser = true
filesystem = true
shell = true
memory = true

[mcp.resources]
files = true
skills = true
config = true
```

**CLI Commands:**
```bash
# Start MCP server (stdio)
dx mcp serve

# Start MCP server (HTTP)
dx mcp serve --transport http --port 8791

# List available tools
dx mcp tools

# Call a tool
dx mcp call browser.navigate --url "https://example.com"
```

### 1.3 Webhooks

Receive and process external webhooks.

```sr
# ~/.dx/config/webhooks.sr

[webhooks]
enabled = true
port = 8792
auth = "bearer"  # none, bearer, hmac

[webhooks.endpoints]
github = { path = "/webhook/github", secret = "$GITHUB_WEBHOOK_SECRET" }
stripe = { path = "/webhook/stripe", secret = "$STRIPE_WEBHOOK_SECRET" }
custom = { path = "/webhook/custom", handler = "skills/webhook-handler.lua" }

[webhooks.security]
allowed_ips = ["0.0.0.0/0"]  # Or specific IPs
rate_limit = 100  # requests per minute
```

**CLI Commands:**
```bash
# Start webhook server
dx webhooks serve --port 8792

# Test webhook endpoint
dx webhooks test github --payload '{"event": "push"}'

# List received webhooks
dx webhooks list --last 10
```

---

## 2. Voice & Audio

### 2.1 ElevenLabs (TTS)

Premium text-to-speech integration.

```sr
# ~/.dx/config/tts.sr

[tts]
provider = "elevenlabs"  # elevenlabs, openai, edge, local
auto_mode = "smart"  # always, never, smart (based on length)
max_text_length = 1500
summarize_long = true

[tts.elevenlabs]
api_key = "$ELEVENLABS_API_KEY"
voice_id = "pMsXgVXv3BLzUgSXRplE"  # Default voice
model_id = "eleven_multilingual_v2"
stability = 0.5
similarity_boost = 0.75
style = 0.0
speed = 1.0

[tts.openai]
api_key = "$OPENAI_API_KEY"
model = "gpt-4o-mini-tts"
voice = "alloy"  # alloy, echo, fable, onyx, nova, shimmer

[tts.edge]
voice = "en-US-MichelleNeural"
language = "en-US"
output_format = "audio-24khz-48kbitrate-mono-mp3"
```

**CLI Commands:**
```bash
# Speak text
dx speak "Hello, I am DX"

# Speak with specific voice
dx speak "Welcome" --voice nova --provider openai

# Convert file to speech
dx speak --file message.txt --output speech.mp3
```

### 2.2 Voice Wake

Always-on voice activation.

```sr
# ~/.dx/config/voice-wake.sr

[voice_wake]
enabled = true
wake_word = "hey dx"
sensitivity = 0.7
timeout_ms = 5000

[voice_wake.model]
type = "whisper"  # whisper, vosk, porcupine
size = "tiny"  # tiny, base, small
device = "default"

[voice_wake.actions]
on_wake = "talk_mode"  # talk_mode, command, custom
custom_handler = "skills/wake-handler.lua"
```

### 2.3 Shazam (Song Recognition)

Identify music playing around you.

```sr
# ~/.dx/config/shazam.sr

[shazam]
enabled = true
auto_listen_duration = 10  # seconds

[shazam.output]
include_lyrics = true
include_album_art = true
spotify_link = true
```

**CLI Commands:**
```bash
# Identify current song
dx shazam

# Listen for specific duration
dx shazam --duration 15

# Identify from audio file
dx shazam --file recording.mp3
```

### 2.4 Spotify

Music playback control.

```sr
# ~/.dx/config/spotify.sr

[spotify]
enabled = true
auth_type = "oauth"  # oauth, api_key
client_id = "$SPOTIFY_CLIENT_ID"
client_secret = "$SPOTIFY_CLIENT_SECRET"

[spotify.features]
playback_control = true
playlist_management = true
search = true
recommendations = true
```

**CLI Commands:**
```bash
# Current playback
dx spotify now

# Play/pause
dx spotify play
dx spotify pause

# Search and play
dx spotify search "Taylor Swift" --play

# Add to playlist
dx spotify add-to-playlist "My Playlist" --current
```

### 2.5 Sonos

Multi-room audio control.

```sr
# ~/.dx/config/sonos.sr

[sonos]
enabled = true
discovery = "auto"  # auto, manual
household_id = ""  # Optional

[sonos.rooms]
living_room = { ip = "192.168.1.100" }
bedroom = { ip = "192.168.1.101" }

[sonos.features]
volume_control = true
grouping = true
favorites = true
tts_announcements = true
```

**CLI Commands:**
```bash
# List rooms
dx sonos rooms

# Play in room
dx sonos play --room "Living Room"

# Set volume
dx sonos volume 50 --room "Living Room"

# Group rooms
dx sonos group "Living Room" "Bedroom"

# Announce
dx sonos announce "Dinner is ready" --all
```

---

## 3. Automation

### 3.1 Zapier

Trigger Zapier workflows.

```sr
# ~/.dx/config/zapier.sr

[zapier]
enabled = true
webhook_url = "$ZAPIER_WEBHOOK_URL"

[zapier.triggers]
new_task = { zap_id = "abc123", data_format = "json" }
daily_report = { zap_id = "def456", schedule = "0 9 * * *" }
```

**CLI Commands:**
```bash
# Trigger a zap
dx zapier trigger new_task --data '{"task": "Review PR"}'

# List configured zaps
dx zapier list
```

### 3.2 N8N

Self-hosted automation workflows.

```sr
# ~/.dx/config/n8n.sr

[n8n]
enabled = true
base_url = "http://localhost:5678"
api_key = "$N8N_API_KEY"

[n8n.workflows]
backup = { id = "workflow_123" }
sync = { id = "workflow_456" }
```

**CLI Commands:**
```bash
# Execute workflow
dx n8n execute backup

# List workflows
dx n8n list

# Get workflow status
dx n8n status workflow_123
```

### 3.3 Cron (Scheduled Tasks)

Built-in task scheduler.

```sr
# ~/.dx/config/cron.sr

[cron]
enabled = true
timezone = "America/New_York"

[cron.jobs]
daily_standup = { schedule = "0 9 * * 1-5", command = "dx agent --message 'Daily standup reminder'" }
weekly_backup = { schedule = "0 2 * * 0", command = "dx backup create" }
health_check = { schedule = "*/5 * * * *", command = "dx health ping", silent = true }

[cron.jobs.custom_script]
schedule = "0 */4 * * *"
handler = "skills/sync-data.lua"
timeout = 300
```

**CLI Commands:**
```bash
# List scheduled jobs
dx cron list

# Run job manually
dx cron run daily_standup

# Add new job
dx cron add "test" --schedule "0 * * * *" --command "echo test"

# Remove job
dx cron remove test
```

### 3.4 Gmail Pub/Sub

Real-time email notifications.

```sr
# ~/.dx/config/gmail.sr

[gmail]
enabled = true
credentials_path = "~/.dx/credentials/gmail.json"

[gmail.pubsub]
project_id = "$GOOGLE_CLOUD_PROJECT"
topic = "gmail-notifications"
subscription = "dx-gmail-sub"

[gmail.filters]
important = { query = "is:important", action = "notify" }
from_team = { query = "from:team@company.com", action = "process" }

[gmail.actions]
notify = { handler = "skills/gmail-notify.lua" }
process = { handler = "skills/gmail-process.lua" }
```

**CLI Commands:**
```bash
# Setup Gmail Pub/Sub
dx gmail setup

# Test connection
dx gmail test

# List recent emails
dx gmail list --limit 10

# Process email
dx gmail process <message-id>
```

### 3.5 Answer Call

Phone call handling (via Twilio/similar).

```sr
# ~/.dx/config/answer-call.sr

[answer_call]
enabled = true
provider = "twilio"

[answer_call.twilio]
account_sid = "$TWILIO_ACCOUNT_SID"
auth_token = "$TWILIO_AUTH_TOKEN"
phone_number = "$TWILIO_PHONE_NUMBER"

[answer_call.behavior]
greeting = "Hello, this is DX assistant. How can I help you?"
transcribe = true
record = false
max_duration = 300
tts_voice = "elevenlabs"

[answer_call.routing]
default = "agent"  # agent, voicemail, forward
forward_to = "+1234567890"
```

**CLI Commands:**
```bash
# Start call listener
dx calls listen

# Make outbound call
dx calls dial +1234567890 --message "Your order is ready"

# List recent calls
dx calls list
```

---

## 4. Browser & Canvas

### 4.1 Browser (Chrome/Chromium Control)

Full browser automation via CDP.

```sr
# ~/.dx/config/browser.sr

[browser]
enabled = true
executable = "auto"  # auto, chrome, chromium, path
headless = true
user_data_dir = "~/.dx/browser-data"

[browser.viewport]
width = 1920
height = 1080
device_scale_factor = 1

[browser.profiles]
default = { path = "~/.dx/browser-data/default" }
work = { path = "~/.dx/browser-data/work" }

[browser.security]
allow_insecure = false
proxy = ""
```

**CLI Commands:**
```bash
# Open browser
dx browser open "https://example.com"

# Take screenshot
dx browser screenshot "https://example.com" --output page.png

# Extract data
dx browser extract "https://example.com" --selector ".content"

# Fill form
dx browser fill "https://example.com/form" --data '{"email": "test@test.com"}'

# Run automation script
dx browser run scripts/checkout.lua
```

### 4.2 Canvas (Visual Workspace + A2UI)

Agent-driven visual interface.

```sr
# ~/.dx/config/canvas.sr

[canvas]
enabled = true
default_size = [800, 600]
theme = "system"

[canvas.a2ui]
enabled = true
auto_render = true
snapshot_on_change = true

[canvas.components]
charts = true
tables = true
images = true
markdown = true
```

**CLI Commands:**
```bash
# Open canvas
dx canvas open

# Push content to canvas
dx canvas push --content '{"type": "markdown", "text": "# Hello"}'

# Take snapshot
dx canvas snapshot --output canvas.png

# Reset canvas
dx canvas reset
```

---

## 5. Productivity

### 5.1 Notion

Workspace and database management.

```sr
# ~/.dx/config/notion.sr

[notion]
enabled = true
api_key = "$NOTION_API_KEY"
api_version = "2025-09-03"

[notion.defaults]
database_id = ""  # Default database
page_parent = ""  # Default parent page
```

**CLI Commands:**
```bash
# Search
dx notion search "project notes"

# Create page
dx notion create-page "Meeting Notes" --parent <page-id>

# Query database
dx notion query <database-id> --filter '{"Status": "Active"}'

# Add to database
dx notion add <database-id> --properties '{"Name": "New Task", "Status": "Todo"}'
```

### 5.2 GitHub

Code, issues, and PR management.

```sr
# ~/.dx/config/github.sr

[github]
enabled = true
auth_type = "cli"  # cli (gh), token, oauth

[github.defaults]
owner = ""
repo = ""

[github.notifications]
watch_prs = true
watch_issues = true
watch_releases = true
```

**CLI Commands:**
```bash
# List PRs
dx github pr list --repo owner/repo

# Check PR status
dx github pr checks 55 --repo owner/repo

# Create issue
dx github issue create --title "Bug" --body "Description"

# List workflow runs
dx github runs --repo owner/repo

# View failed logs
dx github run-logs <run-id> --failed-only
```

### 5.3 Obsidian

Knowledge graph and notes.

```sr
# ~/.dx/config/obsidian.sr

[obsidian]
enabled = true
vault_path = "~/Documents/Obsidian"

[obsidian.features]
create_notes = true
search = true
backlinks = true
graph_view = false  # Requires Obsidian app
```

**CLI Commands:**
```bash
# Create note
dx obsidian create "Daily Note" --content "## Tasks\n- [ ] Task 1"

# Search notes
dx obsidian search "project ideas"

# List recent
dx obsidian list --recent 10

# Open in Obsidian
dx obsidian open "Daily Note"
```

### 5.4 Trello

Kanban board management.

```sr
# ~/.dx/config/trello.sr

[trello]
enabled = true
api_key = "$TRELLO_API_KEY"
token = "$TRELLO_TOKEN"

[trello.defaults]
board_id = ""
list_id = ""
```

**CLI Commands:**
```bash
# List boards
dx trello boards

# List cards
dx trello cards --board "My Board"

# Create card
dx trello create-card "New Task" --list "To Do"

# Move card
dx trello move-card <card-id> --list "Done"
```

### 5.5 Things 3 (macOS)

GTD task management.

```sr
# ~/.dx/config/things.sr

[things]
enabled = true  # macOS only
```

**CLI Commands:**
```bash
# Add task
dx things add "Review document" --when today

# List inbox
dx things inbox

# List today
dx things today

# Complete task
dx things complete <task-id>
```

### 5.6 Bear Notes (macOS)

Markdown note-taking.

```sr
# ~/.dx/config/bear.sr

[bear]
enabled = true  # macOS only
```

**CLI Commands:**
```bash
# Create note
dx bear create "Note Title" --content "Content here" --tags "work,project"

# Search
dx bear search "keyword"

# Open note
dx bear open <note-id>
```

### 5.7 Apple Notes (macOS/iOS)

Native Apple notes.

```sr
# ~/.dx/config/apple-notes.sr

[apple_notes]
enabled = true  # macOS only
account = "iCloud"  # or local account name
```

**CLI Commands:**
```bash
# Create note
dx notes create "Shopping List" --folder "Personal"

# List notes
dx notes list --folder "Work"

# Search
dx notes search "meeting"
```

### 5.8 Apple Reminders (macOS/iOS)

Task and reminder management.

```sr
# ~/.dx/config/apple-reminders.sr

[apple_reminders]
enabled = true  # macOS only
default_list = "Reminders"
```

**CLI Commands:**
```bash
# Add reminder
dx reminders add "Call John" --due "tomorrow 2pm"

# List reminders
dx reminders list

# Complete reminder
dx reminders complete <reminder-id>
```

---

## 6. Communication

### 6.1 Email (SMTP/IMAP)

Send and receive emails.

```sr
# ~/.dx/config/email.sr

[email]
enabled = true

[email.smtp]
host = "smtp.gmail.com"
port = 587
username = "$EMAIL_USERNAME"
password = "$EMAIL_PASSWORD"
tls = true

[email.imap]
host = "imap.gmail.com"
port = 993
username = "$EMAIL_USERNAME"
password = "$EMAIL_PASSWORD"
```

**CLI Commands:**
```bash
# Send email
dx email send --to "recipient@example.com" --subject "Hello" --body "Message"

# Read emails
dx email inbox --limit 10

# Search emails
dx email search "from:boss@company.com"
```

### 6.2 Twitter/X

Tweet, reply, and search.

```sr
# ~/.dx/config/twitter.sr

[twitter]
enabled = true
api_key = "$TWITTER_API_KEY"
api_secret = "$TWITTER_API_SECRET"
access_token = "$TWITTER_ACCESS_TOKEN"
access_secret = "$TWITTER_ACCESS_SECRET"
```

**CLI Commands:**
```bash
# Post tweet
dx twitter tweet "Hello World! #DX"

# Reply to tweet
dx twitter reply <tweet-id> "Thanks!"

# Search tweets
dx twitter search "rust programming"

# Get timeline
dx twitter timeline --limit 20
```

---

## 7. Media & Capture

### 7.1 Photo/Video Capture

Device camera control.

```sr
# ~/.dx/config/camera.sr

[camera]
enabled = true
default_device = "auto"

[camera.photo]
quality = "high"
format = "jpeg"

[camera.video]
quality = "1080p"
format = "mp4"
max_duration = 300
include_audio = true
```

**CLI Commands:**
```bash
# Take photo
dx camera snap --output photo.jpg

# Record video
dx camera record --duration 30 --output video.mp4

# List devices
dx camera devices
```

### 7.2 Screen Capture

Screenshot and screen recording.

```sr
# ~/.dx/config/screen.sr

[screen]
enabled = true

[screen.capture]
format = "png"
include_cursor = true

[screen.record]
format = "mp4"
quality = "high"
include_audio = true
```

**CLI Commands:**
```bash
# Screenshot
dx screen capture --output screenshot.png

# Capture region
dx screen capture --region "100,100,800,600"

# Record screen
dx screen record --duration 60 --output recording.mp4
```

### 7.3 GIF Finder

Search and download GIFs.

```sr
# ~/.dx/config/gif.sr

[gif]
enabled = true
provider = "giphy"  # giphy, tenor

[gif.giphy]
api_key = "$GIPHY_API_KEY"

[gif.tenor]
api_key = "$TENOR_API_KEY"
```

**CLI Commands:**
```bash
# Search GIFs
dx gif search "happy"

# Get random GIF
dx gif random "celebration"

# Download GIF
dx gif download <gif-id> --output party.gif
```

### 7.4 Weather

Forecasts and conditions.

```sr
# ~/.dx/config/weather.sr

[weather]
enabled = true
provider = "wttr"  # wttr (free), openmeteo
default_location = "auto"  # auto, city name, coordinates
units = "metric"  # metric, imperial
```

**CLI Commands:**
```bash
# Current weather
dx weather

# Forecast
dx weather forecast --days 5

# Specific location
dx weather --location "London"
```

---

## 8. Security & Smart Home

### 8.1 1Password

Secure credential management.

```sr
# ~/.dx/config/1password.sr

[onepassword]
enabled = true
account = "my.1password.com"
vault = "Personal"
```

**CLI Commands:**
```bash
# Get secret
dx 1password get "API Key"

# List items
dx 1password list --vault "Work"

# Inject secrets
dx 1password inject -- npm start
```

### 8.2 IoT & Smart Home

Home automation control.

```sr
# ~/.dx/config/smarthome.sr

[smarthome]
enabled = true

[smarthome.hue]
bridge_ip = "192.168.1.50"
username = "$HUE_USERNAME"

[smarthome.homeassistant]
url = "http://homeassistant.local:8123"
token = "$HA_TOKEN"
```

**CLI Commands:**
```bash
# List devices
dx home devices

# Control light
dx home light "Living Room" --on --brightness 80

# Run scene
dx home scene "Movie Night"

# Get sensor data
dx home sensor "Temperature" --room "Bedroom"
```

---

## Integration Configuration Summary

All integrations are configured via `.sr` files in `~/.dx/config/`:

```
~/.dx/config/
├── lsp.sr           # Language Server Protocol
├── mcp.sr           # Model Context Protocol
├── webhooks.sr      # Webhook endpoints
├── tts.sr           # Text-to-Speech (ElevenLabs, etc.)
├── voice-wake.sr    # Voice activation
├── shazam.sr        # Song recognition
├── spotify.sr       # Music playback
├── sonos.sr         # Multi-room audio
├── zapier.sr        # Zapier automation
├── n8n.sr           # N8N workflows
├── cron.sr          # Scheduled tasks
├── gmail.sr         # Gmail Pub/Sub
├── answer-call.sr   # Phone call handling
├── browser.sr       # Browser control
├── canvas.sr        # Visual workspace
├── notion.sr        # Notion integration
├── github.sr        # GitHub CLI
├── obsidian.sr      # Obsidian notes
├── trello.sr        # Trello boards
├── things.sr        # Things 3 (macOS)
├── bear.sr          # Bear Notes (macOS)
├── apple-notes.sr   # Apple Notes
├── apple-reminders.sr # Apple Reminders
├── email.sr         # SMTP/IMAP
├── twitter.sr       # Twitter/X
├── camera.sr        # Photo/Video capture
├── screen.sr        # Screen capture
├── gif.sr           # GIF finder
├── weather.sr       # Weather service
├── 1password.sr     # 1Password CLI
└── smarthome.sr     # IoT/Smart Home
```

---

## Token Efficiency in Integrations

Each integration is designed for maximum token efficiency:

| Integration | Data Format | Token Savings |
|-------------|-------------|---------------|
| All APIs | DX Serializer LLM Format | 52-73% |
| Documentation | DX Markdown | 10-80% |
| Config Files | .sr (Human Format) | 65% vs JSON |
| IPC/Storage | Machine Format | 65% + ~48ns |

Example - Notion API response optimization:

```
# Raw JSON from Notion API (~450 tokens)
{
  "object": "page",
  "id": "abc123",
  "created_time": "2024-02-01T10:00:00.000Z",
  "last_edited_time": "2024-02-15T14:30:00.000Z",
  "properties": {
    "Name": {
      "id": "title",
      "type": "title",
      "title": [{"type": "text", "text": {"content": "Meeting Notes"}}]
    }
  }
}

# DX LLM Format (~120 tokens, 73% savings)
notion.page:1[id=abc123 created=2024-02-01T10:00 edited=2024-02-15T14:30]
notion.page.props:1[Name:title="Meeting Notes"]
```

---

## CLI Unified Interface

All integrations follow a consistent CLI pattern:

```bash
dx <integration> <command> [options]

# Examples
dx notion search "query"
dx github pr list
dx spotify play
dx browser open "url"
dx cron list
dx 1password get "secret"
```

This unified interface makes DX the single control plane for all your tools and services.
