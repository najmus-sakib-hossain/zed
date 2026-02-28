# DX Website Master Plan (Execution Roadmap)

## Objective
Ship a consistent DX-first website experience across all major routes in `midday/apps/website`, using the current design system and animation stack, while migrating legacy Midday-era copy and SEO safely.

## Principles
- Keep existing UI primitives and Tailwind tokens.
- Prefer surgical copy/component updates over broad rewrites.
- Prioritize top-traffic routes first.
- Validate with `npm`-based checks (not bun).

## Phase 1 — Core Brand Alignment (In Progress)
### Scope
- Home + shared DX carousel routes
- Header/footer consistency
- Key metadata alignment for primary routes

### Tasks
1. Ensure DX positioning is clear on hero and section narrative.
2. Keep launch date, value proposition, and platform coverage consistent.
3. Keep all CTA language aligned (trial/waitlist/docs).

### Exit Criteria
- Main landing flow is visibly DX-branded.
- Assistant/docs/pricing/integrations routes reflect DX language.

## Phase 2 — High-Impact Route Conversion (In Progress)
### Scope
- `about`, `story`, `support`, `testimonials`, `compare/*`

### Tasks
1. Replace legacy Midday references in route metadata.
2. Convert visible headings/body copy to DX.
3. Re-align compare route copy and CTAs to DX.
4. Keep content structure/UX unchanged.

### Exit Criteria
- No obvious Midday wording on the above pages.
- Compare experience reads as DX-first.

## Phase 3 — Docs & Content System Migration (Planned)
### Scope
- Docs landing + docs layout + AI assist prompts
- Selected docs pages with highest traffic

### Tasks
1. Convert docs shell text from Midday context to DX context.
2. Update docs helper prompts (“Ask AI” text) to DX wording.
3. Create migration list for markdown docs (`src/app/docs/content/*.mdx`).

### Exit Criteria
- Docs chrome is DX-branded.
- Priority doc pages no longer mention Midday branding.

## Phase 4 — Long-Tail Route Migration (Planned)
### Scope
- Remaining marketing/product routes and metadata
- Updates/blog index and templates (selective)

### Tasks
1. Route-by-route metadata cleanup.
2. CTA destination review (remove stale external Midday links where needed).
3. Defer legal/policy entity changes to explicit legal review.

### Exit Criteria
- Product/marketing pages are internally consistent for DX.
- Known legal/compliance text remains intentionally unchanged until approved.

## Phase 5 — UI/Content Quality Pass (Planned)
### Scope
- Spelling, grammar, consistency, SEO copy quality
- Animation behavior checks

### Tasks
1. Normalize terminology (`DX`, `RLM`, `DX Serializer`, `DCP`).
2. Ensure no broken anchors/links from updated CTAs.
3. Verify responsive readability and section hierarchy.

### Exit Criteria
- Clean editorial quality baseline.
- No obvious broken copy/UI issues on key breakpoints.

## Phase 6 — Validation & Release Readiness (Planned)
### Scope
- Build/lint/type checks via npm
- Manual smoke checklist

### Tasks
1. Run `npm` workflow for lint/build in the website app.
2. Fix only issues introduced by migration changes.
3. Prepare release notes of converted routes and deferred items.

### Exit Criteria
- Validation commands pass in target environment.
- Ready-to-ship summary with known deferred scope.

## Deferred / Needs Decision
1. Legal text (`terms`, `policy`) includes company/entity references and should be updated only with legal approval.
2. Historical update posts (`src/app/updates/posts/*.mdx`) are legacy records; decide whether to preserve historical branding or rewrite.
3. External domains (`app.midday.ai`, `api.midday.ai`) should be switched only after DX production endpoints are confirmed.

## Immediate Next Sprint (Execution Order)
1. Finish metadata/copy cleanup for remaining high-value routes.
2. Convert docs shell and compare residual Midday strings.
3. Run npm-based validation and address introduced issues.
