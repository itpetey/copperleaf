use copperleaf_core::UnitExt;
use copperleaf_ir::{Constraint, Design, Net, NetClass};

#[test]
fn json_snapshot_design_basic() {
    let mut d = Design::default();
    let mut v3v3 = Net::power("V3V3", 3.3.volt()).ripple(50.0.millivolt());
    v3v3.class = NetClass {
        min_width: Some(0.3.mm()),
        clearance: Some(0.2.mm()),
    };
    v3v3.constraints.push(Constraint::NetClass {
        min_width: 0.3.mm(),
        clearance: 0.2.mm(),
    });
    d.add_net(Net::power("VBUS", 5.0.volt()));
    d.add_net(Net::ground());
    d.add_net(v3v3);
    d.connect("U1", "VDD", "V3V3");

    let json = serde_json::to_string_pretty(&d).unwrap();
    let expected = r#"{
  "nets": [
    {
      "name": "VBUS",
      "kind": {
        "Power": {
          "v_nom": {
            "value": 5.0,
            "unit": "V"
          },
          "ripple": null
        }
      },
      "class": {
        "min_width": null,
        "clearance": null
      },
      "constraints": []
    },
    {
      "name": "GND",
      "kind": {
        "Power": {
          "v_nom": {
            "value": 0.0,
            "unit": "V"
          },
          "ripple": null
        }
      },
      "class": {
        "min_width": null,
        "clearance": null
      },
      "constraints": []
    },
    {
      "name": "V3V3",
      "kind": {
        "Power": {
          "v_nom": {
            "value": 3.3,
            "unit": "V"
          },
          "ripple": {
            "value": 0.05,
            "unit": "V"
          }
        }
      },
      "class": {
        "min_width": {
          "value": 0.0003,
          "unit": "m"
        },
        "clearance": {
          "value": 0.0002,
          "unit": "m"
        }
      },
      "constraints": [
        {
          "NetClass": {
            "min_width": {
              "value": 0.0003,
              "unit": "m"
            },
            "clearance": {
              "value": 0.0002,
              "unit": "m"
            }
          }
        }
      ]
    }
  ],
  "components": [],
  "constraints": [],
  "diagnostics": []
}"#;

    assert_eq!(json, expected);
}
