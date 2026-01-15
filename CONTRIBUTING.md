# Contributing to feedparser-rs

Thank you for your interest in contributing to feedparser-rs! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.88.0 or later (edition 2024)
- [cargo-make](https://github.com/sagiegurari/cargo-make) for task automation
- Node.js 18+ (for Node.js bindings development)
- Python 3.10+ (for Python bindings development)

### Setup

1. Fork and clone the repository:

   ```bash
   git clone https://github.com/YOUR_USERNAME/feedparser-rs.git
   cd feedparser-rs
   ```

2. Install cargo-make:

   ```bash
   cargo install cargo-make
   ```

3. Verify the setup:

   ```bash
   cargo make ci-all
   ```

## Development Workflow

### Branch Naming

Use descriptive branch names:

- `feat/feature-name` — New features
- `fix/issue-description` — Bug fixes
- `docs/what-changed` — Documentation updates
- `refactor/what-changed` — Code refactoring
- `test/what-tested` — Test additions

### Making Changes

1. Create a new branch from `main`:

   ```bash
   git checkout -b feat/your-feature
   ```

2. Make your changes, following the [code style guidelines](#code-style).

3. Run checks before committing:

   ```bash
   cargo make pre-commit
   ```

4. Commit your changes with a clear message:

   ```bash
   git commit -m "feat: add support for XYZ"
   ```

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation only
- `style:` — Formatting, no code change
- `refactor:` — Code change that neither fixes a bug nor adds a feature
- `test:` — Adding or updating tests
- `chore:` — Maintenance tasks

Examples:

```
feat: add JSON Feed 1.1 support
fix: handle malformed RSS dates correctly
docs: update installation instructions
test: add tests for Atom parsing edge cases
```

## Code Style

### Rust

- Run `cargo make fmt` before committing
- All code must pass `cargo make clippy` without warnings
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `thiserror` for error types
- Prefer `&str` over `String` in function parameters where possible
- Document all public APIs with doc comments

### Documentation

- All public items must have documentation
- Include examples in doc comments where helpful
- Keep documentation in English

### Testing

- Write tests for new functionality
- Ensure all tests pass: `cargo make test`
- Add test fixtures to `tests/fixtures/` for new feed formats

## Pull Request Process

1. **Update documentation** — If your change affects the API, update relevant docs.

2. **Add tests** — All new features and bug fixes should have tests.

3. **Run all checks**:

   ```bash
   cargo make pre-push
   ```

4. **Create the pull request** with:
   - Clear title following commit message format
   - Description of what changed and why
   - Reference to related issues (e.g., "Fixes #123")

5. **Address review feedback** — Respond to comments and make requested changes.

### PR Checklist

- [ ] Code follows the project style guidelines
- [ ] All tests pass (`cargo make test`)
- [ ] Linting passes (`cargo make lint`)
- [ ] Documentation updated if needed
- [ ] CHANGELOG.md updated for notable changes

## Testing

### Running Tests

```bash
# All tests
cargo make test

# Rust tests only
cargo make test-rust

# With coverage
cargo make coverage
```

### Test Fixtures

Test feeds are located in `tests/fixtures/`. When adding support for new feed quirks:

1. Add a minimal test fixture demonstrating the issue
2. Add a test that uses the fixture
3. Implement the fix
4. Verify the test passes

## Reporting Issues

### Bug Reports

Include:

- feedparser-rs version
- Rust/Python/Node.js version
- Minimal reproduction case
- Expected vs actual behavior
- Sample feed (if applicable, sanitized of sensitive data)

### Feature Requests

- Check existing issues first
- Describe the use case
- Explain why existing features don't solve the problem

## Architecture Overview

```
feedparser-rs/
├── crates/
│   ├── feedparser-rs-core/    # Core Rust parser
│   ├── feedparser-rs-node/    # Node.js bindings (napi-rs)
│   └── feedparser-rs-py/      # Python bindings (PyO3)
├── tests/
│   └── fixtures/              # Test feed files
└── benchmarks/                # Performance benchmarks
```

### Core Crate Structure

- `src/lib.rs` — Public API
- `src/parser/` — Format-specific parsers (RSS, Atom, JSON Feed)
- `src/feed.rs` — Data structures
- `src/date.rs` — Date parsing
- `src/sanitize.rs` — HTML sanitization
- `src/encoding.rs` — Character encoding detection

## Release Process

Releases are automated via GitHub Actions. Maintainers tag releases following [Semantic Versioning](https://semver.org/):

- MAJOR: Breaking API changes
- MINOR: New features, MSRV increases
- PATCH: Bug fixes, documentation

## Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for questions

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
