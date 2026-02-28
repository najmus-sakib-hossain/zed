<p align="center">
  <h1 align="center">DX 🚀</h1>
</p>

<p align="center">
  <strong>Enhance the development experience for <em>everyone</em>. 100% Rust. Offline-first. All platforms.</strong><br>
  ⚡️ <strong>100+ LLM providers · saves 30–90% tokens · saves up to 70% RAM · runs on any OS</strong>
</p>

<p align="center">
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="NOTICE"><img src="https://img.shields.io/github/contributors/zeroclaw-labs/zeroclaw?color=green" alt="Contributors" /></a>
  <a href="https://buymeacoffee.com/argenistherose"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Donate-yellow.svg?style=flat&logo=buy-me-a-coffee" alt="Buy Me a Coffee" /></a>
  <a href="https://x.com/zeroclawlabs?s=21"><img src="https://img.shields.io/badge/X-%40dx-000000?style=flat&logo=x&logoColor=white" alt="X: @dx" /></a>
  <a href="https://t.me/zeroclawlabs"><img src="https://img.shields.io/badge/Telegram-%40dx-26A5E4?style=flat&logo=telegram&logoColor=white" alt="Telegram: @dx" /></a>
</p>

<p align="center">
  🌐 <strong>Languages:</strong> <a href="README.md">English</a> · <a href="docs/i18n/zh-CN/README.md">简体中文</a> · <a href="docs/i18n/ja/README.md">日本語</a> · <a href="docs/i18n/ru/README.md">Русский</a> · <a href="docs/i18n/fr/README.md">Français</a> · <a href="docs/i18n/vi/README.md">Tiếng Việt</a> · <a href="docs/i18n/el/README.md">Ελληνικά</a>
</p>

<p align="center">
  <a href="#quick-start">Getting Started</a> |
  <a href="bootstrap.sh">One-Click Setup</a> |
  <a href="docs/README.md">Docs Hub</a> |
  <a href="docs/SUMMARY.md">Docs TOC</a>
</p>

<p align="center">
  <strong>Quick Routes:</strong>
  <a href="docs/reference/README.md">Reference</a> ·
  <a href="docs/operations/README.md">Operations</a> ·
  <a href="docs/troubleshooting.md">Troubleshoot</a> ·
  <a href="docs/security/README.md">Security</a> ·
  <a href="docs/hardware/README.md">Hardware</a> ·
  <a href="docs/contributing/README.md">Contribute</a>
</p>

<p align="center">
  <strong>DX is the universal AI runtime for every developer, on every device.</strong><br />
  Ask · Agent · Plan · Search · Study · Research — all in one.
</p>

<p align="center">
  DX is the <strong>development experience platform</strong> for agentic workflows — a single runtime that runs natively on macOS, Windows, Linux, Android, iOS, ChromeOS, watchOS, tvOS, and more. Deploy anywhere. Swap anything.
</p>

<p align="center"><code>100+ LLM providers · 400+ connectors · native on all OS · offline-first · trait-driven · secure-by-default</code></p>

### 📢 Announcements

Use this board for important notices (breaking changes, security advisories, maintenance windows, and release blockers).

