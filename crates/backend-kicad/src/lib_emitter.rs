//! KiCad library file emitter.
//!
//! Generates standalone `.kicad_sym` (symbol library) and `.kicad_mod`
//! (footprint) files from [`CompiledComponent`] data so the backend can
//! populate the idiomatic `symbols/` and `footprints/` directories.
//!
//! Symbol pin placement comes from [`crate::sym_layout`] (functional
//! auto-layout on the 100 mil grid); footprint pads come from
//! [`crate::fp_geom`] (the physical pad geometry carried by the part
//! definitions).

use copperleaf::{CompiledComponent, Role};

use crate::{
    common::{SymbolProps, footprint_ref, refdes_prefix, symbol_name},
    deterministic_id, fp_geom,
    sexpr::Sexpr,
    sym_layout::{self, LayoutPin},
};

/// Emit a standalone `.kicad_mod` footprint file for a single component.
///
/// `fp_name` is the footprint name used in the file content and (by the
/// caller) as the filename stem (e.g. `"RP2354A"` → `RP2354A.kicad_mod`).
pub fn emit_footprint_lib(comp: &CompiledComponent, fp_name: &str) -> String {
    footprint_def(comp, fp_name)
}

/// Emit a standalone `.kicad_sym` library file containing all given
/// components' symbols.
///
/// Symbols are deduplicated by their name within the library.
pub fn emit_symbol_lib(components: &[&CompiledComponent], _library_name: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut symbol_nodes: Vec<Sexpr> = Vec::new();

    for comp in components {
        let name = symbol_name(comp);
        if !seen.insert(name.to_string()) {
            continue; // duplicate symbol name in this library
        }
        symbol_nodes.push(symbol_def_sexpr(comp, name));
    }

    let mut children = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20251024")]),
        Sexpr::list([Sexpr::atom("generator"), Sexpr::str("copperleaf")]),
    ];
    children.extend(symbol_nodes);

    let lib = Sexpr::list(std::iter::once(Sexpr::atom("kicad_symbol_lib")).chain(children));
    format!("{}\n", lib)
}

/// Build the `(symbol ...)` definition for one component.
///
/// `symbol_name` is the name the definition is registered under: the plain
/// symbol name in `.kicad_sym` library files, or the full `lib:symbol`
/// identifier in a schematic's embedded `lib_symbols` section.
pub(crate) fn symbol_def_sexpr(comp: &CompiledComponent, symbol_name: &str) -> Sexpr {
    let mut layout_pins: Vec<LayoutPin> = comp
        .pins
        .iter()
        .enumerate()
        .map(|(i, p)| LayoutPin {
            name: p.name().to_string(),
            number: fp_geom::pin_number(p, i),
            role: p.role(),
        })
        .collect();

    for (number, name) in fp_geom::mech_pad_names(comp) {
        layout_pins.push(LayoutPin {
            name,
            number,
            role: Role::Passive,
        });
    }

    let layout = sym_layout::layout_symbol(&layout_pins);
    let fp_ref = footprint_ref(comp);
    let fp_filter = footprint_filter(comp);

    crate::common::build_symbol_sexpr(
        &SymbolProps {
            lib_id: symbol_name,
            reference: &refdes_prefix(&comp.refdes),
            value: symbol_name,
            footprint: &fp_ref,
            datasheet: comp.meta.datasheet.as_deref().unwrap_or("~"),
            description: comp.meta.description.as_deref().unwrap_or(""),
            fp_filter: Some(&fp_filter),
            bypass: comp.meta.is_bypass,
        },
        &layout,
    )
}

