//! KiCad backend: netlist, schematic, and PCB emitters for the Copperleaf IR.

use std::fs;

use copperleaf_core::{Diagnostic, Severity};
use copperleaf_ir::Design;

pub mod common;
pub mod netlist;
pub mod pcb;
pub mod project;
pub mod schematic;
pub mod sexpr;
pub mod sym_parser;

pub use common::{build_net_codes, fmt_mm, format_float, refdes_prefix};
pub use netlist::emit_netlist;
pub use pcb::emit_pcb;
pub use project::emit_project;
pub use schematic::emit_schematic;
pub use sexpr::{Sexpr, deterministic_uuid, kv};
pub use sym_parser::{PinDef, SymbolDef, find_symbol, parse_symbol_lib};

/// Resolve KiCad symbol pin positions for components that declare a
/// `kicad_symbol` but do not yet have per-pin positions set.
///
/// Reads the `.kicad_sym` file at `lib_path` once, parses it, and matches each
/// IR pin by name (case-insensitive) against the symbol library. Missing
/// symbols or unmatched pins produce warning diagnostics attached to `design`.
pub fn resolve_symbols(design: &mut Design, lib_path: &str) {
    let symbols = match fs::read_to_string(lib_path) {
        Ok(content) => match parse_symbol_lib(&content) {
            Ok(syms) => syms,
            Err(e) => {
                design.diagnostics.push(Diagnostic {
                    code: "SYM:PARSE".into(),
                    severity: Severity::Warning,
                    message: format!("Failed to parse symbol library '{}': {}", lib_path, e),
                    entities: vec![lib_path.into()],
                    hint: Some("Check the file is a valid .kicad_sym file".into()),
                });
                return;
            }
        },
        Err(e) => {
            design.diagnostics.push(Diagnostic {
                code: "SYM:READ".into(),
                severity: Severity::Warning,
                message: format!("Failed to read symbol library '{}': {}", lib_path, e),
                entities: vec![lib_path.into()],
                hint: Some("Verify the --symbol-lib path is correct".into()),
            });
            return;
        }
    };

    for comp in &mut design.components {
        let Some(sym_id) = &comp.kicad_symbol else {
            continue;
        };
        if !comp.pins.iter().any(|p| p.pos.is_none()) {
            continue;
        }

        let Some(sym) = find_symbol(&symbols, sym_id) else {
            design.diagnostics.push(Diagnostic {
                code: "SYM:NOT_FOUND".into(),
                severity: Severity::Warning,
                message: format!("Symbol '{}' not found in library '{}'", sym_id, lib_path),
                entities: vec![comp.refdes.clone(), sym_id.clone()],
                hint: Some("Check the symbol name and library file".into()),
            });
            continue;
        };

        for pin in &mut comp.pins {
            if pin.pos.is_some() {
                continue;
            }
            let Some(pin_def) = sym
                .pins
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(&pin.name))
            else {
                design.diagnostics.push(Diagnostic {
                    code: "SYM:PIN_MISMATCH".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "Pin '{}.{}' not found in symbol '{}'",
                        comp.refdes, pin.name, sym_id
                    ),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name), sym_id.clone()],
                    hint: Some("Check the pin name matches the KiCad symbol".into()),
                });
                continue;
            };
            pin.pos = Some((pin_def.pos.0, pin_def.pos.1));
            pin.rotation = Some(pin_def.rotation);
            pin.length = Some(pin_def.length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{ComponentInst, Design, Limits, Pin, Role};
    use std::io::Write;

    fn sample_sym_lib() -> &'static str {
        r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
    (pin power_in line (at -15.24 -5.08 0) (length 2.54) (name "GND") (number "2"))
  )
)"#
    }

    fn temp_lib(contents: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("copperleaf_test_{}.kicad_sym", n));
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[derive(Clone, Debug)]
    struct SymBlock {
        pins: Vec<Pin>,
        symbol: Option<&'static str>,
    }

    impl copperleaf_ir::Block for SymBlock {
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
        fn kicad_symbol(&self) -> Option<&str> {
            self.symbol
        }
    }

    #[test]
    fn resolve_fills_pin_positions() {
        let path = temp_lib(sample_sym_lib());
        let mut d = Design::default();
        let block = SymBlock {
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
            ],
            symbol: Some("RP2040:RP2354a"),
        };
        d.add_component(ComponentInst::new("U1", block));

        resolve_symbols(&mut d, path.to_str().unwrap());

        let u1 = d.component_by_refdes("U1").unwrap();
        assert_eq!(u1.pins[0].pos, Some((-15.24, 5.08)));
        assert_eq!(u1.pins[0].rotation, Some(0.0));
        assert_eq!(u1.pins[0].length, Some(2.54));
        assert_eq!(u1.pins[1].pos, Some((-15.24, -5.08)));
        assert_eq!(u1.pins[1].rotation, Some(0.0));
        assert_eq!(u1.pins[1].length, Some(2.54));
        assert!(d.diagnostics.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolve_warns_when_symbol_missing() {
        let path = temp_lib(sample_sym_lib());
        let mut d = Design::default();
        let block = SymBlock {
            pins: vec![Pin::new(
                "VDD",
                Role::PowerIn,
                Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                None,
            )],
            symbol: Some("Missing:Missing"),
        };
        d.add_component(ComponentInst::new("U1", block));

        resolve_symbols(&mut d, path.to_str().unwrap());

        assert!(
            d.diagnostics
                .iter()
                .any(|diag| diag.code == "SYM:NOT_FOUND")
        );
        let u1 = d.component_by_refdes("U1").unwrap();
        assert_eq!(u1.pins[0].pos, None);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolve_warns_when_pin_missing() {
        let path = temp_lib(sample_sym_lib());
        let mut d = Design::default();
        let block = SymBlock {
            pins: vec![Pin::new(
                "NO_SUCH_PIN",
                Role::PowerIn,
                Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                None,
            )],
            symbol: Some("RP2040:RP2354a"),
        };
        d.add_component(ComponentInst::new("U1", block));

        resolve_symbols(&mut d, path.to_str().unwrap());

        assert!(
            d.diagnostics
                .iter()
                .any(|diag| diag.code == "SYM:PIN_MISMATCH")
        );
        let u1 = d.component_by_refdes("U1").unwrap();
        assert_eq!(u1.pins[0].pos, None);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolve_skips_when_positions_already_set() {
        let path = temp_lib(sample_sym_lib());
        let mut d = Design::default();
        let mut pin = Pin::new(
            "VDD",
            Role::PowerIn,
            Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
            None,
        );
        pin.pos = Some((1.0, 2.0));
        pin.rotation = Some(90.0);
        let block = SymBlock {
            pins: vec![pin],
            symbol: Some("RP2040:RP2354a"),
        };
        d.add_component(ComponentInst::new("U1", block));

        resolve_symbols(&mut d, path.to_str().unwrap());

        let u1 = d.component_by_refdes("U1").unwrap();
        assert_eq!(u1.pins[0].pos, Some((1.0, 2.0)));
        assert_eq!(u1.pins[0].rotation, Some(90.0));
        assert!(d.diagnostics.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolve_skips_components_without_symbol() {
        let path = temp_lib(sample_sym_lib());
        let mut d = Design::default();
        let block = SymBlock {
            pins: vec![Pin::new(
                "VDD",
                Role::PowerIn,
                Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                None,
            )],
            symbol: None,
        };
        d.add_component(ComponentInst::new("U1", block));

        resolve_symbols(&mut d, path.to_str().unwrap());

        let u1 = d.component_by_refdes("U1").unwrap();
        assert_eq!(u1.pins[0].pos, None);
        assert!(d.diagnostics.is_empty());

        std::fs::remove_file(&path).ok();
    }
}
