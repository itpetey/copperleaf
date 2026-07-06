use std::{env, fs};

use copperleaf::{
    ComponentInst, Constraint, Design, Net, NetClass, Role, UnitExt, backend_kicad,
    erc_voltage_pin_to_net, parts, synthesize_decoupling,
};

#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum PatchOp {
    // { "op": "connect", "net": "VDD", "pins": ["U1.VDD", "C1.1"] }
    Connect { net: String, pins: Vec<String> },
}

#[derive(serde::Deserialize)]
struct Patch {
    ops: Vec<PatchOp>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
        return;
    }
    match args[1].as_str() {
        "verify" => cmd_verify(&args[2..]),
        "export" => cmd_export(&args[2..]),
        "json" => cmd_json(&args[2..]),
        "decouple" => cmd_decouple(&args[2..]),
        "report" => cmd_report(&args[2..]),
        "emit" => cmd_emit(),
        "apply" => cmd_apply(&args[2..]),
        _ => usage(),
    }
}

fn load_design(path: Option<&str>) -> Design {
    match path {
        Some(p) => {
            let data = match fs::read_to_string(p) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error reading design file '{}': {}", p, e);
                    std::process::exit(1);
                }
            };
            match serde_json::from_str(&data) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error parsing design file '{}': {}", p, e);
                    std::process::exit(1);
                }
            }
        }
        None => build_example_design(),
    }
}

fn build_example_design() -> Design {
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

    let buck = parts::Buck::new("MPM3610", 3.3.volt(), 2.0.amp());
    let u_reg = ComponentInst::new("U1", buck);

    let mcu = parts::Mcu::new("STM32F405RG");
    let u_mcu = ComponentInst::new("U2", mcu);

    let mut d = Design::default();
    d.add_net(vbus);
    d.add_net(gnd);
    d.add_net(v3v3);
    d.add_component(&u_reg);
    d.add_component(&u_mcu);
    d.connect("U1", "VIN", "VBUS");
    d.connect("U1", "GND", "GND");
    d.connect("U2", "VDD", "V3V3");
    d.connect("U2", "VSS", "GND");

    d.add_constraint(Constraint::ResonanceIndex { max: 0.5 });
    d.add_constraint(Constraint::MaxJunction {
        temp: 85.0.celsius(),
    });
    d
}

fn read_or_exit(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn cmd_apply(args: &[String]) {
    if args.len() != 3 {
        return usage();
    }
    let in_path = &args[0];
    let patch_path = &args[1];
    let out_path = &args[2];

    let data = read_or_exit(in_path);
    let mut d: Design = match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing design file '{}': {}", in_path, e);
            std::process::exit(1);
        }
    };
    let patch_data = read_or_exit(patch_path);
    let patch: Patch = match serde_json::from_str(&patch_data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing patch file '{}': {}", patch_path, e);
            std::process::exit(1);
        }
    };

    for op in patch.ops {
        match op {
            PatchOp::Connect { net, pins } => {
                for p in pins {
                    if let Some((r, pin)) = p.split_once('.') {
                        d.connect(r, pin, &net);
                    }
                }
            }
        }
    }

    let out = match serde_json::to_string_pretty(&d) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error serializing design: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = fs::write(out_path, out) {
        eprintln!("Error writing output file '{}': {}", out_path, e);
        std::process::exit(1);
    }
}

fn cmd_decouple(args: &[String]) {
    let d = load_design(args.first().map(|s| s.as_str()));
    let result = synthesize_decoupling(&d);
    if result.caps.is_empty() {
        println!("[Info] DECOUPLE: no capacitors placed");
    } else {
        for cap in &result.caps {
            println!(
                "  {}: {} F on {} (from {}.{})",
                cap.refdes,
                cap.value.as_base(),
                cap.net,
                cap.source_component,
                cap.source_pin,
            );
        }
    }
    for diag in &result.diagnostics {
        println!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
    }
}

fn cmd_emit() {
    let d = build_example_design();
    match serde_json::to_string_pretty(&d) {
        Ok(s) => println!("{}", s),
        Err(e) => {
            eprintln!("Error serializing design: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_export(args: &[String]) {
    let d = load_design(args.first().map(|s| s.as_str()));
    let txt = backend_kicad::emit_netlist_text(&d);
    println!("{}", txt);
}

fn cmd_json(args: &[String]) {
    let d = load_design(args.first().map(|s| s.as_str()));
    match serde_json::to_string_pretty(&d) {
        Ok(s) => println!("{}", s),
        Err(e) => {
            eprintln!("Error serializing design: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_report(args: &[String]) {
    let d = load_design(args.first().map(|s| s.as_str()));
    println!("{}", copperleaf::report(&d));
}

fn cmd_verify(args: &[String]) {
    let d = load_design(args.first().map(|s| s.as_str()));
    let mut issues = false;
    for c in &d.components {
        for pin in &c.pins {
            if !matches!(pin.role, Role::PowerIn) {
                continue;
            }
            for net_name in d.nets_of_pin(&c.refdes, &pin.name) {
                if let Some(net) = d.nets.iter().find(|n| n.name == net_name) {
                    if let Some(diag) = erc_voltage_pin_to_net(net, pin) {
                        println!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
                        issues = true;
                    }
                }
            }
        }
    }
    if !issues {
        println!("[Info] ERC:OK — no overvoltage detected");
    }
}

fn usage() {
    eprintln!(
        "Usage: copperleaf <verify|export|json|decouple|report|emit|apply>\n  verify|export|json|decouple|report [design.json]\n  emit\n  apply <in.json> <patch.json> <out.json>"
    );
}