fn footprint_def(comp: &CompiledComponent, fp_name: &str) -> String {
    let pads = fp_geom::pads_from_component(comp);
    let extent = fp_geom::pads_extent(&pads);
    let seed = format!("fp:{}", fp_name);

    // KLC F9.1: the description should carry the datasheet URL when known.
    let descr = match (&comp.meta.description, &comp.meta.datasheet) {
        (Some(d), Some(url)) => format!("{}, {}", d, url),
        (Some(d), None) => d.clone(),
        (None, Some(url)) => url.clone(),
        (None, None) => format!("copperleaf-generated footprint for {}", comp.refdes),
    };

    let mut children = vec![
        Sexpr::str(fp_name),
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20231218")]),
        Sexpr::list([Sexpr::atom("generator"), Sexpr::str("copperleaf")]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom("F.Cu")]),
        Sexpr::list([Sexpr::atom("tedit"), Sexpr::atom("00000000")]),
        Sexpr::list([Sexpr::atom("descr"), Sexpr::str(&descr)]),
        Sexpr::list([Sexpr::atom("tags"), Sexpr::str("copperleaf")]),
        Sexpr::list([
            Sexpr::atom("attr"),
            Sexpr::atom(fp_geom::footprint_attr(&pads)),
        ]),
    ];

    // Text items: reference on silk, value + second reference on fab.
    let (cx, ref_y, val_y) = match extent {
        Some((x1, y1, x2, y2)) => ((x1 + x2) / 2.0, y1 - 1.52, y2 + 1.52),
        None => (0.0, -2.54, 2.54),
    };
    children.push(fp_geom::fp_text(
        "reference",
        "REF**",
        (cx, ref_y),
        "F.SilkS",
    ));
    children.push(fp_geom::fp_text("value", fp_name, (cx, val_y), "F.Fab"));
    children.push(fp_geom::fp_text("user", "${REFERENCE}", (cx, 0.0), "F.Fab"));

    // Pads.
    for pad in &pads {
        let uuid = deterministic_id(&format!("{}:pad:{}", seed, pad.number));
        children.push(fp_geom::pad_sexpr(pad, Some(&uuid), None));
    }

    // Outlines (fab, silk, courtyard, pin-1 marker).
    if let Some(ext) = extent {
        for node in fp_geom::outline_sexprs(ext, fp_geom::pin1_pos(&pads), Some(&seed)) {
            children.push(node);
        }
    }

    // 3D model reference (KLC F9.3; missing files are ignored by KiCad).
    children.push(fp_geom::model_sexpr(
        fp_name,
        comp.meta.model_3d.as_deref(),
        comp.meta.model_3d_offset,
        comp.meta.model_3d_rotation,
    ));

    let fp = Sexpr::list(std::iter::once(Sexpr::atom("footprint")).chain(children));
    format!("{}\n", fp)
}

