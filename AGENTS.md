# Agent Guide for the rust-image Workspace

This file explains _how an automated agent_ should interact with this repository. It includes environment setup, common commands, constraints, tests.

> Note: The repository already contains useful policy files and developer instructions such as `.github/instructions/*` and `AGENTS.md` in other subfolders; use those files as a source of truth when applicable.

## NEVER RUN THE FOLLOWING COMMANDS

- **`cargo build` or `cargo run` (ABSOLUTELY NEVER run these except when running examples in the `examples/` folder; otherwise, use `npm run build-bindings` instead).**
- **`npm run dev` or any command that starts a server or Electron app (the development server is already running and will auto-reload on changes; running this will crash the server causing errors).**

---

## Quick Summary (TL;DR) ✅

- Use Cargo to build, test, and run examples. The workspace uses Rust 2024 edition.
- **CRITICAL RULE FOR AGENTS: NEVER run `npm run build-bindings` unless Rust files have been added or modified. Agents MUST check for Rust file changes before executing this command. Running it otherwise is unnecessary and may cause issues.**
- Follow the **No Deprecations Policy** — remove legacy code rather than keep it behind feature flags.
- When refactoring, update all call sites and add/adjust tests.
- This is NOT a public library: you are NOT required to preserve backward compatibility (including public APIs or FFI). Breaking changes are allowed and expected; you do not need to perform migrations for downstream consumers.
- Status: ALPHA — The library is in active alpha development. APIs, FFI and behavior are unstable and may change frequently. Agents should expect, document, and be comfortable making breaking changes.

---

## Alpha stage: what agents should expect

- Frequent changes: The API surface may change frequently; expect breakages across versions.
- No backward compatibility promise: Agents are not required to preserve old APIs across PRs.

---

## Design Principles & Long-term Approach 💡

These principles guide decisions for long-lived code in the workspace. Agents must favor options that produce the best long-term outcome, even if that requires large refactors.

- Long-term-first: Always prefer the implementation that is clean, maintainable, and future-proof — not a temporary or drop-in change.
- Large refactors are encouraged: If a task requires a substantial refactor to produce a durable, correct and clean solution, do the refactor (and update call sites/tests accordingly). Do not hedge with temporary fixes.
- Speed first: Optimize for runtime performance; prefer algorithmic improvements, efficient data structures, and reduced allocations for image-processing features.
- Usability & single source-of-truth: Make one authoritative implementation for each capability (for example, prefer `packages/abra`/`packages/core`/`drawing` to provide the single canonical implementation for a feature). Avoid multiple implementations doing the same thing.
- Observe the No Deprecations Policy — replace, migrate or remove rather than leaving legacy code around.

### Single Source of Truth (SSoT) — Policy & Workflow 🔧

To keep the codebase clear, predictable, and maintainable, agents must follow a strict "Single Source of Truth" policy for each capability or feature area.

- Why: Multiple implementations of the same feature are expensive to maintain, increase the risk of bugs, and introduce API drift. The SSoT approach ensures a single authoritative implementation is optimized, documented, and tested.
- Selection & Designation: When an agent introduces a new capability or consolidates duplicate implementations, they must:
  1. Choose a single authoritative location for the implementation (e.g., `packages/abra`, `packages/core`, or `drawing`). Consider performance, clarity, consumer-facing APIs, and test coverage when making this choice.

2.  Add a short doc comment to the chosen implementation documenting that this is the SSoT and why it was chosen.
3.  Update `docs/` and any README files that point users to the canonical location.

- Consolidation Steps: If duplications are found across the repo, agents must:
  1. Search for duplicate implementations using `rg` or `git grep` (commands below).
  2. Designate the authoritative implementation and leave a clear `/// NOTE` doc comment on it explaining the reason and the date of consolidation.
  3. Add tests (and benchmarks when relevant) for SSoT and update/replace tests that referenced older implementations.
  4. Remove or delete the old implementations. If there's a reason to retain them temporarily, use a clear deprecation comment and schedule for removal—do not keep them indefinitely.
