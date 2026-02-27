Please list all of the APIs that can generate Pdf, Docs, Charts related stuffs by creating an account in their platform and getting an API key to programmatically create videos. As for DX Rust software we have to be able to create Pdf, Docs, Charts related stuffs but as we don't have any money we will rely on other third-party providers so that the users can log in to those platforms and get an API key and generate Pdf, Docs, Charts related stuffs in our DX Rust software. And if you can then also list awesome GitHub repos listing all of the free video providers that give free credit to users so that the users can use those free credits to create Pdf, Docs, Charts related stuffs on our DX Rust platform.

**Here is a complete, up-to-date (Feb 2026) list of APIs** for generating **PDFs, Documents (DOCX, XLSX, PPTX, etc.), and Charts/Graphs** (from templates, HTML, JSON data, Markdown, or direct Chart.js config) via **BYOK** (Bring Your Own Key).

Users create a free account on the provider, generate their personal API key, and paste it into your DX Rust software. Your Rust backend (`reqwest` + `serde_json` + `tokio`) makes the call — the provider bills **only the user**. Zero cost to you. Outputs are downloadable PDFs, DOCX files, or chart images/PDFs (PNG/SVG/PDF).

All are standard REST JSON APIs — trivial in Rust. Many support embedding charts inside PDFs/docs.

### 1. Core PDF / Document / Chart Generation APIs
All support direct signup + personal API key (no sales call needed).

| Provider | Key Capabilities | Free Tier / Credits on Signup | API Key Generation | Rust Integration Notes |
|----------|------------------|-------------------------------|--------------------|------------------------|
| **Carbone.io** (Best multi-format) | JSON + template → PDF/DOCX/XLSX/PPTX/ODT (built-in charts, tables, loops, images) | **100 docs/month free** (production token) + unlimited watermarked sandbox | carbone.io → Dashboard → API Tokens | `POST /render` with `templateId` + `data` JSON; returns PDF/DOCX buffer |
| **CraftMyPDF** | Drag-and-drop templates + HTML components + embedded charts/QR/barcodes | **50 PDFs/images per month free** (recurring) | craftmypdf.com → Account → API | `POST /api/v1/template/{id}/output` + JSON payload; async/webhook support |
| **APITemplate.io** | WYSIWYG editor or HTML/Markdown → PDF + images; charts via HTML | **50 PDFs/images per month free** | apitemplate.io → Dashboard → API Key | Single endpoint for PDF/image; perfect for invoices/reports with charts |
| **PDFShift** | HTML/URL → high-fidelity PDF (full CSS/JS, headers/footers, charts) | **50 PDFs/month free** (permanent) | pdfshift.io → Dashboard | `POST /convert/pdf` with `source` (HTML or URL) |
| **DocRaptor** | PrinceXML engine: pixel-perfect HTML/CSS → PDF (best typography, complex charts/tables) | Unlimited test (watermarked) + limited free tier | docraptor.com → Account → API Key | `POST /docs` with `document_content` HTML |
| **PDF Generator API** | Visual templates + JSON merge → PDF; batch + conditional logic | 14-day trial + sandbox (2,500 free merges) | pdfgeneratorapi.com → Sign up → API token | `POST /documents/generate` with template + data |
| **Adobe PDF Services API** | HTML/Word/JSON → PDF; extract, OCR, combine, accessibility | **500 free transactions/month** | developer.adobe.com → Console → Credentials (API key + secret) | JWT/OAuth but simple for generation; Document Generation SDK |
| **QuickChart.io** (Best dedicated Charts) | Chart.js config → PNG/SVG/PDF chart images (bar, line, pie, radar, etc.) | Free community (high limits: ~100k/month, rate-limited); optional paid key for more | quickchart.io → Sign up (optional for key/higher limits) | GET/POST `https://quickchart.io/chart?c={chart.js json}` or `/chart/create` |
| **PDF.co** | All-in-one: HTML→PDF, merge, edit, + chart generation from data | Limited free credits on signup (~10k operations) | pdf.co → Dashboard → API Key | 100+ endpoints (e.g., `/pdf/convert/from/html`, `/pdf/add/chart`) |
| **iLovePDF** | Convert/merge/split/compress PDFs; HTML→PDF; basic chart embedding | **250 docs/month free** | developer.ilovepdf.com → API Key | Simple REST for processing + generation |

### 2. How Users Generate PDFs/Docs/Charts in Your DX Rust Software
1. User signs up free on e.g. carbone.io / craftmypdf.com / quickchart.io.
2. Copies API key from dashboard.
3. Pastes into your Rust app (store encrypted per-user).
4. Your UI: dropdown (Provider + Type: PDF from HTML / DOCX from template / Chart from data) + editor or JSON upload.
5. Rust backend examples (works for 95% of them):

```rust
// PDF from HTML/template (e.g. PDFShift, Carbone, CraftMyPDF)
let client = reqwest::Client::new();
let res = client.post("https://api.carbone.io/render")
    .header("Authorization", format!("Bearer {}", user_key))
    .json(&serde_json::json!({
        "templateId": "your-template-id",
        "data": { "name": "John", "chartData": [...] }  // supports charts
    }))
    .send().await?
    .bytes().await?;  // save as .pdf or .docx

// Chart image (QuickChart - even simpler, no key needed for basic)
let chart_config = r#"{ "type": "bar", "data": { ... } }"#;
let url = format!("https://quickchart.io/chart?c={}", urlencoding::encode(chart_config));
let chart_bytes = client.get(&url).send().await?.bytes().await?;
```

Most return direct file bytes or signed download URL. Use webhooks/async polling for large docs.

**Charts inside PDFs/Docs**: Carbone, CraftMyPDF, APITemplate, and DocRaptor all support embedding QuickChart-style charts via HTML/JSON.

### Awesome GitHub Repos Listing Free-Credit PDF / Doc / Chart Providers
These actively maintained lists highlight **free tiers / signup credits / API keys** (ideal “Discover Free Credits” tab in your app):

- **https://github.com/public-apis/public-apis** → Official “Documents & Productivity” + “Data Visualization” sections (Carbone, CraftMyPDF, APITemplate, QuickChart, Image-Charts, PDF.co, iLovePDF, etc. with free-tier tags).
- **https://github.com/OneOffTech/awesome-pdf** → Curated PDF libraries, services & APIs (includes generation tools with free tiers).
- **https://github.com/cporter202/API-mega-list** → 10k+ APIs with dedicated PDF/document sections (free-credit notes).
- **https://github.com/Kikobeats/awesome-api** → REST API resources including document/chart generators.

**Pro tip for your Rust platform**: Default to **Carbone.io** (multi-format + charts) or **QuickChart** (charts) + **CraftMyPDF/APITemplate** (visual templates). Add a tab “Free PDF/Doc/Chart Credits” linking these GitHub repos — users can rotate keys across Carbone (100 free), Adobe (500), PDFShift (50), QuickChart (unlimited basic), and iLovePDF (250) for maximum free usage.

This covers **every major production-ready PDF/Document/Chart generation API** as of Feb 2026. The space is stable but new template editors appear regularly, so star the GitHub lists.

Need ready-to-copy Rust code for Carbone multi-format, QuickChart with custom fonts, or a full multi-provider wrapper? Or integration with LLM prompt → chart data? Just say the provider!
