//! Emit the representative W5500 project used by the external KiCad
//! acceptance harness.
//!
//! This is deliberately a real board fixture rather than a synthetic one: it
//! exercises repeated pin names, a plane pour, a mechanical pad, a back-side
//! placement, tracks, and a via. The acceptance script loads the emitted
//! files with a pinned `kicad-cli` and compares ERC, DRC, and
//! schematic-parity JSON against a reviewed baseline.
//!
//! ```sh
//! cargo run --locked -p copperleaf-backend-kicad --example w5500_acceptance -- target/acceptance
//! ```

use std::{collections::BTreeMap, path::PathBuf};

use copperleaf::{
    Backend, Board, BoardSide, ComponentHandle, CopperLayer, Layout, PinRef, Placement, Role,
    Track, Via, Zone, units::UnitExt,
};
use copperleaf_backend_kicad::KiCad;

const PROJECT: &str = "w5500";

/// Tie ground pins together and group power pins by nominal voltage, leaving
/// signal and NC pins unconnected so ERC findings are visible in the baseline.
fn auto_wire(board: &mut Board, handle: ComponentHandle) {
    let mut groups: BTreeMap<(bool, i64), Vec<&'static str>> = BTreeMap::new();
    let pins = board.components[handle.0].component.pins();
    let identities = copperleaf::resolve_pin_identities(&board.components[handle.0].name, pins);
    for (pin, canonical) in pins.iter().zip(identities.pins.iter()) {
        if pin.nc() || pin.name() == "NC" || pin.name().starts_with("NC_") {
            continue;
        }
        let key = match pin.role() {
            Role::Gnd => (true, 0),
            Role::PowerIn | Role::PowerOut => {
                let Some(v) = pin.power_spec().v_nom else {
                    continue;
                };
                (false, (v.as_base() * 1000.0).round() as i64)
            }
            _ => continue,
        };
        let identity: &'static str = Box::leak(canonical.key.clone().into_boxed_str());
        groups.entry(key).or_default().push(identity);
    }
    for names in groups.values() {
        for pair in names.windows(2) {
            board
                .connect(handle.pin(PinRef(pair[0])), handle.pin(PinRef(pair[1])))
                .expect("auto-wire connect");
        }
    }
}

fn main() {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/acceptance".to_string()),
    );
    std::fs::create_dir_all(&out).expect("create output directory");

    let mut board = Board::new(PROJECT);
    let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
    let j1 = board.add("J1", copperleaf_parts_connectors::Bh123a::new());
    let r1 = board.add(
        "R1",
        copperleaf_parts_passives::Resistor::new(
            33.0.kohm(),
            copperleaf_parts_passives::footprint::Package::M1608,
        ),
    );
    for handle in [u1, j1] {
        auto_wire(&mut board, handle);
    }

    // Ground: W5500 pad 29 to the connector return, as an explicit plane net.
    let gnd = board
        .connect(u1.pin(PinRef("29")), j1.pin(PinRef("N")))
        .expect("ground connection");
    board.set_net_name(gnd, "GND");
    board.set_net_voltage(gnd, 0.0.volt());

    // Repeated AVDD pins are connected independently as one power net.
    let v3v3 = board
        .connect(u1.pin(PinRef("4")), u1.pin(PinRef("8")))
        .expect("AVDD connection");
    board.set_net_name(v3v3, "V3V3");
    board.set_net_voltage(v3v3, 3.3.volt());

    // A signal track crosses the board to a back-side series resistor.
    let sclk = board
        .connect(u1.pin(PinRef("33")), r1.pin(PinRef("1")))
        .expect("SPI clock connection");
    board.set_net_name(sclk, "SPI_SCLK");

    let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())
        .expect("W5500 acceptance board must compile");

    let gnd_idx = report.board.find_net("GND").expect("GND net");
    let v3v3_idx = report.board.find_net("V3V3").expect("V3V3 net");
    let sclk_idx = report.board.find_net("SPI_SCLK").expect("SPI_SCLK net");

    let layout = Layout {
        placements: vec![
            Placement {
                component: 0,
                at: (55.0, 45.0),
                rotation: 0.0,
                side: BoardSide::Front,
            },
            Placement {
                component: 1,
                at: (20.0, 25.0),
                rotation: 0.0,
                side: BoardSide::Front,
            },
            Placement {
                component: 2,
                at: (82.0, 55.0),
                rotation: 90.0,
                side: BoardSide::Back,
            },
        ],
        tracks: vec![Track {
            net: sclk_idx,
            layer: CopperLayer(0),
            width: 0.25.mm(),
            path: vec![(55.0, 45.0), (68.0, 50.0), (82.0, 55.0)],
        }],
        vias: vec![Via {
            net: sclk_idx,
            at: (68.0, 50.0),
            drill: 0.4.mm(),
            diameter: 0.8.mm(),
            layers: (CopperLayer(0), CopperLayer(1)),
        }],
        zones: vec![
            Zone {
                net: gnd_idx,
                layer: CopperLayer(1),
                outline: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)],
            },
            Zone {
                net: v3v3_idx,
                layer: CopperLayer(0),
                outline: vec![(40.0, 35.0), (70.0, 35.0), (70.0, 55.0), (40.0, 55.0)],
            },
        ],
    };

    KiCad::new()
        .with_project_name(PROJECT)
        .emit_with_layout(&out, &report.board, &layout)
        .expect("emit W5500 acceptance project");

    for warning in &report.warnings {
        eprintln!(
            "[{:?}] {} — {}",
            warning.severity, warning.code, warning.message
        );
    }
    for finding in &report.validation {
        eprintln!(
            "validation [{:?}] {} — {}",
            finding.severity, finding.code, finding.message
        );
    }
    println!("emitted {} to {}", PROJECT, out.display());
}
