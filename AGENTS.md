# AGENTS.md — XStar Adventures Repository Guidelines

This repository is the **authoritative design and architecture workspace** for *XStar Adventures*.

It is intentionally **system-first**, **implementation-aware**, and designed to support a Bevy (Rust) implementation without drifting into mission-driven or content-first design.

All documents in this repo are **constraints**, not suggestions.

---

## 1. Repository Purpose

This repository currently serves as a **design/specification source of truth** for a Bevy-based game.

Gameplay code is minimal and bootstrapped for Bevy, but most work still defines:

* system behavior
* architectural boundaries
* MVP scope
* non-negotiable design intent

This repo is now a **design + implementation** hybrid.

---

## 2. Directory Structure

All design documents live under the `docs/` directory. Runtime code lives in `src/`.

```
.
├── AGENTS.md
├── Cargo.toml
├── src/
└── docs/
```

`AGENTS.md` defines **how work is done**. The `docs/` directory defines **what the game is**.

---

## 3. Canonical Documents (Source of Truth)

These files define scope, architecture, and core intent. They must remain internally consistent.

Located in `docs/`:

* `one-page-mvp.md` — MVP truth test and scope lock
* `mvp.md` — MVP requirements, success criteria, kill criteria
* `bevy-architecture.md` — Bevy ECS architecture plan
* `design.md` — Consolidated high-level design philosophy

If any of these change, downstream documents must be reviewed.

---

## 4. System Design Documents

These files define individual systems and their behavior. All are located in `docs/`:

* `station-lifecycle.md`
* `fleet-roles.md`
* `fleet-ai.md`
* `ui-delegation.md`
* `crisis-escalation.md`
* `pirate-escalation-ai-doctrine.md`
* `player-ship-late-game.md`
* `first-thirty-minutes.md`
* `run-ending.md`
* `factions.md`

If a change impacts another system, **update both files explicitly**. Silent divergence is not allowed.

---

## 5. Architectural Guardrails (Hard Rules)

Any future design or implementation must respect:

* The game is **systemic**, not mission-driven
* Delegation is the primary interaction model
* Failure creates gameplay; it is not punishment
* No system may permanently solve the sector
* Player presence must remain relevant at all scales
* Runs are finite (≈60–90 minutes)

If a proposal violates any of these, it must be rejected or redesigned.

---

## 6. Bevy / Rust Expectations

Use these commands:

* `cargo run`
* `cargo test`

Implementation expectations:

* Use **FixedUpdate** for all simulation logic
* Separate **simulation state** from **presentation/UI**
* UI may emit commands, but must not mutate simulation state directly
* Prefer explicit enums and state machines over implicit flags
* Deterministic behavior under fixed seeds is required

---

## 7. Naming & Style Conventions

### Files

* Use **kebab-case**
* Filenames should match system names
* Avoid vague names like `notes.md` or `ideas.md`

### Documentation

* Markdown only
* Short paragraphs
* Bullet lists preferred
* Headings should describe behavior, not intent

### Code (Future)

* Rust formatted via `cargo fmt`
* Explicit types for core systems
* Enums for lifecycle and escalation states

---

## 8. Testing & Validation Philosophy

Designs should be written with **testability** in mind even before code exists.

When implementation begins:

* Pure logic goes in testable modules
* State transitions must be unit-tested
* Simulation behavior must be deterministic

Example test naming:

* `tests/station_lifecycle_spec.rs`

---

## 9. Change & Scope Management

All commits **must follow this standard format**:

- A **title line no longer than 72 characters**
- Followed by a **blank line**
- Followed by a **bulleted list of changes**
- **One blank line between bullets**

#### Example

```
Add transcript-aware clip segmentation

- Parse Whisper JSON into time-aligned segments
- Replace fixed 30s slicing with semantic boundaries
- Add unit tests for edge cases and silence gaps
```

Additional rules:

- Use **imperative mood** in the title (e.g., “Add”, “Fix”, “Refactor”)
- Be concise but descriptive
- Do not squash unrelated changes into a single commit
- Do not create commits unless explicitly requested (agent rule)

### Pull Requests

- Include a short description of the change
- Note tests run (e.g., `cargo test`, `cargo clippy`) or state if not run
- Link related issues if applicable

Scope expansion must be **explicit and visible**.
Update `docs/one-page-mvp.md` and `docs/mvp.md` when scope changes.

---

## 10. AI Agent Rules

Any AI agent working in this repository must:

* Treat existing documents as **constraints**
* Ask before proposing scope expansion
* Prefer refinement over new systems
* Preserve delegation, pressure, and failure-driven gameplay

If uncertain, defer to the question:

> “Does this preserve pressure, responsibility, and player relevance?”

---

## 11. Final Principle

> This project succeeds if it ships a compelling **60–90 minute systemic run**.

Anything that does not serve that goal is out of scope.

---

## 12. Never Allowed

The following actions are never allowed:

* Installing **global** dependencies
* Accessing **network** resources
* Deleting large sections of code without clear justification

If repeated failures occur (2+ attempts), **stop and ask for guidance** rather than retrying indefinitely.

---

## 13. Rust Guidelines (When Code Is Added)

### Error Handling (Hard Rules)

* **NEVER** use `unwrap()`, `expect()`, or a bare `?`
* Use explicit `match` (or equivalent) for error handling and to preserve context
* Prefer returning structured errors (e.g., `Result<T, E>`) over panics

### Control Flow

* Always use braces `{}` for control structures (`if`, `else`, `for`, `while`, etc.), even for single-line bodies

### Formatting & Compatibility

* Keep code compatible with `cargo fmt`

### Style Preferences

* Prefer a **functional style** where it improves clarity:

  * favor pure functions and explicit inputs/outputs
  * minimize side effects (especially in simulation logic)
  * use small, composable helpers over deeply nested logic

### Modularity

* Break code into focused modules/components
* Avoid “god files” (e.g., a single 5,000-line file)
* Use a plugin/module structure that mirrors the architecture docs in `docs/`
