//! End-to-end test for KiCad symbol integration.

use copperleaf_backend_kicad::{emit_schematic, resolve_symbols};
use copperleaf_core::UnitExt;
use copperleaf_ir::{Block, ComponentInst, Design, Limits, Net, Pin, Role};

#[derive(Clone, Debug)]
struct Rp2354a {
    pins: Vec<Pin>,
}

impl Rp2354a {
    fn new() -> Self {
        Self {
            pins: vec![
                Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                    None,
                ),
                Pin::new(
                    "GND",
                    Role::Gnd,
                    Limits::new(0.0.volt(), 0.0.volt(), 0.1.amp()),
                    None,
                ),
                Pin::new(
                    "GPIO0",
                    Role::DigitalIO,
                    Limits::new(0.0.volt(), 3.6.volt(), 0.05.amp()),
                    None,
                ),
            ],
        }
    }
}

impl Block for Rp2354a {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
    fn kicad_symbol(&self) -> Option<&str> {
        Some("RP2040:RP2354a")
    }
}

#[test]
fn end_to_end_resolve_and_emit_uses_symbol_positions() {
    let mut d = Design::default();
    d.add_net(Net::power("V3V3", 3.3.volt()));
    d.add_net(Net::ground());
    d.add_net(Net::power("LED", 3.3.volt()));
    d.add_component(ComponentInst::new("U1", Rp2354a::new()));
    d.connect("U1", "VDD", "V3V3");
    d.connect("U1", "GND", "GND");
    d.connect("U1", "GPIO0", "LED");

    let lib_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rp2354a.kicad_sym");
    resolve_symbols(&mut d, None, Some(lib_path.to_str().unwrap()));

    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);

    let u1 = d.component_by_refdes("U1").unwrap();
    assert_eq!(u1.pins[0].pos, Some((-15.24, 5.08)));
    assert_eq!(u1.pins[1].pos, Some((-15.24, -5.08)));
    assert_eq!(u1.pins[2].pos, Some((10.16, 0.0)));

    let sch = emit_schematic(&d);
    assert!(sch.contains("(lib_id \"RP2040:RP2354a\")"));
    assert!(sch.contains("(symbol \"RP2040:RP2354a\""));
    assert!(sch.contains("(at -15.24 5.08 0)"));
    assert!(sch.contains("(at -15.24 -5.08 0)"));
    assert!(sch.contains("(at 10.16 0 180)"));

    // U1 is at symbol_position(0) = (25.4, 25.4).
    // KiCad pin (at) is the electrical connection point (tip), not the body end.
    // VDD is a power net: the pin points right (into the body), so the stub
    // extends left away from the body. VDD tip: (25.4 + (-15.24), 25.4 + 5.08)
    // = (10.16, 30.48); stub end = (7.62, 30.48).
    assert!(sch.contains("(at 7.62 30.48 0)"));
    // LED is also declared as a power net in this test, so GPIO0 gets a stub.
    // GPIO0 pin points left (rotation 180, into the body), so the stub extends
    // right away from the body: (25.4 + 10.16 + 2.54, 25.4 + 0.0) = (38.1, 25.4).
    assert!(sch.contains("(at 38.1 25.4 0)"));
}
