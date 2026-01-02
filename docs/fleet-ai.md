# XStar Adventures — Fleet AI Decision Model

This document formalizes how fleet AI **thinks, decides, fails, and escalates**. It is designed to be **code-adjacent**, predictable to the player, and rich in emergent behavior.

---

## 1. Core AI Philosophy

**Fleet AI is reasonable, not optimal.**

The AI does not attempt perfect play. It operates under:

* Limited information
* Conflicting priorities
* Time pressure

Players should be able to understand *why* a decision was made, even when the outcome is bad.

---

## 2. High-Level Decision Loop

Every fleet runs the same repeating loop:

1. **Intent** – What am I responsible for?
2. **Awareness** – What do I believe is happening?
3. **Evaluation** – How risky is this situation?
4. **Decision** – What action best satisfies my priorities?
5. **Outcome** – Apply consequences and report

No fleet skips steps. Complexity comes from data quality, not branching logic.

---

## 3. Intent Model

Each fleet has exactly **one primary intent** at any time.

Examples:

* Maintain mining output
* Protect Station Alpha
* Patrol Route Delta
* Scout Sector J4

Intent defines:

* Acceptable risk
* Priority weights
* Failure tolerance

Changing intent resets decision assumptions.

---

## 4. Awareness Model (Imperfect Knowledge)

Fleet awareness is **partial and degradable**.

Awareness inputs include:

* Scout reports (timed, outdated)
* Last known pirate activity
* Distance to support
* Fuel state
* Station status

Key rule:

> Fleets act on what they *believe*, not what is true.

Missing information increases mistake probability.

---

## 5. Risk Assessment

Risk is calculated continuously but interpreted through tolerance.

### Risk Factors

* Threat strength (estimated)
* Asset value at risk
* Retreat distance
* Reinforcement delay

### Risk Tolerance Levels

* **Cautious:** Avoid loss, accept inefficiency
* **Balanced:** Accept moderate risk
* **Aggressive:** Prioritize objective completion
* **Desperate:** Ignore losses to resolve crisis

Tolerance skews all decisions — it does not add new ones.

---

## 6. Priority Weighting System

Each fleet evaluates choices using weighted priorities.

Example (Mining Fleet):

* Yield: 40%
* Safety: 30%
* Fuel efficiency: 20%
* Political exposure: 10%

Weights are influenced by:

* Player sliders
* Fleet role
* Autonomy tier

Priority imbalance is the primary cause of believable failure.

---

## 7. Decision Types

Fleets choose from a small set of **action classes**:

* Continue operation
* Delay / hold position
* Reroute
* Request support
* Retreat
* Abort task

They do not invent new actions. Complexity comes from *when* these are chosen.

---

## 8. Autonomy Tiers (Behavioral Impact)

Autonomy affects **decision freedom**, not intelligence.

### Tier I — Manual

* Executes orders exactly
* No self-preservation

### Tier II — Assisted

* Requests help
* Will not abandon intent

### Tier III — Autonomous

* Temporarily reprioritizes
* Retreats if justified

### Tier IV — Strategic

* Coordinates with other fleets
* May override player intent

Higher autonomy increases scale — and political risk.

---

## 9. Failure Generation (Intentional)

Failures emerge from:

* Incomplete awareness
* Conflicting priorities
* Over-aggressive tolerance
* Delayed information

There is **no random failure roll**.

Example:
A mining fleet chooses a higher-yield zone based on outdated scout data and underestimates pirate response.

---

## 10. Communication & Reporting

Fleets report decisions and outcomes concisely.

Examples:

* "Mining Fleet Theta delayed extraction due to rising threat estimates. Yield reduced by 18%."
* "Security Fleet Gamma rerouted to respond to station distress. Patrol coverage reduced elsewhere."

Reports explain *reasoning*, not just results.

---

## 11. Player Intervention Points

The player may intervene at **decision boundaries**, not mid-action.

Allowed interventions:

* Change intent
  n- Adjust risk tolerance
* Assign escorts
* Override autonomy

This preserves delegation while maintaining agency.

---

## 12. Scaling Behavior

As the sector grows:

* Fleets act less frequently but with larger consequences
* Mistakes affect multiple systems
* Response windows shorten

AI complexity remains constant; impact scales.

---

## 13. Anti-Frustration Guardrails

* No instant irreversible losses
* Visible warning states
* Multiple valid responses
* Clear causal chains

Players should feel pressure, not confusion.

---

## 14. Design Intent Summary

Fleet AI exists to:

* Enable delegation
* Create believable mistakes
* Generate cascading crises
* Keep the player relevant

The player does not command perfectly.
They **manage imperfection at scale**.

