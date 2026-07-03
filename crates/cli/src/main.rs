use std::{env, fs};

use copperleaf::{
    ComponentInst, Constraint, Design, Limits, Net, NetClass, Pin, Role, UnitExt, backend_kicad,
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
        "verify" => cmd_verify(),
        "export" => cmd_export(),
        "json" => cmd_json(),
        "decouple" => cmd_decouple(),
        "apply" => cmd_apply(&args[2..]),
        _ => usage(),
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

fn cmd_apply(args: &[String]) {
    if args.len() != 3 {
        return usage();
    }
    let in_path = &args[0];
    let patch_path = &args[1];
    let out_path = &args[2];

    let data = fs::read_to_string(in_path).expect("read design");
    let mut d: Design = serde_json::from_str(&data).expect("parse design");
    let patch_data = fs::read_to_string(patch_path).expect("read patch");
    let patch: Patch = serde_json::from_str(&patch_data).expect("parse patch");

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

    let out = serde_json::to_string_pretty(&d).expect("serialize design");
    fs::write(out_path, out).expect("write output");
}

fn cmd_decouple() {
    let d = build_example_design();
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

fn cmd_export() {
    let d = build_example_design();
    let txt = backend_kicad::emit_netlist_text(&d);
    println!("{}", txt);
}

fn cmd_json() {
    let d = build_example_design();
    match serde_json::to_string_pretty(&d) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("Error serializing design: {}", e),
    }
}

fn cmd_verify() {
    let d = build_example_design();
    let vdd_pin = Pin {
        name: "VDD".into(),
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
    } else {
        println!("[Info] ERC:OK — no overvoltage detected");
    }
}

fn usage() {
    eprintln!(
        "Usage: copperleaf <verify|export|json|decouple|apply>\n  apply <in.json> <patch.json> <out.json>"
    );
}
