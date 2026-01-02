# XStar Adventures — MVP One-Page Spec

## Player Fantasy

You are a lone frontier captain who begins by personally scouting, mining, and defending fragile infrastructure. Over time, you transition into a sector commander who delegates work to fleets while responding to escalating crises that automation alone cannot solve.

---

## Core Loop

**Scout → Mine → Build Station → Pressure Emerges → Delegate Fleets → Problems Escalate → Player Intervenes → World Changes**

The loop is systemic and reactive. There are no explicit missions; progress and conflict emerge from player actions and world response.

---

## Vertical Slice Scope

* **World:** One procedurally generated sector (5–7 star systems)
* **Playtime Target:** ~90 minutes per run
* **Structure:** Open sandbox, finite but replayable

---

## Included Systems (MVP Required)

### Player Ship

* Top-down 2D movement
* Fuel consumption
* Manual mining
* Scanning
* Lightweight combat

### Resources

* **Common Ore:** Used for all basic construction
* **Advanced Ore:** Required for higher-tier fleets or stations

### Stations

* **Basic Outpost:** Produces ore, requires fuel, vulnerable to attack
* **Fuel Depot:** Extends operational range and supports fleets

### Fleets

* **Scout Fleet:** Reveals systems and pirate activity
* **Mining Fleet:** Automates ore extraction
* **Security Fleet:** Defends stations and responds to pirate pressure

Each fleet supports:

* Assigned intent
* Risk tolerance slider
* Imperfect execution and failure

### Delegation UI

* Galaxy map with status indicators
* Fleet panel (intent + risk)
* Station panel (output + fuel state)
* Problems feed (no alert spam)

### Pirates

* Roaming patrols that escalate over time
* Pressure zones that stress logistics and stations
* One pirate base with a boss encounter

### Crisis System (Lightweight)

* Fuel shortages
* Pirate harassment
* Clear escalation states: **Stable → Strained → Failing**

---

## Explicitly Out of Scope (For MVP)

* Factions and politics
* Logistics fleets
* Advanced AI learning
* Crew personalities
* Exotic or relic ores
* Multiple regions or sectors
* Narrative endings
* Visual polish and cinematic presentation

---

## Success Criteria

The MVP is successful if players:

* Understand the delegation-first fantasy
* Feel constant but fair pressure
* Personally intervene during crises
* Accept losses as meaningful consequences
* Want to see the system scale outward

---

## Design Pillars

* **Delegation over micromanagement**
* **Pressure over missions**
* **Failure creates new gameplay**
* **The player ship remains relevant through flexibility, not raw power**

---

## Core Question (Gate Check)

If everything else were cut, would this slice still be compelling to play for two hours?

If the answer is no, the scope must be reduced further.

