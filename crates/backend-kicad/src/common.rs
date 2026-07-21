//! Shared helpers for the KiCad backend emitters.

use copperleaf::{CompiledBoard, CompiledComponent, Role};

use crate::sexpr::Sexpr;

/// Nickname of the project-local library written into `symbols/`/`footprints/`.
pub const PROJECT_LIB: &str = "copperleaf";

/// Properties passed to [`build_symbol_sexpr`].
pub struct SymbolProps<'a> {
    /// Library identifier (e.g. `"copperleaf:RP2354A"`).
    pub lib_id: &'a str,
    /// KiCad `Reference` property value.
    pub reference: &'a str,
    /// KiCad `Value` property value.
    pub value: &'a str,
    /// KiCad `Footprint` property value.
    pub footprint: &'a str,
    /// Datasheet URL (or `"~"`).
    pub datasheet: &'a str,
    /// Human-readable description.
    pub description: &'a str,
    /// Optional `ki_fp_filters` value.
    pub fp_filter: Option<&'a str>,
}

/// Build deterministic 1-based net codes from a compiled board.
pub fn build_net_codes(board: &CompiledBoard) -> Vec<(String, usize)> {
    board
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i + 1))
        .collect()
}

/// Build the complete `(symbol …)` S-expression for a symbol definition.
///
/// This is the single source of truth for symbol emission — both the
/// standalone `.kicad_sym` path ([`crate::sym_emitter`]) and the board-
/// compile path ([`crate::lib_emitter`] / [`crate::schematic`]) call this
/// function with their respective property values.
pub fn build_symbol_sexpr(props: &SymbolProps, layout: &crate::sym_layout::SymbolLayout) -> Sexpr {
    let lib_id = props.lib_id;
    let mut children = vec![
        Sexpr::atom("symbol"),
        Sexpr::str(lib_id),
        Sexpr::list([Sexpr::atom("exclude_from_sim"), Sexpr::atom("no")]),
        Sexpr::list([Sexpr::atom("in_bom"), Sexpr::atom("yes")]),
        Sexpr::list([Sexpr::atom("on_board"), Sexpr::atom("yes")]),
        property_sym_node(
            "Reference",
            props.reference,
            (layout.x1, layout.y1 + 1.27),
            false,
            true,
        ),
        property_sym_node(
            "Value",
            props.value,
            (layout.x1, layout.y2 - 1.27),
            false,
            true,
        ),
        property_sym_node("Footprint", props.footprint, (0.0, 0.0), true, false),
        property_sym_node("Datasheet", props.datasheet, (0.0, 0.0), true, false),
        property_sym_node("Description", props.description, (0.0, 0.0), true, false),
        property_sym_node("ki_keywords", "copperleaf", (0.0, 0.0), true, false),
    ];
    if let Some(filter) = props.fp_filter {
        children.push(property_sym_node(
            "ki_fp_filters",
            filter,
            (0.0, 0.0),
            true,
            false,
        ));
    }

    // Unit sub-symbol with body and pins.
    let bare = lib_id.split(':').next_back().unwrap_or(lib_id);
    let unit_name = format!("{}_0_1", bare);
    let mut unit = vec![Sexpr::atom("symbol"), Sexpr::str(&unit_name)];
    unit.push(crate::sym_layout::body_rect_sexpr(layout));
    for pin in &layout.pins {
        unit.push(crate::sym_layout::placed_pin_sexpr(pin));
    }
    children.push(Sexpr::list(unit));

    Sexpr::list(children)
}

/// Convert metres to millimetres as a formatted string.
pub fn fmt_mm(meters: f64) -> String {
    format_float(meters * 1000.0, 6)
}