| Date (UTC) | Level       | Notice                                                                                                                                                                                                                                                                                                                                                 | Action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-02-19 | _Important_ | DX is the official rebrand of the project formerly known as ZeroClaw. All functionality is preserved; the binary is now `dx` and config lives in `~/.dx/`. | Update your scripts: replace `zeroclaw` → `dx` and `~/.zeroclaw/` → `~/.dx/`. |
| 2026-02-21 | _Important_ | DX project is live. Use [this repository](https://github.com/zeroclaw-labs/zeroclaw) as the single source of truth until the repository is renamed. | Follow official social accounts for updates. |

### ✨ Features

- 🚀 **All Operating Systems:** Native apps for macOS, Windows, Linux, Android, iOS, ChromeOS, watchOS, tvOS — and a web dashboard to manage them all from the browser.
- 🧠 **100+ LLM Providers:** More providers than any other tool — all supported, all swappable. Runs offline too, using RLM and the DX serializer.
- 💾 **30–90% Token Savings:** DX serializer, image tokenizer, and context compression cut token usage dramatically without losing meaning.
- 🦀 **Up to 70% Less RAM:** 100% Rust runtime does more with less — sub-5MB memory footprint at idle.
- 🔌 **400+ Connectors:** Link any skill from cloud CLI, any plugin, WhatsApp, Telegram, Discord, and all major social and communication apps.
- 🎨 **Multi-Modal Generation:** Text, image, video, 3D/AR/VR, documents (PDF, DOCX) — generate everything in one place.
- 📚 **6 Modes:** Ask · Agent · Plan · Search · Study · Research — each tuned for a specific workflow.
- 🔧 **Extensions for Everything:** Chrome, Safari, Firefox, Edge · VS Code, JetBrains · Photoshop, Figma, DaVinci Resolve — and all the popular ones.
- 🛡️ **Traffic Security (Green/Yellow/Red):** Agent auto-acts safely, warns on sensitive tasks, and always keeps a backup before destructive operations.
- 📦 **Built-in Tools:** Forge (media VCS), Check (security rank), Workspace, Serializer, i18n, Driven, DCP.

### Why developers pick DX

- **Works everywhere:** single binary for every OS — ARM, x86, RISC-V, mobile, embedded, browser.
- **Offline-first:** RLM + DX serializer means full capability without an internet connection.
- **Lean by default:** small Rust binary, fast startup, under 5MB memory on release builds.
- **Secure by design:** traffic-coloured agent (green/yellow/red), pairing, sandboxing, explicit allowlists.
- **Fully swappable:** providers, channels, tools, memory, tunnels are all traits — swap anything.
- **No lock-in:** 100+ OpenAI-compatible and native provider endpoints.

## Benchmark Snapshot (DX vs alternatives, Reproducible)

Local machine quick benchmark (macOS arm64, Feb 2026) normalized for 0.8GHz edge hardware.

|                           | OpenClaw      | NanoBot        | PicoClaw        | DX 🚀                |
| ------------------------- | ------------- | -------------- | --------------- | -------------------- |
| **Language**              | TypeScript    | Python         | Go              | **Rust**             |
| **RAM**                   | > 1GB         | > 100MB        | < 10MB          | **< 5MB**            |
| **Startup (0.8GHz core)** | > 500s        | > 30s          | < 1s            | **< 10ms**           |
| **Binary Size**           | ~28MB (dist)  | N/A (Scripts)  | ~8MB            | **~8.8 MB**          |
| **Cost**                  | Mac Mini $599 | Linux SBC ~$50 | Linux Board $10 | **Any hardware**     |

> Notes: DX results are measured on release builds using `/usr/bin/time -l`. OpenClaw requires Node.js runtime (~390MB additional memory overhead). DX is a static binary. The RAM figures are runtime memory; build-time compilation requirements are higher.

<p align="center">
  <img src="zero-claw.jpeg" alt="ZeroClaw vs OpenClaw Comparison" width="800" />
</p>

### 🙏 Special Thanks

A heartfelt thank you to the communities and institutions that inspire and fuel this open-source work:

- **The open-source community** — for the libraries, ideas, and patches that make DX possible.
- **Every early contributor** — your issues, PRs, and feedback shaped what DX is today.
- **The World & Beyond** 🌍✨ — to every developer, dreamer, and builder out there. DX is for you.

We're building in the open because the best ideas come from everywhere. If you're reading this, you're part of it. Welcome. 🚀❤️

## ⚠️ Official Repository & Impersonation Warning

**This is the only official DX repository:**

> https://github.com/zeroclaw-labs/zeroclaw

Any other repository, organization, domain, or package claiming to be "DX" or implying affiliation with DX Labs is **unauthorized and not affiliated with this project**. Known unauthorized forks will be listed in [TRADEMARK.md](TRADEMARK.md).

If you encounter impersonation or trademark misuse, please [open an issue](https://github.com/zeroclaw-labs/zeroclaw/issues).

---

## License

DX is dual-licensed for maximum openness and contributor protection:

| License | Use case |
|---|---|
| [MIT](LICENSE-MIT) | Open-source, research, academic, personal use |
| [Apache 2.0](LICENSE-APACHE) | Patent protection, institutional, commercial deployment |

You may choose either license. **Contributors automatically grant rights under both** — see [CLA.md](CLA.md) for the full contributor agreement.

### Trademark

The **DX** name and logo are trademarks of DX Labs. This license does not grant permission to use them to imply endorsement or affiliation. See [TRADEMARK.md](TRADEMARK.md) for permitted and prohibited uses.

### Contributor Protections

- You **retain copyright** of your contributions
- **Patent grant** (Apache 2.0) shields you from patent claims by other contributors
- Your contributions are **permanently attributed** in commit history and [NOTICE](NOTICE)
- No trademark rights are transferred by contributing

## Contributing

New to DX? Look for issues labeled [`good first issue`](https://github.com/zeroclaw-labs/zeroclaw/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) — see our [Contributing Guide](CONTRIBUTING.md#first-time-contributors) for how to get started.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CLA.md](CLA.md). Implement a trait, submit a PR:

- CI workflow guide: [docs/ci-map.md](docs/ci-map.md)
- New `Provider` → `src/providers/`
- New `Channel` → `src/channels/`
- New `Observer` → `src/observability/`
- New `Tool` → `src/tools/`
- New `Memory` → `src/memory/`
- New `Tunnel` → `src/tunnel/`
- New `Skill` → `~/.dx/workspace/skills/<name>/`

---

**DX** — Enhance the development experience for everyone. Deploy anywhere. Swap anything. 🚀

## Star History

<p align="center">
  <a href="https://www.star-history.com/#zeroclaw-labs/zeroclaw&type=date&legend=top-left">
    <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=zeroclaw-labs/zeroclaw&type=date&theme=dark&legend=top-left" />
     <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=zeroclaw-labs/zeroclaw&type=date&legend=top-left" />
     <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=zeroclaw-labs/zeroclaw&type=date&legend=top-left" />
    </picture>
  </a>
</p>