- Conflict Resolution & Exceptions: If the repository legitimately requires multiple implementations (e.g., different backends, FFI constraints), agents must:
  1. Centralize shared logic in the SSoT and keep backend-specific code minimal and isolated.
  2. Avoid feature duplication across modules where possible; prefer small adapter layers that call into SSoT.
- Commands & tools (useful when consolidating):

```bash
# Find duplicate/similar function names across workspace
rg "fn <function_name>\b" || true
git grep -n "<function_name>\b" || true

# Find duplicate type names or struct definitions
rg "struct <TypeName>\b|enum <TypeName>\b" || true

# Search for implementations of similar logic (e.g., gaussian blur variants)
rg "gaussian|blur|box_blur|fast_blur" || true
```

Follow the above commands by evaluating the duplicates and selecting the best canonical implementation.

### Version & Stability — Alpha

This project is in the ALPHA stage. Treat the workspace as an evolving, experimental implementation. Key notes:

- APIs and FFI are unstable and may change at any time.
- Backward compatibility is not a requirement — design and code for best long-term architecture rather than short-lived compatibility.

### Performance & Usability (core priorities)

These are non-negotiable priorities for this workspace. When designing or changing features, prioritize:

- Speed: Prefer algorithmic improvements over micro-optimizations that complicate code. Minimize allocations and copies; use efficient data structures and algorithms for image-processing-heavy code.
- Usability: Choose a single authoritative implementation for each capability. The `drawing` crate or `packages/abra` may serve as the canonical implementation for a feature; avoid duplicating logic in multiple places.
- Predictability and API clarity: Use stable, simple, and explicit names for public API items. Prefer a single consistent naming and behavior across the workspace.

Commands & checks for performance and duplication:

```bash
# Run benchmarks (if you use criterion/benches):
cargo bench --package <package>

# Quick runtime measure (local builds, iterations):
cargo run --release --bin <example> # measure with external timer

# Find duplicate implementations / call sites:
rg "fn <function_name>\b" || true
git grep -n "<function_name>\b" || true
```

If you find duplicate implementations, consolidate into the authoritative implementation and update call sites. Add a benchmark and tests to validate performance and correctness after the change.

---

## Agent Responsibilities 🎯

Agents operating on this repo must:

- Respect the project's policies (no deprecations, aggressive refactoring allowed with adequate tests).
- Run the right tools and commands to verify changes (build/test/lint etc.).
- Keep changes focused, atomic, and well-tested. If a change affects many files, include a short change plan and tests.
- Do not be concerned about preserving backward compatibility. Breaking changes to public APIs and FFI are allowed and expected — update call sites and tests accordingly.
- Prefer the long-term solution: select the design or implementation that will scale better and remain maintainable. If a big refactor is needed, do the refactor.
- Optimize for performance and usability: provide microbenchmarks or benchmarks when making performance-related changes, and update the authoritative implementation as the 'source of truth'.
- Run the `Run` task in local builds for GUI verification when making UI changes (see Run Tasks section).

---

## Environment & Setup ⚙️

Minimum environment expectations for an agent or CI:

- Rust toolchain installed (matching the CI/rust-toolchain file if present). Use `rustup`.
- Cargo available on path.
- OS: Windows is supported as shown; Linux/macOS also supported for development.
- Optional: If running the GUI (`alakazam`) set `SLINT_BACKEND` env var as in VS Code tasks.

Commands to set up (example for Git Bash / Windows `bash.exe`):

```bash
# Install toolchain if needed (example):
rustup toolchain install stable
rustup default stable

# Verify toolchain and environment:
rustc --version
cargo --version
```

---

## Common Agent Commands (How to run & check) 🧭

Use these commands to build and validate changes.

- Build the whole workspace (debug):

```bash
cargo build
```

- Build release:

```bash
cargo build --release
```

- Run tests (whole workspace):

```bash
cargo test
```

- Run a specific example/test package (using the `examples` or workspace packages):

```bash
# Running an example package - adjust package and bin names
/c/Users/untun/.cargo/bin/cargo.EXE run --package <package-name> --bin <package-name>
```

- Run the UI app `alakazam` (Run task provided in VS Code):

