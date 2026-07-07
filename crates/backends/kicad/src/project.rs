//! KiCad `.kicad_pro` project file emitter.
//!
//! Emits a minimal but valid JSON project file that KiCad 6+ will open without
//! prompting the user to repair.  Empty arrays / default settings are acceptable
//! — KiCad fills in robust defaults on load — so we keep the footprint of this
//! structure small and maintainable.

use serde_json::{Value, json};

/// Build a minimal `.kicad_pro` JSON document for a project named `name`.
pub fn emit_project(name: &str) -> String {
    let root: Value = json!({
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
            "meta": { "version": 1 },
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
        "libraries": { "pinned_footprint_libs": [], "pinned_symbol_libs": [] },
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
            "meta": { "version": 5 },
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

    // Pretty-print with 2-space indentation, matching KiCad's own formatting.
    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".into()) + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_is_valid_json_with_top_level_keys() {
        let s = emit_project("example");
        let v: Value = serde_json::from_str(&s).expect("must be valid JSON");
        assert_eq!(v["meta"]["filename"], "example");
        for key in [
            "board",
            "boards",
            "erc",
            "libraries",
            "meta",
            "net_settings",
            "pcbnew",
            "schematic",
            "sheets",
            "text_variables",
        ] {
            assert!(v.get(key).is_some(), "missing top-level key: {key}");
        }
    }

    #[test]
    fn project_has_default_net_class() {
        let v: Value = serde_json::from_str(&emit_project("x")).unwrap();
        let classes = v["net_settings"]["classes"].as_array().unwrap();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0]["name"], "Default");
        assert_eq!(classes[0]["clearance"], 0.2);
        assert_eq!(classes[0]["track_width"], 0.25);
    }

    #[test]
    fn project_meta_version_matches_kicad6() {
        let v: Value = serde_json::from_str(&emit_project("foo")).unwrap();
        assert_eq!(v["meta"]["version"], 3);
        assert_eq!(v["net_settings"]["meta"]["version"], 5);
        assert_eq!(v["board"]["design_settings"]["meta"]["version"], 2);
        assert_eq!(v["erc"]["meta"]["version"], 1);
        assert_eq!(v["schematic"]["meta"]["version"], 1);
    }

    #[test]
    fn project_includes_filenames_in_last_paths() {
        let v: Value = serde_json::from_str(&emit_project("proj")).unwrap();
        let lp = &v["pcbnew"]["last_paths"];
        assert!(lp.get("netlist").is_some());
    }

    /// KiCad's `JSON_SETTINGS::SaveToFile` calls `nlohmann::json::update()` to
    /// merge its in-memory settings into the on-disk file.  The `update` method
    /// recurses when the *source* value is an object, converting `null` targets
    /// to empty objects along the way — but it throws `type_error(313)` when
    /// the target is a non-null, non-object value such as an array.
    ///
    /// These tests guard against fields that must be objects (or null) but were
    /// previously emitted as `[]`, causing KiCad to abort on save.
    #[test]
    fn bom_fmt_settings_is_object_not_array() {
        let v: Value = serde_json::from_str(&emit_project("x")).unwrap();
        let bfs = &v["schematic"]["bom_fmt_settings"];
        assert!(bfs.is_object(), "bom_fmt_settings must be an object, got {bfs}");
        assert_eq!(bfs["name"], "CSV");
        assert_eq!(bfs["field_delimiter"], ",");
    }

    #[test]
    fn net_colors_is_null_not_array() {
        let v: Value = serde_json::from_str(&emit_project("x")).unwrap();
        assert!(v["net_settings"]["net_colors"].is_null());
    }

    #[test]
    fn netclass_assignments_is_null_not_array() {
        let v: Value = serde_json::from_str(&emit_project("x")).unwrap();
        assert!(v["net_settings"]["netclass_assignments"].is_null());
    }
}
