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

## Agent Responsibilities 🎯
Agents operating on this repo must:
- Respect the project's policies (no deprecations, aggressive refactoring allowed with adequate tests).
- Run the right tools and commands to verify changes (build/test/lint etc.).
- Keep changes focused, atomic, and well-tested. If a change affects many files, include a migration plan and tests.
- Provide clear commit messages and PR descriptions with the rationale for changes.
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
```bash
cargo test --package <package> --test <name>
cargo test --lib <function_name>
```

---

## Code/Project Patterns (Repo rules & helpful links) 📚
- `packages/abra` is the core library. `apps/alakazam` is the GUI app built on Abra.
- `mod.rs` should only contain module declarations and re-exports.
- Public API should use clean simple names; internal structs should use `Inner` suffix (e.g., `CanvasInner`).
- Follow the naming patterns in `.github/instructions/naming-conventions.instructions.md` and coding patterns in `.github/instructions/patterns.instructions.md`.

---

## Policies & Constraints ⚖️

### No Deprecations Policy — (detailed)
We follow a strict **No Deprecations** policy: do not keep old code paths, deprecated branches or legacy fallbacks in the codebase. Instead, whenever a feature or an API is replaced, the old implementation should be removed (or migrated) as part of the change. This keeps the codebase clean and maintainable and reduces complexity.

Why:
- Avoids accumulation of dead code paths and technical debt.
- Makes refactors and code health improvements straightforward for future contributors.
- Forces agents and developers to be explicit about migration and breakages.

Scope and exceptions:
- Applies to any code under workspaces in this repository (core libraries, apps, tests).
- Public APIs and FFI surfaces are not guaranteed to be stable: this repo is a private/internal implementation and breaking changes are acceptable. Agents may update or remove FFI/public APIs without preserving backwards compatibility and are not required to coordinate migrations.
 - Public APIs and FFI surfaces are not guaranteed to be stable: this repo is an internal implementation and a breaking change is acceptable regardless of FFI or public API impact. Agents may update or remove FFI/public APIs without preserving backwards compatibility.
 - No temporary transitional or compatibility helpers: temporary shims, compatibility wrappers, or transitional helpers are not permitted. Agents must update call sites directly or coordinate a staged plan with maintainers if they still feel coordination is needed (coordination is optional, not required for compatibility reasons).

Required steps for agents making a change:
1. Discover usages: search the repository for all use sites of the old API (e.g., `rg 'old_function_name'` or `git grep -- 'old_function_name'`).
2. Add test coverage: add a test that shows the earlier behavior — then implement the new behavior and update tests.
3. Implement new feature or refactoring and update all call sites.
4. Remove all references to the old API and delete the old implementation.
5. Update docs and examples where applicable; add a short migration guide in the PR description.
6. Run a full workspace build & tests and adjust fixups for any dependent modules.

- Migration & Replacement Flow (optional):
- Optionally create a short plan in the PR description titled "Migration Plan" including: Motivation, Affected Modules & Call Sites, Tests Added/Updated, Breaking Behavior, and Rollback Plan. This is useful for reviewer context but not required.
- Update all call sites in the same change when possible. If a staged migration is required, coordinate in the PR description with maintainers and include a clear migration plan and timeline — but do not introduce temporary transitional helpers into the codebase.
- Run automated search and replace across the repo only where safe. Add a human review of the find/replace commits in PRs.
- Add a short note to the `docs/` directory or your package's README if this change affects public usage.

Bad and Good code practice examples
```rust
// Bad: Adding a new feature while keeping the old one
pub fn old_feature() { /* Old implementation */ }
pub fn new_feature() { /* New implementation */ }
// Usage of the features
pub fn use_feature() {
  if use_old {
    old_feature();
  } else {
    new_feature();
  }
}
```

```rust
// Good: Removing the old feature when adding a new one
pub fn new_feature() { /* New implementation */ }
// Usage of the new feature
pub fn use_feature() { new_feature(); }
```

Agent enforcement checks you should run locally in a change that replaces functionality:
- `git grep -n "old_feature\(|old_feature\b" || true` — confirm no references remain.
- `cargo test --workspace` — tests must pass.
- `cargo build --workspace --release` — build to ensure no hidden failures.

### Aggressive Refactoring
Aggressive refactoring is encouraged, since we prefer active cleanup and cohesion over keeping legacy paths. When doing large refactors, break them into smaller PRs where possible, add tests for each step, and use the Migration & Replacement Flow above.

