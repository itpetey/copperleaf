//! snippets.rs
//!
//! Purpose: a compact, self-contained sketch of the core API shape,
//! traits, structs, and usage. This is *not* production-ready; it’s a scaffold
//! to guide real implementation. Start pulling types into crates as per
//! ARCHITECTURE.md. Replace the minimal units system with `uom` ASAP.

use std::marker::PhantomData;

// ========== 0) Minimal Units & Extensions (replace with `uom`) ==========

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Qty<U>(pub f64, pub PhantomData<U>);

macro_rules! unit {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $name;
    };
}

unit!(Volt);
unit!(Amp);
unit!(Ohm);
unit!(Henry);
unit!(Farad);
unit!(Meter);
unit!(Second);
unit!(Celsius);

pub trait UnitExt {
    fn volt(self) -> Qty<Volt>;
    fn amp(self) -> Qty<Amp>;
    fn ohm(self) -> Qty<Ohm>;
    fn henry(self) -> Qty<Henry>;
    fn farad(self) -> Qty<Farad>;
    fn mm(self) -> Qty<Meter>;
    fn meter(self) -> Qty<Meter>;
    fn sec(self) -> Qty<Second>;
    fn celsius(self) -> Qty<Celsius>;
    // convenience
    fn mhz(self) -> Qty<Second>;
    fn nf(self) -> Qty<Farad>;
    fn uf(self) -> Qty<Farad>;
    fn pf(self) -> Qty<Farad>;
    fn kohm(self) -> Qty<Ohm>;
    fn millivolt(self) -> Qty<Volt>;
}
impl UnitExt for f64 {
    fn volt(self) -> Qty<Volt> {
        Qty(self, PhantomData)
    }
    fn amp(self) -> Qty<Amp> {
        Qty(self, PhantomData)
    }
    fn ohm(self) -> Qty<Ohm> {
        Qty(self, PhantomData)
    }
    fn henry(self) -> Qty<Henry> {
        Qty(self, PhantomData)
    }
    fn farad(self) -> Qty<Farad> {
        Qty(self, PhantomData)
    }
    fn mm(self) -> Qty<Meter> {
        Qty(self / 1000.0, PhantomData)
    }
    fn meter(self) -> Qty<Meter> {
        Qty(self, PhantomData)
    }
    fn sec(self) -> Qty<Second> {
        Qty(self, PhantomData)
    }
    fn celsius(self) -> Qty<Celsius> {
        Qty(self, PhantomData)
    }
    fn mhz(self) -> Qty<Second> {
        Qty(1.0 / (self * 1.0e6), PhantomData)
    }
    fn nf(self) -> Qty<Farad> {
        Qty(self * 1.0e-9, PhantomData)
    }
    fn uf(self) -> Qty<Farad> {
        Qty(self * 1.0e-6, PhantomData)
    }
    fn pf(self) -> Qty<Farad> {
        Qty(self * 1.0e-12, PhantomData)
    }
    fn kohm(self) -> Qty<Ohm> {
        Qty(self * 1.0e3, PhantomData)
    }
    fn millivolt(self) -> Qty<Volt> {
        Qty(self * 1.0e-3, PhantomData)
    }
}

// ========== 1) Core Domain Types ==========

#[derive(Clone, Copy, Debug)]
pub enum Role {
    PowerIn,
    PowerOut,
    AnalogIn,
    AnalogOut,
    DigitalIO,
    DiffPos,
    DiffNeg,
    Gnd,
}

#[derive(Clone, Copy, Debug)]
pub enum SigKind {
    Generic,
    Usb2Hs,
    Usb3,
    Ddr3,
    PcieGen2,
    Clock,
    AnalogLowNoise,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub i_max: Qty<Amp>,
}

#[derive(Clone, Copy, Debug)]
pub struct SigSpec {
    pub kind: SigKind,
    pub bandwidth: Option<Qty<Second>>, // as period (1/f) for simplicity
    pub edge_rate: Option<Qty<Second>>,
    pub target_impedance: Option<Qty<Ohm>>,
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub name: &'static str,
    pub role: Role,
    pub limits: Limits,
    pub sig: Option<SigSpec>,
}

#[derive(Clone, Debug)]
pub enum NetKind {
    Power {
        v_nom: Qty<Volt>,
        ripple: Option<Qty<Volt>>,
    },
    Signal {
        spec: SigSpec,
    },
}

#[derive(Clone, Debug, Default)]
pub struct NetClass {
    pub min_width: Option<Qty<Meter>>,
    pub clearance: Option<Qty<Meter>>,
}

