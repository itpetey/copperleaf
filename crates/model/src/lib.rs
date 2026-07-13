use std::collections::HashMap;

use petgraph::graph::UnGraph;
use uuid::Uuid;

use crate::units::{Amp, Qty, UnitExt, Volt};

pub mod units;

pub type PinId = Uuid;

/// Represents a single part (e.g. a resistor, chip, etc.) on a PCB
pub trait Component {
    /// Retrieves a [`Pin`] from this [`Component`], if it exists
    fn pin(&self, id: PinId) -> Option<&Pin>;

    /// Retrieves a [`Pin`] by its name from this [`Component`], if it exists
    fn pin_name(&self, name: &str) -> Option<&Pin>;

    /// Retrieves all [`Pin`]s attached to the [`Component`]
    fn pins(&self) -> &[Pin];
}

/// Top level structure representing the PCB being designed
///
/// Your project should have 1 or more [`Board`]s.
pub struct Board {
    components: HashMap<String, Box<dyn Component>>,
    connections: UnGraph<PinId, ()>,
}

#[derive(Clone, Debug)]
pub struct Pin {
    id: PinId,
    name: String,
    role: Role,
    power_limit: PowerLimit,
    decouple: bool,
}

pub struct PinBuilder {
    name: String,
    role: Option<Role>,
    power_limit: Option<PowerLimit>,
    decouple: bool,
}

#[derive(Clone, Debug)]
pub struct PowerLimit {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub i_max: Qty<Amp>,
}

/// Electrical role of a pin used to infer ERC rules and routing
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

/// Quick reference to a [`Component`]'s [`Pin`]
///
/// This is useful when connecting two board pins together. The format *must be* "<component_name>.<pin_name>".
pub struct ComponentPin<'a> {
    component: &'a str,
    pin: &'a str,
}

impl Board {
    /// Creates a new, unpopulated [`Board`]
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            connections: UnGraph::new_undirected(),
        }
    }

    /// Add a [`Component`] to this board
    pub fn add<C: Component + 'static>(&mut self, name: &str, component: C) {
        self.components.insert(name.to_owned(), Box::new(component));
    }

    /// Connect one [`Pin`] to another using `ComponentPin` identifiers
    ///
    /// This is a virtual wire between two points that will be resolved into a real PCB
    /// connection by the backend.
    ///
    /// # Errors
    ///
    /// Returns an error if either `Pin` cannot be found on this board. If a connection
    /// already exists, this method does nothing and returns `Ok`.
    ///
    /// # Panics
    ///
    /// Panics if either `from` or `to` are not valid `ComponentPin`s.
    pub fn connect<'a, 'b>(
        &mut self,
        from: impl Into<ComponentPin<'a>>,
        to: impl Into<ComponentPin<'b>>,
    ) -> Result<(), ()> {
        let from = from.into();
        let to = to.into();

        todo!()
    }
}

impl Pin {
    /// Create a new [`Pin`]
    pub fn build(name: &str) -> PinBuilder {
        PinBuilder::new(name)
    }

    fn new(name: &str, role: Role, power_limit: PowerLimit, decouple: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            role,
            power_limit,
            decouple,
        }
    }

    pub fn id(&self) -> PinId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PinBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            role: None,
            power_limit: None,
            decouple: false,
        }
    }

    /// Set the [`Pin`]'s [`Role`]
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Set power limits for digital I/O pins (e.g. CLK, GPIO, etc.)
    pub fn digital_limits(mut self) -> Self {
        self.power_limit = Some(PowerLimit {
            v_min: 0.0.volt(),
            v_max: 3.6.volt(),
            i_max: 0.02.amp(),
        });

        self
    }

    /// Set power limits for the analogue / RF antenna pin
    pub fn rf_limits(mut self) -> Self {
        self.power_limit = Some(PowerLimit {
            v_min: 0.0.volt(),
            v_max: 1.2.volt(),
            i_max: 1.0.amp(),
        });

        self
    }

    /// Set whether this [`Pin`] should add decoupling capacitors
    pub fn decouple(mut self, decouple: bool) -> Self {
        self.decouple = decouple;
        self
    }

    /// Creates a new digital I/O [`Pin`]
    pub fn dio(mut self) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new digital I/O [`Pin`] for SPI
    pub fn spi(mut self) -> Pin {
        // Some(SigSpec::spi(50.0))
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new digital clock signal [`Pin`]
    pub fn clk(mut self) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new ground [`Pin`]
    pub fn gnd(mut self) -> Pin {
        self.role = Some(Role::Gnd);
        self.power_limit = Some(PowerLimit {
            v_min: 0.0.volt(),
            v_max: 0.0.volt(),
            i_max: 100.0.amp(),
        });
        self.pin()
    }

    /// Create a new power supply [`Pin`]
    pub fn pwr(mut self, v_min: Qty<Volt>, v_max: Qty<Volt>, i_max: Qty<Amp>) -> Pin {
        self.role = Some(Role::PowerIn);
        self.power_limit = Some(PowerLimit {
            v_min,
            v_max,
            i_max,
        });
        self.decouple = true;
        self.pin()
    }

    /// Returns a [`Pin`] with the settings configured with this builder.
    ///
    /// # Panics
    ///
    /// This method will panic if `role` or `power_limit` is not set.
    pub fn pin(self) -> Pin {
        Pin {
            id: Uuid::new_v4(),
            name: self.name,
            role: self.role.unwrap(),
            power_limit: self.power_limit.unwrap(),
            decouple: self.decouple,
        }
    }
}

impl<'a> From<&'a str> for ComponentPin<'a> {
    fn from(value: &'a str) -> Self {
        let mut split = value.split('.');
        Self {
            component: split.nth(0).expect("valid `ComponentPin` format"),
            pin: split.nth(1).expect("valid `ComponentPin` format"),
        }
    }
}
