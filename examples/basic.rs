use copperleaf::{
    erc_voltage_pin_to_net, parts, ComponentInst, Constraint, Design, Limits, Net, NetClass, Pin,
    Role, UnitExt,
};

fn main() {
    // Rails
    let vbus = Net::power("VBUS", 5.0.volt());
    let gnd = Net::ground();
    let mut v3v3 = Net::power("V3V3", 3.3.volt()).ripple(50.0.millivolt());
    v3v3.class = NetClass {
        min_width: Some(0.3.mm()),
        clearance: Some(0.2.mm()),
    };
    v3v3.constraints.push(Constraint::NetClass {
        min_width: 0.3.mm(),
        clearance: 0.2.mm(),
    });

    // Blocks
    let buck = parts::Buck::new("MPM3610", 3.3.volt(), 2.0.amp());
    let u_reg = ComponentInst::new("U1", buck);

    let mcu = parts::Mcu::new("STM32F405RG");
    let u_mcu = ComponentInst::new("U2", mcu);

    // Design assembly (graph wiring omitted in this sketch)
    let mut d = Design::default();
    d.add_net(vbus);
    d.add_net(gnd);
    d.add_net(v3v3);
    d.add_component(&u_reg);
    d.add_component(&u_mcu);

    // Pre-layout ERC example (fake hookup for demo)
    let vdd_pin = Pin {
        name: "VDD",
        role: Role::PowerIn,
        limits: Limits {
            v_min: 1.7.volt(),
            v_max: 3.6.volt(),
            i_max: 0.5.amp(),
        },
        sig: None,
    };
    let v3v3 = &d.nets.iter().find(|n| n.name == "V3V3").unwrap();
    if let Some(diag) = erc_voltage_pin_to_net(v3v3, &vdd_pin) {
        println!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
    }

    // Add global constraints (project targets)
    d.add_constraint(Constraint::ResonanceIndex { max: 0.5 });
    d.add_constraint(Constraint::MaxJunction {
        temp: 85.0.celsius(),
    });

    // Export would serialize `d` to JSON IR / KiCad in real backends.
}
