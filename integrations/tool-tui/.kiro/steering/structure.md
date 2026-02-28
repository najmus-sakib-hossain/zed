
# Dx Project Structure

## Root Layout

@tree:dx[]

## Crate Organization (`crates/`)

### DX Tools (standalone utilities)

- `serializer/`
- LLM-optimized serialization format
- `markdown/`
- Context compiler for LLMs
- `style/`
- Binary CSS engine (B-CSS)
- `check/`
- Multi-language linter
- `forge/`
- Build orchestration
- `media/`
- Universal media acquisition
- `icon/`
- SVG icon system
- `font/`
- Binary font subsetting
- `i18n/`
- Internationalization
- `generator/`
- Template code generator
- `workspace/`
- Dev environment configurator
- `driven/`
- AI-assisted development orchestrator
- `security/`
- Security utilities

### DX CLI

- `dx/`
- Main CLI entry point
- `cli/`
- CLI implementation
- `dcp/`
- DX Control Protocol

### DX WWW (Web Framework - `www/`)

Core Runtime: `framework-core/`, `dom/`, `morph/`, `sched/` Binary Protocol: `binary/`, `packet/` Client Runtimes: `client/` (7.5KB), `client-tiny/` (338B) Server: `server/`, `cache/` Data Layer: `form/`, `query/`, `db/`, `state/` Auth & Security: `auth/`, `guard/` Network: `sync/`, `offline/` Accessibility: `a11y/`, `rtl/`, `print/`, `fallback/` Binary Dawn I/O: `reactor/`, `db-teleport/`

### Language Stacks (`javascript/`, `python/`)

- `runtime/`
- Language runtime
- `bundler/`
- Code bundler
- `test-runner/`
- Test execution
- `package-manager/`
- Package management
- `compatibility/`
- API compatibility layers

## Naming Conventions

- Crate names: `dx-{name}` (e.g., `dx-serializer`, `dx-style`)
- WWW crates: `dx-www-{name}` (e.g., `dx-www-dom`, `dx-www-server`)
- Lib names: Short form without prefix (e.g., `serializer`, `style`)

## Excluded Workspaces

Some crates have their own internal workspaces and are excluded from the root: -`crates/javascript/bundler` - 10 internal crates -`crates/javascript/runtime` - Own workspace -`crates/javascript/package-manager` - 12 internal crates -`crates/javascript/compatibility` - 12 sub-crates -`crates/dcp` - Own workspace -`playground/` - Experiments
