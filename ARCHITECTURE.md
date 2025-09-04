# Project Copperleaf (working name): A Programmable, Typed EDA Framework in Rust

> **Tagline:** A typed, constraint-driven Rust library for schematic capture, PCB design, and first-order analysis—optimized for software engineers and AI copilots.

---

## 0. Goals & Non-goals

**Goals**
- Rust-first EDSL that feels like a library, not a DSL: “script your hardware.”
- Strong types and units to encode electrical/physical constraints and catch errors early.
- A first-class **constraint system** powering ERC/DRC/SI/PI/Thermal checks and synthesis (e.g., auto-decoupling).
- Clear, stable **IR** that’s introspectable for AI; patch/diff protocol for propose/fix loops.
- Pragmatic backends: KiCad (schematic + PCB), SPICE netlists, BOM/AVL, IPC‑2581, Gerbers.
- Works incrementally: pre-layout checks, then post-layout checks with stackup and geometry.

**Non-goals (v1)**
- Full-wave EM or 3D field solvers (we provide *heuristics* and export to tools that do).
- Proprietary EDA UI—this is a library/CLI-first tool; integrate with KiCad & friends.
- Infinite part coverage. Start with a crisp part-format and a small, high-quality stdlib.

---

## 1. High-level Architecture

```
      +-----------------------------+
      |           Frontends         |
      |  - Rust EDSL (primary)      |
      |  - Optional spec (TOML)     |
      +--------------+--------------+
                     |
                     v
+------------------------------------------------------+
|                      Core IR                         |
|  - Electrical Graph (components, pins, nets)         |
|  - Geometry (board, footprints, routes, keepouts)    |
|  - Constraints (typed, checkable, satisfiable)       |
|  - Metadata (stackup, materials, env, standards)     |
+---------------+---------------+----------------------+
                |               |
                v               v
     +----------------+   +----------------+
     |  Rule Engine   |   |   Solvers      |
     |  - Constraint  |   |  - ERC/DRC     |
     |    registry    |   |  - SI/PI       |
     |  - Synthesis   |   |  - Thermal     |
     +--------+-------+   +--------+-------+
              |                       |
              v                       v
      +----------------+       +----------------+
      | Diagnostics    |       |   Proposals    |
      | (errors/warns) |       | (structured)   |
      +----------------+       +----------------+
                     \           /
                      \         /
                       v       v
                      +---------+
                      | Backends|
                      +---------+
      KiCad, SPICE, IPC‑2581, Gerber/Excellon, BOM/AVL, JSON IR
```

---

## 2. Crate/Module Layout

```
/crates
  /core           // units, quantities, ids, diagnostics, error types
  /ir             // pins, nets, components, board, stackup, constraints
  /analysis       // ERC, DRC, SI/PI, Thermal, synthesis passes
  /layout         // geometry primitives, netclasses, routing constraints
  /backends
    /kicad        // .kicad_sch, .kicad_pcb emitters
    /spice
    /ipc2581
    /gerber
  /edsl           // ergonomic builders, macros, derive(Component)
  /parts          // standard library of parts & patterns
  /cli            // command-line entrypoint (verify/export), JSON IR i/o
```

> **Build order:** `core` → `ir` → `layout` → `analysis` → backends → `edsl` → `parts` → `cli`.

---

## 3. Core Concepts

### 3.1 Quantities & Units (compile-time guardrails)
- All domain values carry **dimensions** (V, A, Ω, H, F, m, s, °C).
- Prefer the community crate `uom`; a minimal internal system can bootstrap v0.
- Provide ergonomic extensions: `3.3.volt()`, `50.milliohm()`, `0.2.mm()`.

### 3.2 IDs and Versioning
- Stable IDs for components, nets, constraints, footprints to allow **diffs**.
- Part definitions are versioned (semantic version + source).

### 3.3 Electrical IR
- `ComponentDef` (library data) vs `ComponentInst` (placed in a design).
- `Pin` carries **role** (PowerIn/Out, AnalogIn/Out, DigitalIO, DiffPos/Neg, Gnd),
  limits (Vmax/Imax), **signal spec** (bandwidth, edge rate, target impedance),
  and **return-path expectations**.
- `Net` holds **kind**: `Power{ v_nom, ripple }` or `Signal{ spec }`,
  **class** for width/clearance, and **constraints**.

### 3.4 Geometry & Board IR
- `Board` (outline, stackup), `Footprint`, `Keepout`, `RouteSegment`, `Via`.
- Stackup with dielectric, copper weight, controlled-impedance intent.

