# Pirates & Bosses — Enums, Bases, and Escalation Mechanics (MVP)

This document formalizes **PirateGroup**, **PirateBase**, and **Boss** types, and defines **boss escalation mechanics** (timers, waves, and consequences) aligned with the systemic, run-based design of *XStar Adventures*.

> Pirates are a force you manage. Bosses reshape pressure; they do not remove it.

---

## 1. Pirate System Overview

Pirates operate at three simultaneous layers:

1. **Roaming Pressure** — ambient patrols, harassment, raids
2. **Territorial Control** — pirate-owned zones with elevated baseline pressure
3. **Command Nodes** — pirate bases (with bosses) that amplify and coordinate pressure

Bosses exist as **pressure release valves**, not victory conditions.

---

## 2. Core Enums (Formal Definitions)

### 2.1 Pirate Doctrine

Doctrine describes how a group behaves without branching AI logic everywhere.

```rust
enum PirateDoctrine {
  Harass,      // early: probe, strain, retreat
  Ambush,      // prefers low-visibility routes/zones
  Raid,        // hits logistics/stations, aims for disruption
  Siege,       // commits to a target longer; higher damage over time
}
```

---

### 2.2 Pirate Group

A PirateGroup is a roaming unit that applies pressure.

```rust
enum PirateGroupKind {
  Skiff,       // very weak, early-game
  Pack,        // small coordinated group
  Wing,        // stronger mid-game group
  HunterCell,  // late-game, high coordination
}

struct PirateGroup {
  kind: PirateGroupKind,
  doctrine: PirateDoctrine,
  strength_tier: u8,     // 1..N (time-driven)
  aggression: u8,        // 0..100
  preferred_targets: PirateTargetBias,
}

enum PirateTargetBias {
  Miners,
  FuelDepots,
  MiningOutposts,
  Sensors,
  Opportunistic,
}
```

MVP simplification:

* Treat one PirateGroup as a single entity with a “strength tier” (no per-ship simulation).

---

### 2.3 Pirate Base

A PirateBase is a command node anchored to a zone.

```rust
enum PirateBaseTier {
  Hideout,     // early base
  Stronghold,  // mid base
}

struct PirateBase {
  tier: PirateBaseTier,
  zone_id: ZoneId,
  boss_alive: bool,
  influence_radius: u8,       // number of route hops it pressures
  spawn_budget: u16,          // how many groups it can sustain
  raid_cooldown_secs: u32,
}
```

MVP scope:

* Exactly **one** base in the sector.
* Influence radius: 1–2 hops.

---

### 2.4 Boss

Bosses are leaders of bases and define a unique pressure profile.

```rust
enum BossKind {
  Warlord,      // brute force raids
  Corsair,      // high-mobility ambushes
  Quartermaster // logistics sabotage focus
}

struct Boss {
  kind: BossKind,
  tier: u8,                 // 1..N (sector difficulty)
  enraged: bool,
  notoriety: u8,            // increases when player fails/retreats
}
```

MVP recommendation:

* Start with **one** boss kind (Warlord), keep the others as future variants.

---

## 3. Pirate-Controlled Zones

A zone is considered pirate-controlled when:

* It contains a PirateBase with `boss_alive == true`, or
* Pirate pressure remains above a control threshold for a sustained window

Effects of pirate control:

* Elevated baseline pirate pressure
* Higher interception risk for in-transit fleets
* Increased harassment for any stations built there

Pirate control is **visible on the macro map** as a “red zone.”

---

## 4. Boss Encounter Design (MVP)

### 4.1 Encounter Philosophy

* Boss fights are **optional**
* Bosses do not chase the player across the sector
* The player chooses **when** to engage
* The fight is a **strategic intervention** to reshape pressure

---

### 4.2 Encounter Location

* Boss encounter occurs **inside the pirate base zone**
* PlayerShip must be present to initiate
* Fleet assistance is allowed but fragile

---

## 5. Boss Escalation Mechanics (Timers, Waves, Consequences)

Boss encounters run on a simple, deterministic escalation clock.

### 5.1 Encounter Phases (Time Driven)

**Phase 0 — Approach (0–30s)**

* Player enters base influence area
* Minor harassment; warning state

**Phase 1 — Defense Screen (30–120s)**

* Base spawns **Wave A** repeatedly
* Goal: force the player to commit attention and positioning

**Phase 2 — Boss Emergence (120–180s)**

* Boss enters
* **Wave B** spawns less frequently but stronger

**Phase 3 — Overrun Clock (180s+)**

* Base activates “overrun” behavior:

  * spawn rate increases
  * retreat becomes harder
  * damage pressure accelerates

MVP key:

* No cinematic phases needed
* Escalation is purely time-based and legible

---

### 5.2 Wave Definition (Abstract)

Waves are abstracted into strength units rather than individual ships.

```rust
enum BossWaveKind {
  WaveA, // light defenders
  WaveB, // heavy defenders
}

struct BossWave {
  kind: BossWaveKind,
  strength: u16,
  interval_secs: u32,
}
```

MVP default wave schedule:

* WaveA: strength 25, interval 20s (Phase 1)
* WaveB: strength 60, interval 45s (Phase 2+)

---

### 5.3 Player Failure / Retreat Consequences

Failing a boss fight should not end the run.

Instead it increases systemic pressure:

* `Boss.notoriety += 10`
* Base influence radius temporarily increases (+1 hop for 5–10 minutes)
* Pirate groups adopt a more aggressive doctrine for a window
* Player becomes a known target (higher harassment chance)

Failure is a **new problem**, not a game over.

---

### 5.4 Success Outcomes (Pressure Reshape)

Boss defeat should feel like a relief valve.

On success:

* `PirateBase.boss_alive = false`
* Local pirate pressure drops significantly
* Raid coordination collapses for a cooldown window
* Pressure shifts to other zones (pirates redistribute)

What does NOT happen:

* Pirates do not vanish
* The zone does not become permanently safe
* The run does not immediately end

---

## 6. Post-Boss World State (MVP Rules)

After boss defeat:

* The pirate base becomes inert or “ruined”
* The zone becomes viable for expansion (still risky)
* Pirates continue roaming sector-wide

This creates a clear strategic decision:

> Do I spend time buying relief here, or survive elsewhere?

---

## 7. MVP Spawn & Escalation Plan

### Early Run (0–20 min)

* 1–2 PirateGroups (Skiff / Pack)
* Doctrine: Harass

### Mid Run (20–50 min)

* Additional groups appear
* Doctrine expands: Ambush/Raid depending on zone modifiers and routes

### Late Run (50–90 min)

* Stronger groups appear (Wing/HunterCell)
* Higher coordination and longer patrol routes
* Boss fight becomes increasingly risky due to sector instability

Escalation is time-driven; economy success does not stop it.

---

## 8. UI Requirements (Legibility)

The player must be able to see:

* Zone pirate pressure
* Pirate-controlled zones
* Presence of pirate base
* Whether boss is alive
* Boss notoriety indicator (optional MVP)

Keep UI qualitative:

* icons + one-line descriptions
* avoid numeric overload

---

## 9. Design Intent Summary

Pirates exist to:

* apply time pressure
* punish overextension
* force delegation and escorts

Bosses exist to:

* provide a strategic relief valve
* punctuate the run
* let the player reshape pressure without “winning” the sector

> Boss victories are interventions, not endings.