/// `ki_fp_filters` property value for a component: the footprint name with
/// `-`/`_` escaped as `?` single-char wildcards (KLC S5.2), plus a trailing
/// `*`.
fn footprint_filter(comp: &CompiledComponent) -> String {
    let name = comp
        .meta
        .footprint
        .as_deref()
        .map(|s| s.split_once(':').map(|(_, n)| n).unwrap_or(s).to_string())
        .unwrap_or_else(|| comp.refdes.clone());
    let escaped: String = name
        .chars()
        .map(|c| if c == '-' || c == '_' { '?' } else { c })
        .collect();
    format!("{}*", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::UnitExt;
    use copperleaf::{ComponentMeta, Pad, PadShape, PadType, Pin};

    fn make_comp() -> CompiledComponent {
        CompiledComponent {
            refdes: "U1".into(),
            meta: ComponentMeta {
                symbol: Some("TestLib:TestPart".into()),
                footprint: Some("TestFP:QFP-32".into()),
                datasheet: Some("https://example.com/ds.pdf".into()),
                description: Some("A test component.".into()),
                ..ComponentMeta::default()
            },
            pins: vec![
                Pin::build("VDD")
                    .number("1")
                    .pos(0.0, 0.0)
                    .length(2.54)
                    .width(1.0)
                    .height(1.0)
                    .pad_type("smd")
                    .pwr_fixed(3.3.volt(), 0.1.amp())
                    .pin(),
                Pin::build("GND").number("2").gnd(),
                Pin::build("CLK").number("3").dio(),
            ],
            constraints: vec![],
            mechanical: vec![Pad {
                number: String::new(),
                pos: (0.0, 0.0),
                rotation: 0.0,
                width: 0.91,
                height: 0.91,
                pad_type: PadType::Smd,
                pad_shape: PadShape::RoundRect,
                roundrect_rratio: Some(0.25),
                layers: Some("F.Paste".into()),
                drill: None,
                solder_mask_margin: None,
            }],
        }
    }

    #[test]
    fn symbol_lib_starts_with_header() {
        let comp = make_comp();
        let out = emit_symbol_lib(&[&comp], "TestLib");
        assert!(out.starts_with("(kicad_symbol_lib"), "{}", out);
        assert!(out.contains("\"TestPart\""), "{}", out);
    }

    #[test]
    fn symbol_lib_contains_pins() {
        let comp = make_comp();
        let out = emit_symbol_lib(&[&comp], "TestLib");
        assert!(out.contains("\"VDD\""), "{}", out);
        assert!(out.contains("\"GND\""), "{}", out);
        assert!(out.contains("\"CLK\""), "{}", out);
    }

    #[test]
    fn symbol_lib_has_metadata() {
        let comp = make_comp();
        let out = emit_symbol_lib(&[&comp], "TestLib");
        assert!(out.contains("https://example.com/ds.pdf"), "{}", out);
        assert!(out.contains("A test component."), "{}", out);
        assert!(out.contains("ki_fp_filters"), "{}", out);
        assert!(out.contains("TestFP:QFP-32"), "{}", out);
    }

    #[test]
    fn symbol_pins_use_layout_not_pad_positions() {
        let comp = make_comp();
        let out = emit_symbol_lib(&[&comp], "TestLib");
        // Power pin on top, ground on bottom — not at pad position (0, 0).
        assert!(!out.contains("(at 0 0 0.0)"), "{}", out);
    }

    #[test]
    fn symbol_lib_deduplicates() {
        let comp = make_comp();
        let out = emit_symbol_lib(&[&comp, &comp], "TestLib");
        // Passing the same component twice should produce only one (symbol ...)
        // definition.  Count occurrences of the unit sub-symbol — there should
        // be exactly one.
        let units = out.matches("_0_1").count();
        assert_eq!(units, 1, "expected one sub-unit, got {}: {}", units, out);
    }

    #[test]
    fn footprint_lib_starts_with_header() {
        let comp = make_comp();
        let out = emit_footprint_lib(&comp, "QFP-32");
        assert!(out.starts_with("(footprint"), "{}", out);
        assert!(out.contains("\"QFP-32\""), "{}", out);
    }

    #[test]
    fn footprint_lib_contains_pads() {
        let comp = make_comp();
        let out = emit_footprint_lib(&comp, "QFP-32");
        assert!(out.contains("(pad \"1\""), "{}", out);
        assert!(out.contains("(pad \"3\""), "{}", out);
        assert!(out.contains("F.Fab"), "{}", out);
    }

    #[test]
    fn footprint_lib_uses_pad_geometry() {
        let comp = make_comp();
        let out = emit_footprint_lib(&comp, "QFP-32");
        // Pin 1 has explicit 1.0×1.0 smd geometry at the origin.
        assert!(out.contains("(pad \"1\" smd rect"), "{}", out);
        assert!(out.contains("(size 1 1)"), "{}", out);
        // Mechanical paste aperture is preserved.
        assert!(out.contains("F.Paste"), "{}", out);
        // Metadata.
        assert!(out.contains("A test component."), "{}", out);
        assert!(out.contains("(tags \"copperleaf\")"), "{}", out);
    }

    #[test]
    fn footprint_lib_has_outline() {
        let comp = make_comp();
        let out = emit_footprint_lib(&comp, "QFP-32");
        assert!(out.contains("fp_line"), "{}", out);
        assert!(out.contains("F.CrtYd"), "{}", out);
    }

    #[test]
    fn footprint_lib_falls_back_to_refdes() {
        let mut comp = make_comp();
        comp.meta.footprint = None;
        let out = emit_footprint_lib(&comp, "U1");
        assert!(out.contains("\"U1\""), "{}", out);
    }
}
