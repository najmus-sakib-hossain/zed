PS F:\Dx\crates\cli-onboard> cargo run
   Compiling dx-onboard v0.1.0 (F:\Dx\crates\cli-onboard)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.73s
     Running `F:\Dx\target\debug\dx-onboard.exe`
│ DX Onboarding  ────────────────────────────────────────────────────────────────────────────────╮
│                                                                                                │
│  Environment-aware onboarding + auth + provider/channel setup                                  │
│  Detected runtime: Real OS workstation                                                         │
│  Runtime hint: Best for desktop app + extension onboarding                                     │
│                                                                                                │
│  Component install policy: real OS => 5 components, VPS/Container => 2 components              │
│  Auth methods: email/password, GitHub OAuth, Google OAuth                                      │
│  Avatar preview: pixelated ANSI + ASCII art                                                    │
│                                                                                                │
├────────────────────────────────────────────────────────────────────────────────────────────────╯
│
│ Provider Pricing (models.dev)  ────────────────────────────────────────────╮
│                                                                            │
│  opencode => $1.4657/1M in | $8.1027/1M out                                │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────╯
│
│ Provider Connectivity  ────────────────────────────────────────────────────╮
│                                                                            │
│  opencode => unverified (no built-in probe for this provider yet)          │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────╯
│
│ Discovered Provider Models  ───────────────────────────────────────────────╮
│                                                                            │
│  opencode => ok (fetched model list)                                       │
│    - opencode:big-pickle                                                   │
│    - opencode:glm-5-free                                                   │
│    - opencode:minimax-m2.5-free                                            │
│    - opencode:trinity-large-preview-free                                   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────╯
│
✓ Choose model for provider opencode  glm-5-free
│
✓ Run hello prompt probe for opencode:glm-5-free?  Yes
│
│ Provider Hello Probe  ────────────────────────────────────────────────────────────────────────────────────╮
│                                                                                                           │
│  opencode => ok (long-form probe succeeded)                                                               │
│    model: glm-5-free                                                                                      │
│    response: Hello! I'm GLM, trained by Z.ai. How can I assist you today? Whether you have questions, need│
│  information, or just want to chat, I'm here to help. What would you like to discuss?                     │
│                                                                                                           │
├───────────────────────────────────────────────────────────────────────────────────────────────────────────╯
│
│ Onboarding Summary  ──────────────────────────────────────────────────────────╮
│                                                                               │
│  Runtime: Real OS workstation                                                 │
│  Components: desktop_app, tui, ide_extension, browser_extension, local_website│
│  Auth: skipped (dev@localhost)                                                │
│  Providers selected: 1                                                        │
│  Provider config: saved                                                       │
│  Provider config path: C:\Users\Computer\.dx\config\providers.toml            │
│  Channels selected: 0                                                         │
│  Storage: response.json + Turso (when configured)                             │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────╯
│
✓ Saved to response.json
│
✓ DX onboarding completed
│