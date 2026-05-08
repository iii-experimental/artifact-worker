# artifact-cli

Turn APIs, specs, docs, and workflow artifacts into narrowly scoped **Rust iii workers**.

`artifact-cli` is a research project for agent-operable backend surfaces: instead of giving an agent a giant API wrapper or asking it to read docs at runtime, generate a focused Rust worker with a small set of precise iii functions.

```text
artifact -> narrow Rust iii worker -> callable functions
```

## Why this exists

Agents are better when they call stable functions instead of browsing docs, guessing endpoints, or stitching workflows from scratch. `artifact-cli` creates small iii-native Rust workers around a specific job:

- `linear_risk::blocked_issues`
- `github_repo::stale_prs`
- `docs_search::answer_with_sources`
- `hn::top_stories`

The point is not to generate every endpoint. The point is to generate the few functions an agent actually needs.

## Why Rust

`artifact-cli` is infrastructure: parsing, planning, generation, verification, packaging, filesystem work, and eventually worker registry publishing. Rust gives us a single binary, strong manifests, safer execution boundaries, and a cleaner path to binary workers for the iii ecosystem.

## How it fits iii

`artifact-cli` composes with existing workers from [workers.iii.dev](https://workers.iii.dev/):

- `iii-state` — store manifests, source fingerprints, generated worker metadata
- `iii-queue` — run generation and verification asynchronously
- `iii-cron` — refresh synced artifacts on a schedule
- `iii-database` — back generated workers with SQLite/Postgres mirrors
- `iii-sandbox` — build and test generated workers in isolation
- `iii-http` — expose generated functions as HTTP endpoints
- `iii-observability` — traces, logs, and generation/debug telemetry
- `iii-bridge` — share generated workers across iii systems

## Current Rust functions

The Rust core exposes:

- `inspect_artifact` — classify a source artifact and suggest focused worker functions
- `plan_worker` — produce a narrow worker plan from an artifact description
- `generate_worker` — generate a Rust iii worker scaffold
- `verify_worker` — run structural checks on a generated worker

The CLI binary exposes matching commands:

```bash
cargo run --bin artifact-cli-worker -- plan \
  --name hackernews \
  --goal "give agents focused access to top stories and item lookup" \
  --source https://github.com/HackerNews/API
```

Generate a Rust worker scaffold:

```bash
cargo run --bin artifact-cli-worker -- generate \
  --name hackernews \
  --source https://github.com/HackerNews/API \
  --function top_stories,get_item,search_cached_stories \
  --output-dir ./generated/hackernews-worker
```

Verify it:

```bash
cargo run --bin artifact-cli-worker -- verify --output-dir ./generated/hackernews-worker
```

## Generated worker shape

```text
generated/hackernews-worker/
  Cargo.toml
  src/main.rs
  artifact.manifest.json
  README.md
```

Each generated Rust worker keeps function IDs explicit, e.g.

```text
hackernews::top_stories
hackernews::get_item
hackernews::search_cached_stories
```

## Principles

1. **Rust-first** — core, CLI, worker runtime, and generated workers should be Rust.
2. **Narrow beats generic** — generate workers around jobs, not every endpoint.
3. **Functions over docs** — agents call `function_id` instead of reading docs at runtime.
4. **Composable by default** — use existing iii workers for state, queues, cron, database, HTTP, sandboxing, and observability.
5. **Inspectable artifacts** — every generated worker ships with a manifest and verification report.
6. **No hidden side effects** — generated functions should declare whether they read, write, sync, or call external systems.

## Development

```bash
cargo fmt
cargo test
cargo run --bin artifact-cli-worker -- plan --name hackernews
```

## Status

Early Rust MVP scaffold. The first implementation focuses on planning and generating Rust worker skeletons. Runtime registration against `iii-sdk` will be wired once the worker API surface is pinned for this repo.
