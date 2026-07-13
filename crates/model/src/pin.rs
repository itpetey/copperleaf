use crate::units::{Amp, Hertz, Ohm, Qty, Second, UnitExt, Volt};

/// Identifier for a specific pin on a component.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinId(pub String);

/// Electrical role of a pin used to infer ERC rules and routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Absolute electrical limits and nominal voltage for a pin.
#[derive(Clone, Copy, Debug)]
pub struct PowerSpec {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub v_nom: Option<Qty<Volt>>,
    pub i_max: Qty<Amp>,
}

/// Classifies a signal family and integrity expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigKind {
    Generic,
    Usb2Hs,
    Usb3,
    Ddr3,
    PcieGen2,
    Clock,
    AnalogLowNoise,
}

/// Signal integrity specification for a net or pin.
#[derive(Clone, Copy, Debug)]
pub struct SigSpec {
    pub kind: SigKind,
    pub bandwidth: Option<Qty<Hertz>>,
    pub edge_rate: Option<Qty<Second>>,
    pub target_impedance: Option<Qty<Ohm>>,
}

/// A logical pin on a component footprint.
#[derive(Clone, Debug)]
pub struct Pin {
    id: PinId,
    name: String,
    role: Role,
    power_spec: PowerSpec,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    pos: Option<(f64, f64)>,
    rotation: Option<f64>,
    length: Option<f64>,
}

pub struct PinBuilder {
    name: String,
    role: Option<Role>,
    power_spec: Option<PowerSpec>,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    pos: Option<(f64, f64)>,
    rotation: Option<f64>,
    length: Option<f64>,
}

/// Typed reference to a pin name constant on a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinRef(pub &'static str);

/// Handle to a specific pin on a specific component instance.
#[derive(Clone, Copy, Debug)]
pub struct PinHandle {
    pub component: usize,
    pub pin: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawConnection {
    pub(crate) from: PinHandle,
    pub(crate) to: PinHandle,
}

// --- SigSpec impl ---

impl SigSpec {
    pub fn new(
        kind: SigKind,
        bandwidth: Option<Qty<Hertz>>,
        edge_rate: Option<Qty<Second>>,
        target_impedance: Option<Qty<Ohm>>,
    ) -> Self {
        Self {
            kind,
            bandwidth,
            edge_rate,
            target_impedance,
        }
    }

    /// Generic SPI signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// SPI clock signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi_clk(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Clock,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// Generic control signal with no bandwidth or impedance target.
    pub fn control() -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: None,
            edge_rate: None,
            target_impedance: None,
        }
    }

    /// Analog low-noise 50 Ω signal (e.g., RF).
    pub fn rf_50ohm() -> Self {
        Self {
            kind: SigKind::AnalogLowNoise,
            bandwidth: None,
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }
}

// --- Pin impl ---

impl Pin {
    /// Start building a new [`Pin`].
    pub fn build(name: &str) -> PinBuilder {
        PinBuilder::new(name)
    }

    pub fn with_id(mut self, id: PinId) -> Self {
        self.id = id;
        self
    }

    pub fn id(&self) -> &PinId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn power_spec(&self) -> &PowerSpec {
        &self.power_spec
    }
    pub fn decouple(&self) -> bool {
        self.decouple
    }
    pub fn sig_spec(&self) -> Option<SigSpec> {
        self.sig_spec
    }
    pub fn pos(&self) -> Option<(f64, f64)> {
        self.pos
    }
    pub fn rotation(&self) -> Option<f64> {
        self.rotation
    }
    pub fn length(&self) -> Option<f64> {
        self.length
    }
}

// --- PinBuilder impl ---

impl PinBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            role: None,
            power_spec: None,
            decouple: false,
            sig_spec: None,
            pos: None,
            rotation: None,
            length: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn power_spec(mut self, p: PowerSpec) -> Self {
        self.power_spec = Some(p);
        self
    }

    pub fn decouple(mut self, decouple: bool) -> Self {
        self.decouple = decouple;
        self
    }

    pub fn sig_spec(mut self, spec: SigSpec) -> Self {
        self.sig_spec = Some(spec);
        self
    }

    pub fn pos(mut self, x: f64, y: f64) -> Self {
        self.pos = Some((x, y));
        self
    }

    pub fn rotation(mut self, deg: f64) -> Self {
        self.rotation = Some(deg);
        self
    }

    pub fn length(mut self, mm: f64) -> Self {
        self.length = Some(mm);
        self
    }

    /// Fixed-voltage power input: `v_nom = v_min = v_max = v`, `decouple = true`.
    pub fn pwr_fixed(mut self, v: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min: v,
            v_max: v,
            v_nom: Some(v),
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Flexible power input: `v_nom = None`, `decouple = true`.
    pub fn pwr(mut self, v_min: Qty<Volt>, v_max: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min,
            v_max,
            v_nom: None,
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Chainable override to set `v_nom` on a flexible pin.
    pub fn nominal(mut self, v: Qty<Volt>) -> Self {
        if let Some(ref mut p) = self.power_spec {
            p.v_nom = Some(v);
        }
        self
    }

    pub fn digital_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 3.6.volt(),
            v_nom: None,
            i_max: 0.02.amp(),
        });
        self
    }

    pub fn rf_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 1.2.volt(),
            v_nom: None,
            i_max: 1.0.amp(),
        });
        self
    }

    /// Creates a new digital I/O [`Pin`].
    pub fn dio(mut self) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new digital I/O [`Pin`] for SPI.
    pub fn spi(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new digital clock signal [`Pin`].
    pub fn clk(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi_clk(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new ground [`Pin`].
    pub fn gnd(mut self) -> Pin {
        self.role = Some(Role::Gnd);
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 0.0.volt(),
            v_nom: Some(0.0.volt()),
            i_max: 100.0.amp(),
        });
        self.pin()
    }

    /// Creates a new analogue input [`Pin`].
    pub fn analog_in(mut self) -> Pin {
        self.role = Some(Role::AnalogIn);
        self.digital_limits().pin()
    }

    /// Returns a [`Pin`] with the settings configured with this builder.
    ///
    /// # Panics
    ///
    /// This method will panic if `role` or `power_spec` is not set.
    pub fn pin(self) -> Pin {
        Pin {
            id: PinId(String::new()),
            name: self.name,
            role: self.role.unwrap(),
            power_spec: self.power_spec.unwrap(),
            decouple: self.decouple,
            sig_spec: self.sig_spec,
            pos: self.pos,
            rotation: self.rotation,
            length: self.length,
        }
    }
}
