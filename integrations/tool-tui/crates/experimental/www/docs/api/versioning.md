
# API Versioning Strategy

This document describes the versioning strategy for dx-www public APIs.

## Semantic Versioning

dx-www follows Semantic Versioning 2.0.0: -MAJOR version for incompatible API changes -MINOR version for backwards-compatible functionality additions -PATCH version for backwards-compatible bug fixes

## Version Format

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]
```
Examples: -`1.0.0` - First stable release -`1.1.0` - New features, backwards compatible -`1.1.1` - Bug fixes only -`2.0.0-alpha.1` - Pre-release for next major version

## Stability Guarantees

### Stable APIs (v1.0.0+)

Once a crate reaches v1.0.0, the following guarantees apply: -Public Types: Struct fields, enum variants, and trait methods will not be removed or renamed -Function Signatures: Parameter types and return types will not change -Behavior: Documented behavior will not change in breaking ways -Error Types: Error variants will not be removed (new variants may be added)

### Pre-1.0 APIs (v0.x.y)

Before v1.0.0: -Minor version bumps may include breaking changes -Patch versions are backwards compatible -APIs are subject to change based on feedback

## Breaking Change Policy

### What Constitutes a Breaking Change

- Removing or renaming public items (types, functions, modules)
- Changing function signatures (parameters, return types)
- Changing struct field types or visibility
- Removing enum variants
- Changing trait requirements
- Changing default behavior in incompatible ways

### What Is NOT a Breaking Change

- Adding new public items
- Adding new optional parameters with defaults
- Adding new enum variants (if `#[non_exhaustive]`)
- Adding new trait methods with default implementations
- Performance improvements
- Bug fixes that correct documented behavior

## Deprecation Process

- Announce: Deprecation is announced in CHANGELOG.md
- Mark: Item is marked with `#[deprecated(since = "X.Y.Z", note = "...")]`
- Document: Migration guide is provided in deprecation note
- Grace Period: Deprecated items remain for at least one minor version
- Remove: Item is removed in next major version Example:
```rust


#[deprecated(


since = "1.2.0", note = "Use `new_function()` instead. See migration guide at docs/migration/v1-to-v2.md"
)]
pub fn old_function() { ... }
```

## MSRV Policy

- Minimum Supported Rust Version (MSRV): 1.85
- MSRV bumps are considered breaking changes and require a major version bump
- MSRV is tested in CI on every commit

## Crate Versioning

All workspace crates share the same version number for simplicity:
```toml
[workspace.package]
version = "1.0.0"
```
This ensures compatibility between crates and simplifies dependency management.

## Release Process

- Feature Freeze: No new features after freeze date
- Release Candidate: `X.Y.Z-rc.1` published for testing
- Documentation: All public APIs documented
- Changelog: CHANGELOG.md updated with all changes
- Tag: Git tag created for release
- Publish: Crates published to crates.io

## API Stability Tiers

### Tier 1: Stable

- Fully documented
- Covered by tests
- Breaking changes require major version bump

### Tier 2: Unstable

- Marked with `#[doc(hidden)]` or in `unstable` module
- May change without notice
- Not recommended for production use

### Tier 3: Internal

- Private to the crate
- No stability guarantees
- May be removed at any time

## See Also

- Migration Guide: v0.x to v1.x (../migration/v0-to-v1.md)
- Changelog (../../CHANGELOG.md)
- Contributing Guide (../../CONTRIBUTING.md)
