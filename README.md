# Haleacas

> **An offline-first workspace for understanding the repositories around you.**

[![License: MIT](https://img.shields.io/badge/License-MIT-8b9cf6.svg)](LICENSE)
[![Status: Archived](https://img.shields.io/badge/status-archived-64748b.svg)](#project-status)
[![Built with Rust](https://img.shields.io/badge/backend-Rust-CE422B.svg)](https://www.rust-lang.org/)
[![Built with Tauri](https://img.shields.io/badge/desktop-Tauri%202-24C8D8.svg)](https://tauri.app/)

Haleacas is a desktop application concept and implementation for keeping local software repositories understandable and manageable. It brings repository discovery, health checks, Git context, dependency information, GitHub activity, and selected AI-assisted workflows into one place—without trying to replace Git, GitHub, or your editor.

> **A note on the project’s status**
>
> Haleacas has been merged into a larger project and is no longer receiving updates from its original author. This repository remains public as a historical snapshot for people who used it, are curious about its implementation, or may find parts of the code useful. It is provided as-is: there is no promise of future releases, fixes, compatibility updates, or support.

---

## Contents

- [What it is](#what-it-is)
- [What is in the codebase](#what-is-in-the-codebase)
- [Architecture and technology](#architecture-and-technology)
- [Getting started](#getting-started)
- [Development commands](#development-commands)
- [Privacy and data flow](#privacy-and-data-flow)
- [Project status and support](#project-status-and-support)
- [Known limitations](#known-limitations)
- [License](#license)

## What it is

Haleacas was designed around a simple idea: developers often work across many repositories, and each repository carries its own state, history, dependencies, conventions, and maintenance needs. Finding that context shouldn't require opening every tool separately.

The application brings several repository-oriented workflows together in a local desktop interface.

The codebase includes functionality and backend command surfaces for:

- **Repository discovery and scanning** — locate repositories under configured roots and inspect their local state.
- **Repository health** — collect findings intended to help identify maintenance issues.
- **Git context** — work with repository history, timelines, and commit-level statistics.
- **Dependency inspection** — generate dependency reports from supported project manifests.
- **Duplicate and search workflows** — inspect duplicate-related findings and search across discovered repositories.
- **README assistance** — inspect, analyze, preview, and save README content.
- **GitHub integration** — retrieve repository information, releases, inbox items, and Actions data when requested.
- **GitHub Pages workflows** — inspect Pages-related information and create a Pages index.
- **Optional AI assistance** — send an explicit question with selected repository context to a configured local AI-compatible provider.
- **Desktop conveniences** — system-tray navigation, launch-at-login behavior, and configurable close behavior.

These are descriptions of functionality represented in the source code, not a claim that every workflow is complete, production-ready, or equally tested. The repository should be treated as an early implementation.

## What is in the codebase

| Path | Purpose |
| --- | --- |
| `src/` | React + TypeScript frontend |
| `src-tauri/` | Tauri desktop shell and Rust backend |
| `src-tauri/crates/haleacas-core/` | Framework-independent repository, Git, and persistence logic |
| `public/` | Static frontend assets |
| `.github/workflows/` | Build automation |
| `package.json` / `package-lock.json` | Frontend dependencies and scripts |
| `Cargo.toml` files | Rust workspace and crate dependencies |
| `LICENSE` | MIT license |

The `haleacas-core` crate is deliberately separated from the Tauri shell. Its repository-scanning, Git, and persistence logic can be compiled and tested without starting the desktop GUI.

## Architecture and technology

**Frontend**
- React
- TypeScript
- Vite
- Tailwind CSS
- React Router

**Desktop application**
- Tauri 2
- Rust
- SQLite through `rusqlite`
- `git2` for Git integration
- `walkdir` for filesystem traversal

**Other integrations**
- GitHub API access
- Optional local AI-compatible endpoints
- OS-native credential storage for supported platforms

The frontend is built by Vite. Tauri packages the generated frontend together with the Rust application.

## Getting started

### Prerequisites

- Node.js and npm
- Rust toolchain and Cargo
- The platform-specific prerequisites required by [Tauri 2](https://v2.tauri.app/start/prerequisites/)

On Linux, the original development setup used WebKitGTK, GTK, AppIndicator, librsvg, libsoup, `pkg-config`, and a C/C++ build toolchain. Exact package names vary by distribution; consult the current Tauri prerequisites for your system.

On Windows, install the Microsoft C++ build tools and WebView2 prerequisites described in the Tauri documentation.

### Install dependencies

From the repository root:

```bash
npm install
```

### Run the desktop application in development mode

```bash
npx tauri dev
```

### Build a production bundle

```bash
npx tauri build
```

Packaged installers are written under:

```text
src-tauri/target/release/bundle/
```

The available bundle formats depend on the operating system and its packaging prerequisites.

## Development commands

The npm scripts currently defined in `package.json` are:

| Command | Description |
| --- | --- |
| `npm run dev` | Start the Vite development server |
| `npm run build` | Type-check and build the frontend |
| `npm run lint` | Run Oxlint |
| `npm run preview` | Preview the built frontend |

Rust core tests:

```bash
cd src-tauri
cargo test -p haleacas-core
```

The repository also contains ignored tests that may contact a real GitHub API or a local AI provider. They are not part of the default test run:

```bash
cd src-tauri
cargo test -p haleacas-core -- --ignored
```

Check the test source and configure the required services before running ignored tests.

Check the Tauri application crate:

```bash
cd src-tauri
cargo check -p app
```

Build the frontend independently:

```bash
npm run build
```

> These commands describe the checked-in project configuration. Because this repository is archived, newer operating systems, toolchains, or dependency releases may require adjustments.

## Privacy and data flow

Haleacas was designed with a local-first approach.

- Repository discovery, local Git inspection, and the SQLite cache are intended to remain on the user's machine.
- GitHub access is optional and is used for GitHub-related workflows. Public repository information can be fetched without authentication; connecting a personal access token enables authenticated access.
- The original project documentation states that a connected GitHub token is stored using the operating system's credential store rather than in the application's SQLite database.
- AI assistance is optional. The original design describes local AI-compatible providers such as Ollama, LM Studio, or a custom OpenAI-compatible endpoint.
- AI requests are explicit: the user asks a question from a repository's AI workflow, and the intended context is repository facts already computed by the application or files specifically selected by the user.

This section describes the documented design and checked-in integration approach. It is not an independent security audit or a guarantee that every build and configuration behaves identically. Review the code and your local configuration before using the application with sensitive repositories or credentials.

## Project status and support

**Status: archived / no longer maintained by the original author.**

Haleacas has been merged into a larger project. Development continues, if at all, in that separate project—not in this repository.

- This repository is retained for reference.
- Issues and pull requests may not receive a response.
- There may be unresolved bugs, incomplete features, or outdated dependencies.
- No security response time or compatibility commitment is offered here.
- If you are considering a fork, please review the code, dependencies, and security implications first.

If you arrived here looking for the actively developed successor, check the original author's GitHub profile and repositories for the current project. No successor URL is specified in this archive README.

## Known limitations

- This is an early implementation, not a claim of a finished or production-hardened product.
- The feature set and level of test coverage vary across modules.
- Some integration tests require external services and are ignored by default.
- Platform packaging and signing may need additional setup.
- The archived dependency versions may become incompatible with future toolchains.

## License

Haleacas is distributed under the [MIT License](LICENSE).

You are welcome to study, fork, and adapt the code under the terms of that license.

---

*Thanks to everyone who explored, tested, or found something useful in Haleacas. Even when a project moves on, its code can still be a useful starting point for someone else.*
