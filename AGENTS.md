<!--
Agent-oriented guide for operating in this repository.
This document is tuned to be actionable for automation and agent helpers (human overridden guidance included).
-->

# Agent Guide for the rust-image Workspace

This file explains *how an automated agent* (or a developer using an agent) should interact with this repository. It includes environment setup, common commands, responsibilities, constraints, tests, and a complete PR checklist.

> Note: The repository already contains useful policy files and developer instructions such as `.github/instructions/*` and `AGENTS.md` in other subfolders; use those files as a source of truth when applicable.

---

## Quick Summary (TL;DR) ✅
- Use Cargo to build, test, and run examples. The workspace uses Rust 2024 edition.
- Always run `cargo test` and `cargo build` (and optional linters) before opening a PR.
- Follow the **No Deprecations Policy** — remove legacy code rather than keep it behind feature flags.
 - When refactoring, update all call sites and add/adjust tests.
 - This is NOT a public library: you are NOT required to preserve backward compatibility (including public APIs or FFI). Breaking changes are allowed and expected; you do not need to perform migrations for downstream consumers.

---

## Design Principles & Long-term Approach 💡
These principles guide decisions for long-lived code in the workspace. Agents must favor options that produce the best long-term outcome, even if that requires large refactors.
- Long-term-first: Always prefer the implementation that is clean, maintainable, and future-proof — not a temporary or drop-in change.
- Large refactors are encouraged: If a task requires a substantial refactor to produce a durable, correct and clean solution, do the refactor (and update call sites/tests accordingly). Do not hedge with temporary fixes.
- Speed first: Optimize for runtime performance; prefer algorithmic improvements, efficient data structures, and reduced allocations for image-processing features.
- Usability & single source-of-truth: Make one authoritative implementation for each capability (for example, prefer `packages/abra`/`packages/core`/`drawing` to provide the single canonical implementation for a feature). Avoid multiple implementations doing the same thing.
- Observe the No Deprecations Policy — replace, migrate or remove rather than leaving legacy code around.

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
- Provide clear commit messages and PR descriptions with the rationale for changes.
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

- Run a specific example/test package (using the `packages/tests` or workspace packages):
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
```


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
5. Update docs and examples where applicable; add a short change summary in the PR description (optional — helpful for reviewers).
6. Run a full workspace build & tests and adjust fixups for any dependent modules. Run benchmarks and performance checks where relevant.

- Migration & Replacement Flow (optional):
- Optionally create a short plan in the PR description titled "Change Plan" or "Migration Plan" including: Motivation, Affected Modules & Call Sites, Tests Added/Updated, Breaking Behavior, and Rollback Plan. This is useful for reviewer context but not required.
- Update all call sites in the same change when possible. If a staged plan is required, optionally include a clear plan and timeline in the PR description — but do not introduce temporary transitional helpers into the codebase.
- Run automated search and replace across the repo only where safe. Add a human review of the find/replace commits in PRs.
- Add a short note to the `docs/` directory or your package's README if this change affects public usage.

---

## PR Checklist for Agents (automated checks & human review items) ✅
Before creating a PR, ensure the following items are addressed (mark as N/A if not applicable):

1. Summary: Short explanation of what changed and why; include the decision rationale for choosing the long-term solution.
2. Test suite: Add new tests or update existing ones to cover the change. Run `cargo test`.
2a. Benchmarks / perf checks (optional): If the change touches performance-critical code, include benchmarks or microbenchmarks and report before/after results (e.g., `cargo bench`, criterion, or other tools).
3. Build: Run `cargo build --release` and ensure no errors.
4. Linting: `cargo fmt` and `cargo clippy --all-targets`.
5. Documentation updates: If behavior or API changes, update `docs` and doc comments.
5a. Identify source-of-truth: If the change adds or modifies an implementation, state the authoritative implementation and consolidate duplicates if found.
6. Modularity: Avoid large monolithic commits; prefer smaller logical commits with clear messages.
7. No outdated code paths: Remove legacy code instead of leaving dead code paths.
8. CI-friendly: Ensure PRs trigger CI checks and respond to any automated feedback.
9. Breaking change tag: Optional — include if you want to signal the PR contains breaking changes; coordination with maintainers is optional.

If any item cannot be completed, note the reason in the PR and ask for a human reviewer.


Bad and Good code practice examples
```rust
// Usage of the features
pub fn use_feature() {
  if use_old {
    old_feature();
  } else {
  }
```

// Usage of the new feature
```

- `git grep -n "old_feature\(|old_feature\b" || true` — confirm no references remain.
- `cargo test --workspace` — tests must pass.
- `cargo build --workspace --release` — build to ensure no hidden failures.

### Aggressive Refactoring

### FFI/Bindings Surface
- When modifying FFI boundaries, ensure the FFI contracts are clearly described; coordinating with bindings authors (example: `bindings/javascript/`) is optional. Agents may change FFI boundaries without preserving backward compatibility, but they should update `bindings/` packages and document changes if those bindings exist.

---
## Error Handling & Escalation (When the agent is blocked) 🚨
- If a build fails due to unknown reasons, gather logs and create a short issue or open a draft PR with the failure attached.


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
- Test programs: `packages/tests`.

---
## Helpful Extras for Automated Agents ✨
If you are an agent that can run actions or tools, add these to your local checks or CI steps:
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check` (to detect formatting changes)
- `cargo clippy --all-targets --all-features -- -D warnings` (if you need stricter checks)
## FAQ (short answers for common agent questions) ❓
- Q: Can I make large changes? A: Yes — but include tests, docs, and run a full test suite. Keep PRs well-documented.
If you want me (an agent) to include automation that enforces parts of this document, tell me which checks to add to the CI and I can prepare a draft PR.

----


---
This project uses Cargo as its build system and package manager, and targets the Rust 2024 edition.
- `packages/abra` is the core image processing library.
- `apps/alakazam` is a GUI application built on top of Abra.
- `packages/tests` contains small example and test programs.

### Project conventions and rules
- `mod.rs` files: only module declarations and re-exports.
- Public API: choose clear, simple names; internal structs should use `Inner` suffix.
- Follow naming and coding patterns in `.github/instructions/`.
- No Deprecations Policy: remove old code instead of leaving it in-place behind flags or checks.

### Examples and test programs
Use these for local verification and for reproducing bugs. Example programs are under `packages/tests`.

---

## Where to get help and escalation
- Open an Issue if something fails in CI or for design/UX discussions.
- Use PR review to request human validation for decisions requiring policy exceptions.
