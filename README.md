# Haleacas

Haleacas is a desktop app for keeping track of local software repositories. It brings repository discovery, Git history, dependency information, and a few GitHub tools into one place.

The app is built with Tauri, Rust, React, and TypeScript. Its Rust core handles much of the repository and Git work, while the frontend provides the desktop interface.

## Project status

Haleacas has been merged into a larger project. This repository is no longer being developed by the original author and is kept here for reference.

The code is still available to read, build, and fork under the terms of the MIT License. Please keep in mind that some parts may be unfinished, dependencies may be out of date, and issues or pull requests may not receive a response.

## What it includes

The codebase contains features for:

- Finding repositories in configured folders
- Checking repository information and maintenance findings
- Viewing Git history and commit statistics
- Inspecting dependencies from supported project files
- Searching across discovered repositories
- Reviewing and editing README content
- Connecting to GitHub for repository details, releases, inbox items, and Actions information
- Working with GitHub Pages information
- Asking questions through an optional, configured AI-compatible service
- Using desktop features such as the system tray and launch-at-login settings

This list describes functionality present in the source code. It does not mean every feature is complete or ready for production use.

## Project layout

| Path | Description |
| --- | --- |
| `src/` | React and TypeScript frontend |
| `src-tauri/` | Tauri application and Rust backend |
| `src-tauri/crates/haleacas-core/` | Core repository, Git, and persistence logic |
| `public/` | Static frontend assets |
| `.github/workflows/` | GitHub Actions workflows |
| `package.json` | Frontend scripts and dependencies |
| `package-lock.json` | Locked npm dependency versions |
| `Cargo.toml` files | Rust workspace and crate configuration |
| `LICENSE` | MIT License |

The `haleacas-core` crate is separate from the Tauri desktop application. Its tests can run without opening the GUI.

## Requirements

To build the application, you need:

- Node.js and npm
- Rust and Cargo
- The platform dependencies required by [Tauri 2](https://v2.tauri.app/start/prerequisites/)

Tauri's requirements differ between Linux, Windows, and macOS. Check its documentation for the packages and tools needed on your system.

## Setup

Install the frontend dependencies from the repository root:

```bash
npm install
```

Start the desktop app in development mode:

```bash
npx tauri dev
```

Build the desktop app:

```bash
npx tauri build
```

Build output is placed under:

```text
src-tauri/target/release/bundle/
```

The installer formats available depend on your operating system and its packaging tools.

## Useful commands

These are the npm scripts currently defined in `package.json`:

| Command | Description |
| --- | --- |
| `npm run dev` | Start the Vite development server |
| `npm run build` | Type-check and build the frontend |
| `npm run lint` | Run Oxlint |
| `npm run preview` | Preview the frontend build |

Run the core Rust tests:

```bash
cd src-tauri
cargo test -p haleacas-core
```

Some tests are marked as ignored because they need access to external services, such as GitHub or a local AI provider. Review the tests and configure those services before running:

```bash
cd src-tauri
cargo test -p haleacas-core -- --ignored
```

Check the Tauri application crate:

```bash
cd src-tauri
cargo check -p app
```

## Data and integrations

Most repository scanning and Git inspection is intended to happen locally. The application also uses SQLite for local data.

GitHub features make network requests when used. The original project documentation describes storing a connected GitHub token in the operating system's credential store.

AI support is optional and depends on the provider configured by the user. The original design includes local AI services and OpenAI-compatible endpoints. Requests are made from the relevant AI workflow in the app.

These notes describe the project's intended design. They are not a security audit. If you plan to use this code with private repositories or credentials, review the implementation and configuration first.

## Building on this project

You are welcome to fork the repository and make changes under the MIT License. Since it is no longer maintained here, you will need to handle dependency updates, bug fixes, and compatibility changes in your own fork.

If you are looking for the project that includes the merged work, check the original author's GitHub repositories for the current location. No successor link is listed here.

## License

Haleacas is released under the [MIT License](LICENSE).
