# CLAUDE.md — XStar Adventures

## TL;DR
- `docs/` is the canonical source of truth — treat specs as constraints
- FixedUpdate for all simulation; strict simulation/UI separation
- No `unwrap()`/`expect()` — explicit error handling only
- Ask before expanding scope
- Don't commit unless explicitly asked

---

## Project Structure
```
xstar-adventures/
├── docs/                    # Canonical design specs (authoritative)
│   ├── one-page-mvp.md      # High-level vision
│   ├── mvp.md               # MVP scope definition
│   ├── bevy-architecture.md # ECS patterns & conventions
│   └── design.md            # System interactions
└── src/                     # Game implementation
```

---

## Design Principles
> Reference only — full rationale lives in `docs/design.md`

- **Systemic, not mission-driven** — emergent gameplay over scripted content
- **Delegation as core loop** — player commands, doesn't micromanage
- **Failure creates gameplay** — setbacks open new paths, not punishment
- **No permanent solutions** — no system may "solve" the sector
- **Player relevance at all scales** — presence always matters
- **Finite runs** — target 60–90 minutes

---

## Architecture Rules

### Bevy / Simulation
- All simulation logic in **FixedUpdate**
- Simulation state and presentation/UI are strictly separated
- UI may emit commands but **never mutates simulation state directly**
- Prefer explicit enums and state machines over implicit flags
- Deterministic behavior under fixed seeds is required

### Code Organization
- No "god files" — split into focused modules/plugins mirroring `docs/`
- Pure logic in testable modules
- State transitions must be unit-tested

---

## Commands
```bash
cargo test                      # Build + run all tests (preferred)
cargo run                       # Launch game
cargo fmt --check               # Verify formatting
cargo clippy -- -D warnings     # Lint (warnings as errors)
```

**Do not use `cargo build`** — use `cargo test` instead, which builds and tests in one step.

Do not access network resources or install global dependencies.

### Verification Workflow
After writing or modifying code:
1. Run `cargo test`
2. Fix **all** errors and warnings from the output
3. Repeat until clean
4. Run `cargo clippy -- -D warnings` and fix any issues
5. Run `cargo fmt --check` (or `cargo fmt` to auto-fix)

---

## Workflow

### Before Writing Code
1. Check relevant docs in `docs/` for existing constraints
2. If a change impacts multiple systems, plan updates to all affected docs

### Scope Changes
- **Ask before proposing or implementing scope expansion**
- Prefer refinement over adding new systems
- Gut check: *"Does this preserve pressure, responsibility, and player relevance?"*

### Commits
Only create commits when explicitly requested. Format:
```
<imperative title, ≤72 chars>

- Change one
- Change two
```

### Error Recovery
If 2+ attempts fail, **stop and ask** rather than retrying.

---

## Enforced via Tooling
> These rules are checked by `cargo clippy` and `cargo fmt`. Claude should run these commands to verify compliance rather than manually checking.

- No `unwrap()`, `expect()`, or bare `?` — use explicit `match` or `map_err`
- Always use braces `{}` for `if/else/for/while`
- Code must pass `cargo fmt`
- Prefer functional style: pure functions, explicit inputs/outputs, minimal side effects

---

## Never Allowed
- Accessing network resources
- Installing global dependencies
- Deleting large code sections without clear justification
- Silent divergence between docs — if one changes, update all affected
