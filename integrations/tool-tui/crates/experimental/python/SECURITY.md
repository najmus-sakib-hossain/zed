
# Security Policy

## Supported Versions

Version: 0.1.x, Supported: ✅

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly: -Do not open a public issue -Email security concerns to the maintainers -Include detailed steps to reproduce -Allow reasonable time for a fix before disclosure

## Security Considerations

### Package Manager

- Hash verification: All packages verified with SHA256/BLAKE3
- TLS only: HTTPS required for all registry connections
- No arbitrary code execution: Lock files are data-only
- Sandboxed builds: Build scripts run in isolated environments

### Runtime

- Memory safety: Written in Rust with safe abstractions
- No eval of untrusted code: User code runs in controlled environment
- Resource limits: Configurable memory and CPU limits

### Test Runner

- Isolated execution: Tests run in separate processes
- No network by default: Network access must be explicitly enabled
- Temporary directories: Test artifacts cleaned up automatically

## Best Practices

### For Users

- Pin dependencies to specific versions
- Use lock files in production
- Verify package hashes
- Keep DX-Py updated

### For Contributors

- No unsafe code without justification
- All inputs validated
- No secrets in code or logs
- Dependencies audited regularly

## Audit Status

- cargo-audit: Clean
- cargo-deny: Configured
- RUSTSEC: No known vulnerabilities

## Contact

For security concerns, contact the maintainers directly.