#[derive(Clone, Debug)]
pub struct Net {
    pub name: String,
    pub kind: NetKind,
    pub class: NetClass,
    pub constraints: Vec<Constraint>,
}

impl Net {
    pub fn power(name: &str, v_nom: Qty<Volt>) -> Self {
        Self {
            name: name.to_string(),
            kind: NetKind::Power {
                v_nom,
                ripple: None,
            },
            class: NetClass::default(),
            constraints: vec![],
        }
    }
    pub fn ground() -> Self {
        Self::power("GND", 0.0.volt())
    }
    pub fn ripple(mut self, r: Qty<Volt>) -> Self {
        if let NetKind::Power { v_nom, .. } = self.kind {
            self.kind = NetKind::Power {
                v_nom,
                ripple: Some(r),
            };
        }
        self
    }
}

#[derive(Clone, Debug)]
pub struct Params; // placeholder

// ========== 2) Constraints ==========

#[derive(Clone, Debug)]
pub enum Constraint {
    Impedance {
        target: Qty<Ohm>,
        tol_pct: f64,
    },
    LengthMatch {
        group: String,
        skew_ps: f64,
    },
    ReturnPath {
        requires_plane: bool,
    }, // simplified
    NetClass {
        min_width: Qty<Meter>,
        clearance: Qty<Meter>,
    },
    Creepage {
        min: Qty<Meter>,
        voltage: Qty<Volt>,
    },
    Decoupling {
        values: Vec<Qty<Farad>>,
        per_pin: bool,
    },
    ResonanceIndex {
        max: f64,
    },
    MaxJunction {
        temp: Qty<Celsius>,
    },
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub entities: Vec<String>, // ids of pins/nets/components/etc.
    pub hint: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

// ========== 3) Components & Blocks ==========

pub trait Block {
    fn id(&self) -> &str;
    fn pins(&self) -> &[Pin];
    fn constraints(&self) -> Vec<Constraint> {
        vec![]
    }
}

#[derive(Clone, Debug)]
pub struct ComponentInst<B: Block> {
    pub refdes: String,
    pub block: B,
}

impl<B: Block> ComponentInst<B> {
    pub fn new(refdes: &str, block: B) -> Self {
        Self {
            refdes: refdes.to_owned(),
            block,
        }
    }
}

// Example part: a super-simplified buck regulator “block”
pub mod parts {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Buck {
        id: String,
        pins: Vec<Pin>,
        pub v_out: Qty<Volt>,
        pub i_max: Qty<Amp>,
    }

    impl Buck {
        pub fn new(id: &str, v_out: Qty<Volt>, i_max: Qty<Amp>) -> Self {
            Self {
                id: id.to_owned(),
                v_out,
                i_max,
                pins: vec![
                    Pin {
                        name: "VIN",
                        role: Role::PowerIn,
                        limits: Limits {
                            v_min: 3.0.volt(),
                            v_max: 24.0.volt(),
                            i_max: 3.0.amp(),
                        },
                        sig: None,
                    },
                    Pin {
                        name: "SW",
                        role: Role::PowerOut,
                        limits: Limits {
                            v_min: 0.0.volt(),
                            v_max: 24.0.volt(),
                            i_max: 3.0.amp(),
                        },
                        sig: None,
                    },
                    Pin {
                        name: "GND",
                        role: Role::Gnd,
                        limits: Limits {
                            v_min: 0.0.volt(),
                            v_max: 0.0.volt(),
                            i_max: 100.0.amp(),
                        },
                        sig: None,
                    },
                ],
            }
        }
    }
    impl Block for Buck {
        fn id(&self) -> &str {
            &self.id
        }
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
        fn constraints(&self) -> Vec<Constraint> {
            vec![Constraint::Decoupling {
                values: vec![100.0.nf(), 1.0.uf()],
                per_pin: true,
            }]
        }
    }

