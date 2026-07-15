//! Vendor parts crate scaffolding.

use std::path::Path;

use crate::CliError;

pub fn scaffold(root: &Path, vendor: &str, lib_id: &str) -> Result<(), CliError> {
    let vendor_dir = root.join("parts").join(vendor);
    std::fs::create_dir_all(&vendor_dir)?;

    let cargo_toml = vendor_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        let content = format!(
            "[package]\n\
             name = \"copperleaf-parts-{vendor}\"\n\
             version.workspace = true\n\
             edition.workspace = true\n\
             description = \"{vendor} vendor components\"\n\
             license.workspace = true\n\n\
             [lib]\n\
             path = \"lib.rs\"\n\n\
             [dependencies]\n\
             copperleaf.workspace = true\n\
             copperleaf-part-macro.workspace = true\n"
        );
        std::fs::write(&cargo_toml, content)?;
    }

    let lib_rs = vendor_dir.join("lib.rs");
    let mut lib_content = if lib_rs.exists() {
        std::fs::read_to_string(&lib_rs)?
    } else {
        format!("//! {vendor} vendor components\n\nuse copperleaf_part_macro::build_component;\n")
    };

    let component_line = format!("build_component!(\"{}.toml\");\n", toml_filename(lib_id));
    if !lib_content.contains(&component_line) {
        if !lib_content.ends_with('\n') {
            lib_content.push('\n');
        }
        lib_content.push_str(&component_line);
    }
    std::fs::write(&lib_rs, lib_content)?;

    add_workspace_member(&root.join("Cargo.toml"), &format!("parts/{vendor}"))?;
    Ok(())
}

fn toml_filename(lib_id: &str) -> String {
    let mut out = String::new();
    for ch in lib_id.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("part");
    }
    out
}

fn add_workspace_member(root_cargo: &Path, member: &str) -> Result<(), CliError> {
    let content = std::fs::read_to_string(root_cargo)?;
    if content.contains(&format!("\"{member}\"")) {
        return Ok(());
    }

    let member_line = format!("  \"{member}\"");
    let Some(start) = content.find("members = [") else {
        return Ok(());
    };
    let Some(end) = content[start..].find(']').map(|i| start + i) else {
        return Ok(());
    };

    let array = &content[start..=end];
    let insert_pos = if let Some(last_quote) = array.rfind('"') {
        start + last_quote + 1
    } else {
        start + "members = [".len()
    };

    let mut new_content = content;
    new_content.insert_str(insert_pos, &format!(",\n{member_line}"));
    std::fs::write(root_cargo, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_filename_normalises() {
        assert_eq!(toml_filename("RP2354A"), "rp2354a");
    }

    #[test]
    fn add_member_appends_once() {
        let dir = tempfile::tempdir().unwrap();
        let cargo = dir.path().join("Cargo.toml");
        std::fs::write(&cargo, "[workspace]\nmembers = [\n  \"parts/a\"\n]\n").unwrap();

        add_workspace_member(&cargo, "parts/b").unwrap();
        let content = std::fs::read_to_string(&cargo).unwrap();
        assert!(content.contains("\"parts/a\""));
        assert!(content.contains("\"parts/b\""));

        add_workspace_member(&cargo, "parts/b").unwrap();
        let content = std::fs::read_to_string(&cargo).unwrap();
        assert_eq!(content.matches("\"parts/b\"").count(), 1);
    }

    #[test]
    fn scaffold_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let cargo = dir.path().join("Cargo.toml");
        std::fs::write(&cargo, "[workspace]\nmembers = [\n  \"parts/a\"\n]\n").unwrap();

        scaffold(dir.path(), "testvendor", "W5500").unwrap();

        let cargo_toml = dir.path().join("parts/testvendor/Cargo.toml");
        assert!(cargo_toml.exists());
        let cargo_content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert!(cargo_content.contains("copperleaf-parts-testvendor"));

        let lib_rs = dir.path().join("parts/testvendor/lib.rs");
        assert!(lib_rs.exists());
        let lib_content = std::fs::read_to_string(&lib_rs).unwrap();
        assert!(lib_content.contains("build_component"));
        assert!(lib_content.contains("w5500.toml"));

        let root_content = std::fs::read_to_string(&cargo).unwrap();
        assert!(root_content.contains("\"parts/testvendor\""));
    }
}
