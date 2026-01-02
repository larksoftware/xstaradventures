# GOAL
Make the game build-ready and runnable for inspection.

# TASK
Complete the next 10 prioritized tasks using this loop:

For task i in 1..10:
1) Implement task i.
2) Add/extend unit tests to comprehensively cover the change.
3) Run tests (full suite).
4) If tests emit errors OR warnings: fix and repeat steps (2)-(3) until clean.
5) Only then proceed to task i+1.

After task 10:
- Print the next 10 tasks (priority order).
- List new things the user can try that were not available in the last build.

STOP and ask questions if any required information is missing.

# Quality Bar
- Zero test warnings
- Zero test errors
- Build succeeds
- Game runs locally to first interactive state


# GOAL
Increase test coverage while keeping the project stable. After completing these tests, the **entire test suite must pass** (no errors, no warnings).

# TASK
Implement the **next 10 tests** (prioritized for maximum coverage impact) using the execution rules below. After each test, run the full suite and fix issues before moving on. After finishing, compute coverage via `cargo llvm-cov`, and if total coverage exceeds **90%**, you may stop.

## Execution Rules (must follow for every test)
For each test (1..10):
1. Add exactly **one** new test (or one cohesive test case addition) targeting uncovered/under-tested code.
2. Run the full test suite.
3. The suite must pass with **no errors and no warnings**.
4. If any error or warning occurs, **fix it immediately** and re-run the suite until clean.
5. Only then proceed to the next test.

## Coverage Gate
After finishing the 10 tests:
1. Run:
   - `cargo llvm-cov --json --summary-only --output-path target/llvm-cov/summary.json`
2. Parse `target/llvm-cov/summary.json` and check overall coverage.
3. If overall coverage is **> 90%**, you may stop.
4. If not, print the **next 10 tests** (priority order) to implement next.

## Output Requirements
When complete (after coverage check):
- Print:
  1) The next 10 tests to work on (priority order)
  2) Current overall coverage percentage
  3) Any remaining coverage hotspots (files/modules/functions) that would yield the biggest gains

## Stop Condition
If you need clarification (missing test harness, unclear expected behavior, ambiguous edge cases, or missing fixtures), **stop immediately** and ask questions before writing more tests.

# Definition of Done
- `cargo test` (or project’s full test command) passes cleanly
- No warnings emitted during tests
- Coverage computed from `cargo llvm-cov` and reported
- If coverage > 90%, testing work may stop