    // Example MCU with USB pins (highly simplified)
    #[derive(Clone, Debug)]
    pub struct Mcu {
        id: String,
        pins: Vec<Pin>,
    }
    impl Mcu {
        pub fn new(id: &str) -> Self {
            let usb_spec = SigSpec {
                kind: SigKind::Usb2Hs,
                bandwidth: Some(480.0.mhz()),
                edge_rate: None,
                target_impedance: Some(90.0.ohm()),
            };
            Self {
                id: id.to_owned(),
                pins: vec![
                    Pin {
                        name: "VDD",
                        role: Role::PowerIn,
                        limits: Limits {
                            v_min: 1.7.volt(),
                            v_max: 3.6.volt(),
                            i_max: 0.5.amp(),
                        },
                        sig: None,
                    },
                    Pin {
                        name: "VSS",
                        role: Role::Gnd,
                        limits: Limits {
                            v_min: 0.0.volt(),
                            v_max: 0.0.volt(),
                            i_max: 100.0.amp(),
                        },
                        sig: None,
                    },
                    Pin {
                        name: "USB_DP",
                        role: Role::DiffPos,
                        limits: Limits {
                            v_min: 0.0.volt(),
                            v_max: 3.6.volt(),
                            i_max: 0.05.amp(),
                        },
                        sig: Some(usb_spec),
                    },
                    Pin {
                        name: "USB_DM",
                        role: Role::DiffNeg,
                        limits: Limits {
                            v_min: 0.0.volt(),
                            v_max: 3.6.volt(),
                            i_max: 0.05.amp(),
                        },
                        sig: Some(usb_spec),
                    },
                ],
            }
        }
    }
    impl Block for Mcu {
        fn id(&self) -> &str {
            &self.id
        }
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
        fn constraints(&self) -> Vec<Constraint> {
            vec![
                Constraint::Impedance {
                    target: 90.0.ohm(),
                    tol_pct: 10.0,
                },
                Constraint::LengthMatch {
                    group: "USB_D".into(),
                    skew_ps: 200.0,
                },
            ]
        }
    }
}

// ========== 4) Design, Connect, Verify, Export (Sketch) ==========

#[derive(Default)]
pub struct Design {
    pub nets: Vec<Net>,
    pub components: Vec<String>, // refdes
    pub constraints: Vec<Constraint>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Design {
    pub fn add_net(&mut self, n: Net) {
        self.nets.push(n);
    }
    pub fn add_component<B: Block>(&mut self, inst: &ComponentInst<B>) {
        self.components.push(inst.refdes.clone());
        // Normally: index pins, populate graph, inherit constraints, etc.
    }
    pub fn add_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }
}

// Super-simple ERC example
pub fn erc_voltage_pin_to_net(net: &Net, pin: &Pin) -> Option<Diagnostic> {
    match net.kind {
        NetKind::Power { v_nom, .. } => {
            if v_nom.0 > pin.limits.v_max.0 + 1e-9 {
                return Some(Diagnostic {
                    code: "ERC:OVERVOLT",
                    severity: Severity::Error,
                    message: format!(
                        "Pin {} max {:.2}V, connected to {:.2}V net",
                        pin.name, pin.limits.v_max.0, v_nom.0
                    ),
                    entities: vec![pin.name.into(), net.name.clone()],
                    hint: Some("Use a level shifter or different pin".into()),
                });
            }
            None
        }
        _ => None,
    }
}

// ========== 5) Macro Shapes (stubs) ==========
// The real project should move these into `edsl` with proc_macros.
// Below are signatures to guide implementation.

/// design!("Name", |d| { ... })
#[macro_export]
macro_rules! design {
    ($name:literal, |$d:ident| $body:block) => {{
        // allocate a Design, pass to closure, return it
        let mut $d = Design::default();
        $body
        $d
    }};
}

/// connect! { A -> net via decouple([...]); ... }
#[macro_export]
macro_rules! connect {
    ($($tt:tt)*) => {
        // parser omitted; should build graph edges and attach constraints
        compile_error!("connect! macro is a stub in snippets.rs");
    };
}

/// verify! { erc: [...]; drc: [...]; ... }
#[macro_export]
macro_rules! verify {
    ($($tt:tt)*) => {
        // dispatch to passes; collect diagnostics
        compile_error!("verify! macro is a stub in snippets.rs");
    };
}

/// export! { kicad_project("..."); bom_csv("..."); ... }
#[macro_export]
macro_rules! export {
    ($($tt:tt)*) => {
        // call selected backends
        compile_error!("export! macro is a stub in snippets.rs");
    };
}

/*
Next steps (turn this scaffold into crates):

1) Replace the ad-hoc units with `uom` and implement `serde` for IR.
2) Introduce stable IDs (newtypes) for pins/nets/components to support diffs.
3) Add a DesignGraph (e.g., petgraph) to represent electrical connections; index pins.
4) Implement Constraint Registry: type → checker (pre/post) → synthesizer → backend hints.
5) Build KiCad backend emitter from IR; start with schematic netlist & footprint refs.
6) Add PDN resonance index pass (loop-L estimation + Ceq/ESR), then USB diff-pair rules.
7) Define JSON IR schema + Patch protocol; implement validate/apply with conflict checks.
*/
