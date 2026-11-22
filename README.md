# Haleacas

> Offline-first repository intelligence toolkit for developers.
>
> "Haleacas" is a placeholder name. It's used consistently throughout the
> code/config (package name, Tauri identifier, window title) so it can be
> renamed globally later — see **Renaming** below.

Haleacas sits above Git, GitHub, and your editor and understands the
repositories around them: what they are, whether they're healthy, and what
needs attention — without replacing any of those tools.

## Status

This is an early, real, compiling implementation — not a mockup. See
[`STATUS.md`](./STATUS.md) for exactly what's built vs. planned, phase by
phase.

## Stack

- **Backend:** Rust + [Tauri 2](https://v2.tauri.app/), SQLite (via `rusqlite`),
  `git2` for Git integration, `walkdir` for filesystem scanning.
- **Frontend:** React + TypeScript + Vite + Tailwind CSS, `react-router-dom`
  for navigation.

## Development setup

Prerequisites:

- Node.js 18+ and npm
- Rust 1.77.2+ (`rustup` recommended) and Cargo
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`,
  `librsvg2-dev`, `libsoup-3.0-dev`, `libgit2-dev`, `pkg-config`, `build-essential`
- Windows: the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)
  (WebView2 runtime + MSVC build tools)

```bash
npm install
npm run tauri dev
```

## Production build

```bash
npm run tauri build
```

Produces a platform installer under `src-tauri/target/release/bundle/`.
Windows distribution is the primary target per the product spec; the
`tauri.conf.json` bundle config currently targets `"all"` and should be
narrowed once code signing / installer details are finalized.

## Testing

```bash
# Domain logic (fast — no GUI toolchain needed)
cd src-tauri && cargo test -p haleacas-core

# Includes tests that hit a real GitHub API and a real local AI provider
# (both skipped by default — see STATUS.md for what each needs to run)
cd src-tauri && cargo test -p haleacas-core -- --ignored

# Tauri shell — type-checked only for now, see STATUS.md
cd src-tauri && cargo check -p app

# Frontend
npm run build   # type-checks via tsc -b, then builds
```

`haleacas-core` (repository scanning, Git integration, persistence) is a
plain Rust crate with no Tauri/GUI dependency, specifically so its test
suite compiles and links quickly and can run in CI without a display —
see `STATUS.md` for the current test count and what's still uncovered.

## Privacy

Repository discovery, Git reads, and the SQLite cache are local-only and
never leave your machine. Two things are optional and off by default:

- **GitHub** — only contacted when you associate a repository with GitHub
  functionality (the GitHub tab/Inbox); public-repo data is fetched
  unauthenticated unless you connect a personal access token, which is
  stored in your OS keychain, never in Haleacas' own database.
- **Local AI** — off until you configure a provider (LM Studio, Ollama, or
  a custom OpenAI-compatible endpoint) in Settings. All three are local
  servers on your own machine, not cloud services; no API key is used.
  Nothing about a repository is sent until you explicitly ask a question
  in that repository's AI tab, and only two kinds of context exist:
  repository facts Haleacas already computed, or specific files you've
  selected — never the whole repository, and the exact text sent is
  always shown alongside the answer. Cloud AI providers are not
  implemented.

## Renaming from "Haleacas"

Search-and-replace these identifiers once a final name is chosen:

- `productName` / `identifier` in `src-tauri/tauri.conf.json`
- `name` in `package.json` and `src-tauri/Cargo.toml`
- `name`/`description` in `src-tauri/crates/haleacas-core/Cargo.toml`
- `ProjectDirs::from("dev", "haleacas", "Haleacas")` in
  `src-tauri/crates/haleacas-core/src/db/mod.rs`
- Window title in `index.html` and `tauri.conf.json`
- Tray tooltip string in `src-tauri/src/system/tray.rs`

## License

MIT — see [`LICENSE`](./LICENSE).
