# Contributing Guide

Thank you for your interest in x-hub. The project currently maintains only the latest version and does not maintain legacy branches.

## Development Environment

- Node.js 20
- Rust stable (including Tauri 2 dependencies)
- Windows (CI validation runs on `windows-latest`)

## Local Validation

```bash
npm ci
npm run build        # vue-tsc type-check + Vite build
cargo check --manifest-path src-tauri/Cargo.toml
```

This should match the CI validation in `.github/workflows/ci.yml` (`check` job). Pull requests must pass CI.

## Submit a PR

1. Fork the repo and create a short-lived branch from `master`; delete it after merge.
2. Target the `master` branch for the PR and include the motivation, changed behavior, and validation details in the description.
3. For substantial changes, open an issue for discussion before implementing.
4. Use squash merge to keep the history linear.

## Support Policy

Only the latest version is maintained; feature requests and bug reports should be based on the latest release.

## License

MIT (see `LICENSE`).
