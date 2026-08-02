//! KiCad `.kicad_pro` project file emitter.

use copperleaf::DesignRules;
use serde_json::{Value, json};

use crate::sexpr::Sexpr;

/// Build a `fp-lib-table` S-expression file registering the project-specific
/// footprint library.  A KiCad footprint library is a **directory** of
/// `.kicad_mod` files, so the URI points at `footprints/`.
pub fn emit_fp_lib_table(lib_nick: &str) -> String {
    let entry = Sexpr::list([
        Sexpr::atom("lib"),
        Sexpr::list([Sexpr::atom("name"), Sexpr::str(lib_nick)]),
        Sexpr::list([Sexpr::atom("type"), Sexpr::str("KiCad")]),
        Sexpr::list([Sexpr::atom("uri"), Sexpr::str("${KIPRJMOD}/footprints")]),
        Sexpr::list([Sexpr::atom("options"), Sexpr::str("")]),
        Sexpr::list([Sexpr::atom("descr"), Sexpr::str("")]),
    ]);
    let table = Sexpr::list([Sexpr::atom("fp_lib_table"), entry]);
    format!("{}\n", table)
}

/// Build a `.kicad_pro` JSON document for a project named `name`.
///
/// When `rules` is provided, the board-level design rules (minimum clearances,
/// track widths, etc.) are emitted into the project's `design_settings.rules`
/// section.  This is the section validated by Quilter — the most critical value
/// is `min_clearance`, which **must** be greater than zero.
pub fn emit_project(name: &str, rules: Option<&DesignRules>) -> String {
    let mut root: Value = json!({
        "board": {
            "3dviewports": [],
            "design_settings": {
                "defaults": {
                    "board_outline_line_width": 0.05,
                    "copper_line_width": 0.2,
                    "copper_text_size_h": 1.5,
                    "copper_text_size_v": 1.5,
                    "copper_text_thickness": 0.3,
                    "courtyard_line_width": 0.05,
                    "fab_line_width": 0.1,
                    "fab_text_size_h": 1.0,
                    "fab_text_size_v": 1.0,
                    "fab_text_thickness": 0.15,
                    "other_line_width": 0.1,
                    "other_text_size_h": 1.0,
                    "other_text_size_v": 1.0,
                    "other_text_thickness": 0.15,
                    "pads": {
                        "drill": 0.762,
                        "height": 1.524,
                        "width": 1.524
                    },
                    "silk_line_width": 0.1,
                    "silk_text_size_h": 1.0,
                    "silk_text_size_v": 1.0,
                    "silk_text_thickness": 0.1
                },
                "diff_pair_dimensions": [],
                "drc_exclusions": [],
                "meta": { "version": 2 },
                "rule_severities": {
                    "clearance": "error",
                    "copper_edge_clearance": "error",
                    "courtyards_overlap": "error",
                    "hole_clearance": "error",
                    "shorting_items": "error",
                    "solder_mask_bridge": "error"
                },
                "track_widths": [],
                "via_dimensions": []
            },
            "ipc2581": { "dist": "", "distpn": "", "internal_id": "", "mfg": "", "mpn": "" },
            "layer_pairs": [],
            "layer_presets": [],
            "viewports": []
        },
        "boards": [],
        "component_class_settings": [],
        "cvpcb": { "equivalence_files": [] },
        "erc": {
            "erc_exclusions": [],
            "meta": { "version": 0 },
            "pin_map": [],
            "rule_severities": {
                "conflicting_pins": "error",
                "different_net": "error",
                "duplicate_sheet_numbers": "error",
                "invalid_label": "error",
                "missing_unit": "error",
                "power_pin_not_driven": "warning",
                "similar_labels": "warning"
            }
        },
        "libraries": {
            "pinned_footprint_libs": [],
            "pinned_symbol_libs": []
        },
        "meta": { "filename": name, "version": 3 },
        "net_settings": {
            "classes": [
                {
                    "clearance": 0.2,
                    "diff_pair_gap": 0.25,
                    "diff_pair_via_gap": 0.25,
                    "microvia_diameter": 0.3,
                    "microvia_drill": 0.1,
                    "name": "Default",
                    "pcb_color": "rgba(0, 0, 0, 0.000)",
                    "schematic_color": "rgba(0, 0, 0, 0.000)",
                    "track_width": 0.25,
                    "via_diameter": 0.8,
                    "via_drill": 0.4,
                    "wire_width": 6.0
                }
            ],
            "meta": { "version": 4 },
            "net_colors": null,
            "netclass_assignments": null,
            "netclass_patterns": []
        },
        "pcbnew": {
            "last_paths": { "gencad": "", "idf": "", "netlist": "", "plot": "", "pos_files": "", "specctra_dsn": "", "step": "", "svg": "", "vrml": "" },
            "page_layout_descr_file": ""
        },
        "schematic": {
            "annotate_start_num": 0,
            "bom_export_filename": "",
            "bom_fmt_presets": [],
            "bom_fmt_settings": {
                "field_delimiter": ",",
                "keep_line_breaks": false,
                "keep_tabs": false,
                "name": "CSV",
                "ref_delimiter": ",",
                "ref_range_delimiter": "",
                "string_delimiter": "\""
            },
            "bom_presets": [],
            "bom_settings": { "exclude_dnp": false, "fields_ordered": [], "fields_not_in_bom": [], "filter_string": "", "group_symbols": true, "include_dnp": true, "name": "Grouping", "normalize_field_names": false, "sort_string": "", "subgrouping": [] },
            "connection_grid_size": 50.0,
            "drawing": { "dashed_lines_dash_length_ratio": 12.0, "dashed_lines_gap_length_ratio": 3.0, "default_line_thickness": 6.0, "default_text_size": 50.0, "field_names": [], "intersheets_ref_own_page": false, "intersheets_ref_prefix": "", "intersheets_ref_short": false, "intersheets_ref_show": false, "intersheets_ref_suffix": "", "junction_size_choice": 3, "label_size_ratio": 0.375, "operating_point_overlay_opacity": 0.0, "pin_symbol_size": 25.0, "text_offset_ratio": 0.15 },
            "legacy_lib_dir": "",
            "legacy_lib_list": [],
            "meta": { "version": 1 },
            "page_layout_descr_file": "",
            "plot_directory": "",
            "subpart_first_id": 0,
            "subpart_id_separator": 0,
            "top_level_sheets": [{ "uuid": "00000000-0000-0000-0000-000000000000", "simplified": false }],
            "used_netclasses_only": false,
            "used_sheet_numbers": [],
            "used_designators": [],
            "variants": [],
            "reuse_designators": false
        },
        "sheets": [],
        "text_variables": {}
    });

    // Inject board-level design rules into the project file.
    // Without this, tools like Quilter see min_clearance=0 and reject the board.
    if let Some(r) = rules
        && let Some(board) = root.get_mut("board")
        && let Some(design) = board.get_mut("design_settings")
        && let Some(obj) = design.as_object_mut()
    {
        obj.insert(
            "rules".to_string(),
            json!({
                "max_error": 0.005,
                "min_clearance": r.min_clearance,
                "min_connection": r.min_connection,
                "min_copper_edge_clearance": r.min_copper_edge_clearance,
                "min_groove_width": 0.0,
                "min_hole_clearance": r.min_hole_clearance,
                "min_hole_to_hole": r.min_hole_to_hole,
                "min_microvia_diameter": r.min_microvia_diameter,
                "min_microvia_drill": r.min_microvia_drill,
                "min_resolved_spokes": 2,
                "min_silk_clearance": r.min_silk_clearance,
                "min_text_height": r.min_text_height,
                "min_text_thickness": r.min_text_thickness,
                "min_through_hole_diameter": r.min_through_hole_diameter,
                "min_track_width": r.min_track_width,
                "min_via_annular_width": r.min_via_annular_width,
                "min_via_diameter": r.min_via_diameter,
                "solder_mask_to_copper_clearance": r.solder_mask_to_copper_clearance,
                "use_height_for_length_calcs": true
            }),
        );
    }

    // Sync the Default net-class clearance with the board-level design rules.
    // KiCad enforces the net-class clearance for DRC, so a hardcoded 0.2 mm
    // would false-flag fine-pitch footprints (e.g. 0201 with 0.18 mm pad gap)
    // even when the board's design rules permit a tighter clearance.
    if let Some(r) = rules
        && let Some(net_settings) = root.get_mut("net_settings")
        && let Some(classes) = net_settings.get_mut("classes")
        && let Some(arr) = classes.as_array_mut()
        && let Some(default_class) = arr.first_mut()
        && let Some(obj) = default_class.as_object_mut()
    {
        obj.insert("clearance".to_string(), json!(r.min_clearance));
        obj.insert("track_width".to_string(), json!(r.min_track_width));
    }

    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".into()) + "\n"
}