/// Full `lib:footprint` reference used in symbol properties and the PCB.
pub fn footprint_ref(comp: &CompiledComponent) -> String {
    match comp.meta.footprint.as_deref() {
        Some(s) => s.to_owned(),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// Format a floating-point value, trimming trailing zeros and normalising
/// negative zero (`-0.0` → `"0"`).
pub fn format_float(v: f64, decimals: usize) -> String {
    let v = if v == 0.0 { 0.0 } else { v };
    if decimals == 0 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{:.prec$}", v, prec = decimals);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Format a coordinate rounded to the 0.01 mm grid required by KLC for
/// courtyard (and fab) geometry.
pub fn format_grid_float(v: f64) -> String {
    format_float((v * 100.0).round() / 100.0, 2)
}

/// The component's footprint name when it refers to a project-local
/// footprint (`None` for external `lib:name` references).
pub fn local_footprint_name(comp: &CompiledComponent) -> Option<String> {
    match comp.meta.footprint.as_deref() {
        Some(s) if s.contains(':') => None,
        Some(s) => Some(s.to_string()),
        None => Some(comp.refdes.clone()),
    }
}

/// Build a `(property …)` node for schematic and symbol-library contexts.
///
/// Emitted properties always use 1.27 mm font size.  Pass `justify_left: true`
/// when the property appears at a non-zero position so KiCad left-aligns the
/// text; pass `false` for properties at the origin (hidden metadata).
pub fn property_sym_node(
    key: &str,
    value: &str,
    pos: (f64, f64),
    hide: bool,
    justify_left: bool,
) -> Sexpr {
    let mut effects = vec![Sexpr::list([
        Sexpr::atom("font"),
        Sexpr::list([
            Sexpr::atom("size"),
            Sexpr::atom("1.27"),
            Sexpr::atom("1.27"),
        ]),
    ])];
    if justify_left {
        effects.push(Sexpr::list([Sexpr::atom("justify"), Sexpr::atom("left")]));
    }

    let mut children = vec![
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(pos.0, 2)),
            Sexpr::atom(format_float(pos.1, 2)),
            Sexpr::atom("0"),
        ]),
    ];
    if hide {
        children.push(Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]));
    }
    children.push(Sexpr::list(
        std::iter::once(Sexpr::atom("effects")).chain(effects),
    ));
    Sexpr::list(children)
}

/// Extract refdes prefix (e.g. `U1` -> `U`).
pub fn refdes_prefix(refdes: &str) -> String {
    let alpha: String = refdes.chars().take_while(|c| c.is_alphabetic()).collect();
    if alpha.is_empty() {
        "?".to_string()
    } else {
        alpha
    }
}

/// Map a Copperleaf pin role to a KiCad pin type string.
pub fn role_to_pin_type(role: Role) -> &'static str {
    match role {
        Role::PowerIn | Role::Gnd => "power_in",
        Role::PowerOut => "power_out",
        Role::AnalogIn => "input",
        Role::AnalogOut => "output",
        Role::DigitalIO | Role::DiffPos | Role::DiffNeg => "bidirectional",
        Role::Passive => "passive",
    }
}

/// Full `lib:symbol` identifier used by schematic instances.
pub fn symbol_lib_id(comp: &CompiledComponent) -> String {
    match comp.meta.symbol.as_deref() {
        Some(s) if s.contains(':') => s.to_string(),
        Some(s) => format!("{}:{s}", PROJECT_LIB),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// Symbol library nickname for a component: the prefix of `symbol()` when it
/// contains a `':'`, otherwise the project-local library.
pub fn symbol_lib_nick(comp: &CompiledComponent) -> String {
    comp.meta
        .symbol
        .as_deref()
        .and_then(|s| s.split_once(':').map(|(l, _)| l.to_string()))
        .unwrap_or_else(|| PROJECT_LIB.to_string())
}

/// Symbol name (without library prefix) for a component.
pub fn symbol_name(comp: &CompiledComponent) -> &str {
    comp.meta
        .symbol
        .as_deref()
        .map(|s| s.split_once(':').map(|(_, n)| n).unwrap_or(s))
        .unwrap_or(&comp.refdes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_float_trims_zeros() {
        assert_eq!(format_float(25.4, 2), "25.4");
        assert_eq!(format_float(100.0, 2), "100");
    }
}
