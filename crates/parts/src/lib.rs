use copperleaf_model::units::{Farad, Hertz, Ohm};

pub struct Capacitor {
    capacity: Farad,
    package: Package,
}

pub struct Crystal {
    frequency: Hertz,
    package: Package,
}

pub struct Resistor {
    value: Ohm,
    package: Package,
}

pub enum Package {
    ThroughHole,
    SMD0201,
    SMD0402,
    SMD0603,
    SMD1005,
    SMD1608,
    SMD2012,
    SMD2520,
    SMD3216,
    SMD3225,
    SMD4516,
    SMD4532,
    SMD5025,
    SMD6332,
}

impl Capacitor {
    pub fn new(capacity: Farad, package: Package) -> Self {
        Self { capacity, package }
    }
}

impl Crystal {
    pub fn new(frequency: Hertz, package: Package) -> Self {
        Self { frequency, package }
    }
}

impl Resistor {
    pub fn new(value: Ohm, package: Package) -> Self {
        Self { value, package }
    }
}