```bash
/c/Users/untun/.cargo/bin/cargo.EXE run --bin alakazam
# or via the provided VS Code Run task which sets SLINT_BACKEND
```

- Run a specific test file or test function:

## Code/Project Patterns (Repo rules & helpful links) 📚

- `packages/abra` is the core library. `apps/alakazam` is the GUI app built on Abra.
- `mod.rs` should only contain module declarations and re-exports.
- Public API should use clean simple names; internal structs should use `Inner` suffix (e.g., `CanvasInner`).
- Follow the naming patterns in `.github/instructions/naming-conventions.instructions.md` and coding patterns in `.github/instructions/patterns.instructions.md`.

---

## Policies & Constraints ⚖️

- Makes refactors and code health improvements straightforward for future contributors.
- Forces agents and developers to be explicit about breaking changes and call-site updates.

Scope and exceptions:

- Applies to any code under workspaces in this repository (core libraries, apps, tests).
- Public APIs and FFI surfaces are not guaranteed to be stable: this repo is a private/internal implementation and breaking changes are acceptable. Agents may update or remove FFI/ public APIs without preserving backwards compatibility; coordination with maintainers is optional and not required.
- No temporary transitional or compatibility helpers: temporary shims, compatibility wrappers, or transitional helpers are not permitted. Agents must update call sites directly; coordination with maintainers is optional and not required.

Required steps for agents making a change:

1. Discover usages: search the repository for all use sites of the old API (e.g., `rg 'old_function_name'` or `git grep -- 'old_function_name'`).
2. Add test coverage: add a test that shows the earlier behavior — then implement the new behavior and update tests. Add benchmarks when making changes that affect performance.
3. Implement new feature or refactoring and update all call sites. Prefer a single authoritative implementation (the ‘source of truth’) for a capability. If multiple implementations exist, consolidate them.
4. Remove all references to the old API and delete the old implementation.
5. Run a full workspace build & tests and adjust fixups for any dependent modules. Run benchmarks and performance checks where relevant.

---

### Aggressive Refactoring

### FFI/Bindings Surface

- When modifying FFI boundaries, ensure the FFI contracts are clearly described; coordinating with bindings authors (example: `bindings/javascript/`) is optional. Agents may change FFI boundaries without preserving backward compatibility, but they should update `bindings/` packages and document changes if those bindings exist.

---

## Example Workflows (typical agent tasks) 🔄

1. Add tests around existing behavior.
2. Apply refactor and update call sites/tests.
3. Run all tests and clippy/fmt; verify.
4. Run benchmarks and performance checks for performance-sensitive changes and consolidate duplicate implementations into the authoritative (source-of-truth) implementation.

- Do not run any untrusted external scripts or install packages outside this repo without explicit permission.
- Avoid committing secrets or credentials. If any secrets are required for testing, use `env` injection in CI or local variables; never commit them.

- Project README: see root `README.md` (if present)
- Coding patterns and naming: `.github/instructions/` directory (naming-conventions and patterns).
- App: `apps/alakazam`
- Test programs: `examples`.

---

This project uses Cargo as its build system and package manager, and targets the Rust 2024 edition.

- `packages/abra` is the core image processing library.
- `apps/alakazam` is a GUI application built on top of Abra.
- `examples` contains small example and test programs.

### Project conventions and rules

- `mod.rs` files: only module declarations and re-exports.
- Public API: choose clear, simple names; internal structs should use `Inner` suffix.
- Follow naming and coding patterns in `.github/instructions/`.
- No Deprecations Policy: remove old code instead of leaving it in-place behind flags or checks.

### Examples and test programs

Use these for local verification and for reproducing bugs. Example programs are under `examples`.

## Development server (GUI app)

### Running the GUI app `alakazam`

The server is already running, it so it doesn't need to be started so do not run it again with `npm run dev`. When a file is saved/modified, the server will automatically reload.

### Rebuild bindings

**Do NOT run `cargo build` or `cargo run` (except when running examples in the `examples/` folder). You should ALWAYS use `npm run build-bindings` to test for code errors.**

When modifying rust code for the gui, run the command `npm run build-bindings` in the root of the workspace to rebuild the bindings. The server will automatically reload when the bindings are rebuilt.