pub fn emit_sym_lib_table(lib_nicks: &[String]) -> String {
    let mut entries = Vec::new();
    for nick in lib_nicks {
        entries.push(Sexpr::list([
            Sexpr::atom("lib"),
            Sexpr::list([Sexpr::atom("name"), Sexpr::str(nick)]),
            Sexpr::list([Sexpr::atom("type"), Sexpr::str("KiCad")]),
            Sexpr::list([
                Sexpr::atom("uri"),
                Sexpr::str(format!("${{KIPRJMOD}}/symbols/{}.kicad_sym", nick)),
            ]),
            Sexpr::list([Sexpr::atom("options"), Sexpr::str("")]),
            Sexpr::list([Sexpr::atom("descr"), Sexpr::str("")]),
        ]));
    }
    let table = Sexpr::list(std::iter::once(Sexpr::atom("sym_lib_table")).chain(entries));
    format!("{}\n", table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_is_valid_json() {
        let s = emit_project("example", None);
        let v: Value = serde_json::from_str(&s).expect("must be valid JSON");
        assert_eq!(v["meta"]["filename"], "example");
        assert_eq!(v["meta"]["version"], 3);
        assert!(
            v["net_settings"]["classes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["name"] == "Default")
        );
    }

    #[test]
    fn project_libraries_are_empty_by_default() {
        let s = emit_project("example", None);
        let v: Value = serde_json::from_str(&s).expect("must be valid JSON");
        assert!(
            v["libraries"]["pinned_symbol_libs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn fp_lib_table_points_at_directory() {
        let s = emit_fp_lib_table("copperleaf");
        assert!(s.contains("(name \"copperleaf\")"), "{s}");
        assert!(s.contains("${KIPRJMOD}/footprints"), "{s}");
        assert!(!s.contains(".kicad_mod"), "{s}");
    }
}