### 3.5 Constraints (first-class)
Constraints are typed records attached to IR entities:
- Electrical (e.g., `Impedance{ target, tol }`, `LengthMatch{ group, skew_ps }`)
- Safety (e.g., `Creepage{ min, voltage_class }`)
- Manufacturing (e.g., `Clearance{ min_by_class }`, `MinAnnularRing`)
- PDN (e.g., `Decoupling{ per_pin_caps, density }`, `ResonanceIndex{ max }`)
- Thermal (e.g., `MaxJunction{ temp }`)

**Constraint Registry** maps types → checkers → synthesizers → emitters.

---

## 4. Analysis Passes

### 4.1 ERC (Pre-layout)
- Pin-role mismatches, over/undervoltage, logic-level mismatches.
- Pull-ups/pull-downs presence on open-drain/open-collector nets.
- Power/current budget per rail.

### 4.2 PI (Pre & Post)
- **Resonance Risk** for each rail: estimate loop L (package + vias + loop area),
  combine with `C_eff` and ESR to compute `f0 = 1/(2π√(L·C))`.
- Flag if `f0` lies near strong sources (buck switching freq, clock harmonics).
- Propose damping (ESR selection), value spread, and placement tightening.

### 4.3 SI (Post-layout heavy, Pre-layout light)
- In pre-layout: edge-rate/trace length sanity, required controlled impedance.
- In post-layout: track widths, diff spacing/length-match, return-path continuity
  (plane splits), via stubs guidance.

### 4.4 DRC & HV
- Netclass width/clearance, creepage per IPC-2221 class, hole sizes, mask slivers.

### 4.5 Thermal
- ΘJA estimate from package, copper pour area, via arrays. Warn if TJ exceeds bounds.

All passes produce **Diagnostics** with severity (Info/Warning/Error) + **Actionables**
(structured proposals).

---

## 5. Synthesis

- **Auto-decoupling**: per-rail/per-pin caps using part rules and app-note recipes.
- **Derived netclasses**: compute widths from current + temp-rise targets.
- **Guard features**: keepouts near antennas, stitching vias around high di/dt loops.

Synthesis emits edits as **Patches** (see §7).

---

## 6. Backends

- **KiCad**: schematic/pcb with footprints, netclasses, constraints, keepouts.
- **SPICE**: netlists for PDN/small-signal checks.
- **IPC‑2581/Gerber**: manufacturing export (v2).
- **BOM/AVL**: CSV/JSON with supplier links and alternates.

---

## 7. AI Hooks

### 7.1 JSON IR
- Canonical, versioned schema: components, nets, constraints, geometry, stackup,
  diagnostics. Designed for **round-trip** (load → modify → apply patch).

### 7.2 Patch Protocol
```json
{
  "patch_id": "p_017",
  "ops": [
    {"op":"add_component","ref":"C43","def":"C_0402_100n","at":{"x":34.2,"y":17.9,"rot":0}},
    {"op":"connect","net":"VDD","pins":["U3.VDD_1","C43.1"]},
    {"op":"constraint","target":"net:USB_D","type":"Impedance","args":{"z_ohm":90,"tol_pct":10}}
  ]
}
```
- Engine validates and applies patches; conflicts become diagnostics.

---

## 8. Part Library Format

- Rust-first `#[derive(Component)]` with metadata (symbol, footprint, pins) and
  **embedded constraints** (e.g., decoupling rules, interface rules).
- Companion `parts.toml` for parametric selection (voltage/current, ESR, package).

---

## 9. Diagnostics & DevEx

- Unified `Diagnostic` type: code, message, severity, span (entity ids), hints, fix.
- `--strict` mode treats Warning as Error in CI.
- Snapshot tests for passes; golden-file tests for backends and JSON IR.

---

## 10. Milestones

**P0 (MVP)**
- Units + core IR + diagnostics.
- ERC basics; decoupling synthesis.
- Netlist (KiCad schematic + SPICE).
- EDSL builders and 2–3 parts (MCU, buck, sensor, USB).

**P1**
- Board/stackup model; DRC basics; KiCad PCB export (footprints + keepouts).
- PI resonance index; USB2 diff-pair rules.

**P2**
- Thermal pass; PDN droop calc; BOM export.
- Patch protocol; JSON IR round-trip.

**P3**
- IPC‑2581/Gerber; DDR/PCIe rules; autorouter hooks.

---

## 11. Guiding Principles

- **Strong types, generous ergonomics.** Default everything safely; make the right thing easy.
- **Transparent math.** Every heuristic has visible assumptions overridable by the user.
- **Composable.** Parts carry their own constraints; designs combine them.
- **Deterministic.** Version everything; make builds reproducible.