### When to ask for human review
- Optional: If the change affects the public API and may impact downstream consumers, tagging maintainers for awareness is optional — add context in the PR description, but this is not required by the policy.
- If the change impacts public APIs or FFI, tagging maintainers is optional; state the change in the PR description and write tests as appropriate.
- If the change makes cross-cutting changes across the workspace in multiple packages, notify maintainers and ensure tests and examples are updated.

### FFI/Bindings Surface
- When modifying FFI boundaries, ensure the FFI contracts are clearly described; coordinating with bindings authors (example: `bindings/javascript/`) is optional. Agents may change FFI boundaries without preserving backward compatibility, but they should update `bindings/` packages and document changes if those bindings exist.

---

---

## PR Checklist for Agents (automated checks & human review items) ✅
Every PR created by an automated agent should include:
1. Summary: Short explanation of what changed and why.
2. Test suite: Add new tests or update existing ones to cover the change. Run `cargo test`.
3. Build: Run `cargo build --release` and ensure no errors.
4. Linting (optional but recommended): `cargo fmt` and `cargo clippy --all-targets`.
5. Documentation updates: If behavior or API changes, update docs in `docs` and doc comments.
6. Modularity: Avoid very large monolithic commits; prefer smaller logical commits with clear messages.
7. No outdated code paths: Remove legacy code instead of leaving dead code paths.
8. CI-friendly: If the repo uses more checks (like GitHub Actions), confirm those steps run in PRs.

Additional items for changes that remove or replace existing APIs or behavior (enforced by the No Deprecations Policy):
- Migration Plan: Add a `Migration Plan` section in the PR description (see Migration & Replacement Flow) describing the change and how to update invocation sites.
- Call sites: List all changed call sites in the PR description and include a brief note about each update.
- Usage grep: Add the exact command run to verify no references remain; e.g., `git grep -n "old_feature\(|old_feature\b"` or `rg 'old_feature' || true`.
- Docs: Update `docs/` and inline doc comments to document the new API and migration steps.
- Breaking change tag: Optional — you may mark the PR as a **breaking change** to signal to other maintainers or review automation, but this is not required by policy.

If the agent cannot perform 2–5, note the reasons in the PR and ask for a human reviewer.

---

## Error Handling & Escalation (When the agent is blocked) 🚨
- If a build fails due to unknown reasons, gather logs and create a short issue or open a draft PR with the failure attached.
- If the change requires a design decision (API or UX), add a detailed comment to the PR and request human review.
- For UI/UX changes, record a screenshot or short recording of the change and add to PR description.

---

## Example Workflows (typical agent tasks) 🔄
- Bug fix flow:
  1. Reproduce the bug locally.
  2. Add a failing test that reproduces the bug.
  3. Implement the fix and run tests locally.
  4. Update docs if needed, create PR, and request a human reviewer for edge-case validation.

- Refactor flow (aggressive):
  1. Add tests around existing behavior.
  2. Apply refactor and update call sites/tests.
  3. Run all tests and clippy/fmt; verify.
  4. Create PR with explanation of why the change improves the system.

---

## Agent Safety & Security Notes 🔒
- Do not run any untrusted external scripts or install packages outside this repo without explicit permission.
- Avoid committing secrets or credentials. If any secrets are required for testing, use `env` injection in CI or local variables; never commit them.

---

## Useful Links & Files 🔗
- Project README: see root `README.md` (if present)
- Coding patterns and naming: `.github/instructions/` directory (naming-conventions and patterns).
- App: `apps/alakazam`
- Library: `packages/abra`
- Test programs: `packages/tests`.

---

## Helpful Extras for Automated Agents ✨
If you are an agent that can run actions or tools, add these to your local checks or CI steps:
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check` (to detect formatting changes)
- `cargo clippy --all-targets --all-features -- -D warnings` (if you need stricter checks)

---

## FAQ (short answers for common agent questions) ❓
- Q: Can I make large changes? A: Yes — but include tests, docs, and run a full test suite. Keep PRs well-documented.
- Q: Where do I ask for help? A: Open a PR and add maintainers as reviewers, or open an issue describing the problem.

---

If you want me (an agent) to include automation that enforces parts of this document, tell me which checks to add to the CI and I can prepare a draft PR.

----

_Generated and tailored for agent consumption. Keep this doc concise and update it as the workflow evolves._

---

## Working with this project (high-level summary)
This project uses Cargo as its build system and package manager, and targets the Rust 2024 edition.

Key project facts:
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
