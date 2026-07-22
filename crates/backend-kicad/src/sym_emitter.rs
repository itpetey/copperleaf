//! KiCad symbol library emitter.
//!
//! Generates `.kicad_sym` files from Copperleaf [`Manifest`] data so the part
//! TOML can serve as the single source of truth for a component's symbol.
//!
//! Pin placement uses the shared functional auto-layout in
//! [`crate::sym_layout`]: power pins across the top, ground/thermal pins
//! across the bottom, and the remaining signals split left/right on the
//! 100 mil grid.

use copperleaf::Role;
use copperleaf_part_codegen::Manifest;

use crate::{
    sexpr::Sexpr,
    sym_layout::{self, LayoutPin},
};

/// Generate a `.kicad_sym` library S-expression string from a component manifest.
pub fn emit_symbol(manifest: &Manifest) -> String {
    let lib_id = manifest
        .component
        .lib_id
        .as_deref()
        .unwrap_or(&manifest.component.name);

    let mut lib_children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20251024")]),
        Sexpr::list([Sexpr::atom("generator"), Sexpr::str("copperleaf")]),
    ];

    lib_children.push(symbol_node(manifest, lib_id));

    Sexpr::list(
        [Sexpr::atom("kicad_symbol_lib")]
            .into_iter()
            .chain(lib_children),
    )
    .to_string()
}

/// Map a Copperleaf pin kind to a core [`Role`] for layout classification.
fn kind_to_role(kind: &str) -> Role {
    match kind {
        "gnd" => Role::Gnd,
        "pwr" | "pwr_fixed" => Role::PowerIn,
        "pwr_out" => Role::PowerOut,
        "analog_in" | "analog_rf" => Role::AnalogIn,
        _ => Role::DigitalIO,
    }
}

fn symbol_node(manifest: &Manifest, lib_id: &str) -> Sexpr {
    let layout_pins: Vec<LayoutPin> = manifest
        .pins
        .iter()
        .map(|p| LayoutPin {
            name: p.name.clone(),
            number: if p.number.is_empty() {
                p.num.to_string()
            } else {
                p.number.clone()
            },
            role: kind_to_role(&p.electrical.kind),
        })
        .collect();
    let layout = sym_layout::layout_symbol(&layout_pins);

    crate::common::build_symbol_sexpr(
        &crate::common::SymbolProps {
            lib_id,
            reference: "U",
            value: lib_id,
            footprint: "",
            datasheet: manifest.component.datasheet.as_deref().unwrap_or("~"),
            description: manifest.component.description.as_deref().unwrap_or(""),
            fp_filter: None,
            bypass: false,
        },
        &layout,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::{ComponentMeta, PinDef};

    fn pin_def(num: usize, name: &str, kind: &str) -> PinDef {
        PinDef {
            num,
            number: num.to_string(),
            name: name.into(),
            purpose: "Test".into(),
            notes: String::new(),
            electrical: copperleaf_part_codegen::ElectricalFields {
                kind: kind.into(),
                ..Default::default()
            },
            pos: None,
            rotation: None,
            length: None,
            width: None,
            height: None,
            pad_type: None,
            pad_shape: None,
            roundrect_rratio: None,
            solder_mask_margin: None,
            layers: None,
            drill: None,
            thermal_vias: vec![],
        }
    }

    fn make_test_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "TestPart".into(),
                title: "Test Part".into(),
                description: Some("A test component.".into()),
                datasheet: Some("https://example.com/ds.pdf".into()),
                lib_id: Some("TestPart".into()),
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![
                pin_def(1, "VDD", "pwr"),
                pin_def(2, "GND", "gnd"),
                pin_def(3, "CLK", "clk"),
                pin_def(4, "D0", "dio"),
                pin_def(5, "EXP", "gnd"),
            ],
            constraints: vec![],
            mechanical: vec![],
        }
    }

    #[test]
    fn emits_valid_s_expression() {
        let out = emit_symbol(&make_test_manifest());
        let parsed = crate::sexpr::parse(&out);
        assert!(parsed.is_ok(), "failed to parse: {out}");
    }

    #[test]
    fn contains_symbol_header() {
        let out = emit_symbol(&make_test_manifest());
        assert!(
            out.starts_with("(kicad_symbol_lib"),
            "missing kicad_symbol_lib header"
        );
        assert!(out.contains("\"TestPart\""), "missing symbol name");
    }

    #[test]
    fn contains_all_pins() {
        let out = emit_symbol(&make_test_manifest());
        assert!(out.contains("\"VDD\""), "missing VDD pin");
        assert!(out.contains("\"GND\""), "missing GND pin");
        assert!(out.contains("\"CLK\""), "missing CLK pin");
        assert!(out.contains("\"D0\""), "missing D0 pin");
        assert!(out.contains("\"EXP\""), "missing EXP pin");
    }

    #[test]
    fn contains_properties() {
        let out = emit_symbol(&make_test_manifest());
        assert!(out.contains("\"Reference\""), "missing Reference property");
        assert!(out.contains("\"Value\""), "missing Value property");
        assert!(out.contains("\"Footprint\""), "missing Footprint property");
    }

    #[test]
    fn contains_datasheet_when_set() {
        let out = emit_symbol(&make_test_manifest());
        assert!(
            out.contains("https://example.com/ds.pdf"),
            "missing datasheet URL"
        );
    }

    #[test]
    fn pins_are_functionally_placed() {
        let out = emit_symbol(&make_test_manifest());
        // Power pins on top (rotation 270), grounds/thermal on the bottom
        // (rotation 90), signals on the sides (0/180).
        let vdd = out.find("\"VDD\"").unwrap();
        let gnd = out.find("\"GND\"").unwrap();
        let exp = out.find("\"EXP\"").unwrap();
        let _ = (vdd, gnd, exp);
        // Each pin's `(at x y rot)` appears before its name in the pin node;
        // check rotations exist in the output at all.
        assert!(out.contains(" 270)"), "missing top pins: {out}");
        assert!(out.contains(" 90)"), "missing bottom pins: {out}");
    }

    #[test]
    fn round_trip_through_parser() {
        let manifest = make_test_manifest();
        let out = emit_symbol(&manifest);

        // Parse it back.
        let symbols =
            crate::sym_parser::parse_symbol_lib(&out).expect("should parse generated symbol");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].lib_id, "TestPart");
        assert_eq!(symbols[0].pins.len(), 5);
        assert_eq!(
            symbols[0].datasheet.as_deref(),
            Some("https://example.com/ds.pdf")
        );
    }
}
