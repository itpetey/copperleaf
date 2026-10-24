//! Vendor parts crate scaffolding.

use std::path::Path;

use crate::CliError;

pub fn scaffold(root: &Path, vendor: &str, lib_id: &str) -> Result<(), CliError> {
    let parts_dir = copperleaf::safe_join(root, "parts")?;
    let vendor_dir = copperleaf::safe_join(&parts_dir, vendor)?;
    std::fs::create_dir_all(&vendor_dir)?;

    let cargo_toml = copperleaf::safe_join(&vendor_dir, "Cargo.toml")?;
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

    let build_rs = copperleaf::safe_join(&vendor_dir, "build.rs")?;
    if !build_rs.exists() {
        std::fs::write(
            &build_rs,
            "//! Rebuild this crate when any part TOML changes.\n\
             fn main() {\n\
                 let dir = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"));\n\
                 if let Ok(entries) = std::fs::read_dir(dir) {\n\
                     for entry in entries.flatten() {\n\
                         let path = entry.path();\n\
                         if path.extension().is_some_and(|extension| extension == \"toml\") {\n\
                             if let Some(name) = path.file_name() {\n\
                                 println!(\"cargo:rerun-if-changed={}\", name.to_string_lossy());\n\
                             }\n\
                         }\n\
                     }\n\
                 }\n\
             }\n",
        )?;
    }

    let lib_rs = copperleaf::safe_join(&vendor_dir, "lib.rs")?;
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

    add_workspace_member(
        &copperleaf::safe_join(root, "Cargo.toml")?,
        &format!("parts/{vendor}"),
    )?;
    Ok(())
}

fn add_workspace_member(root_cargo: &Path, member: &str) -> Result<(), CliError> {
    let content = std::fs::read_to_string(root_cargo)?;
    let mut value: toml::Table = toml::from_str(&content).map_err(|error| {
        CliError::Diagnostic(copperleaf::Diagnostic {
            code: "CLI:WORKSPACE_UPDATE".into(),
            severity: copperleaf::Severity::Error,
            message: format!("cannot parse {}: {error}", root_cargo.display()),
            entities: vec![root_cargo.display().to_string()],
            hint: None,
        })
    })?;

    let workspace = value
        .get_mut("workspace")
        .and_then(|workspace| workspace.as_table_mut())
        .ok_or_else(|| {
            CliError::Diagnostic(copperleaf::Diagnostic {
                code: "CLI:WORKSPACE_UPDATE".into(),
                severity: copperleaf::Severity::Error,
                message: format!("{} has no [workspace] table", root_cargo.display()),
                entities: vec![root_cargo.display().to_string()],
                hint: None,
            })
        })?;

    let members = workspace
        .entry("members")
        .or_insert_with(|| toml::Value::Array(Vec::new()));
    let Some(members) = members.as_array_mut() else {
        return Err(CliError::Diagnostic(copperleaf::Diagnostic {
            code: "CLI:WORKSPACE_UPDATE".into(),
            severity: copperleaf::Severity::Error,
            message: format!(
                "[workspace].members in {} is not an array",
                root_cargo.display()
            ),
            entities: vec![root_cargo.display().to_string()],
            hint: None,
        }));
    };

    let member_value = toml::Value::String(member.to_string());
    if !members.contains(&member_value) {
        members.push(member_value);
    }

    let serialised = toml::to_string(&value).map_err(|error| {
        CliError::Diagnostic(copperleaf::Diagnostic {
            code: "CLI:WORKSPACE_UPDATE".into(),
            severity: copperleaf::Severity::Error,
            message: format!("cannot serialise {}: {error}", root_cargo.display()),
            entities: vec![root_cargo.display().to_string()],
            hint: None,
        })
    })?;
    std::fs::write(root_cargo, serialised)?;
    Ok(())
}

fn toml_filename(lib_id: &str) -> String {
    crate::manifest::toml_stem(lib_id)
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

#[cfg(test)]
mod containment_tests {
    use super::*;

    #[test]
    fn vendor_names_cannot_escape_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let result = scaffold(dir.path(), "../escape", "PART");
        assert!(result.is_err(), "traversal vendor must be rejected");
        assert!(!dir.path().join("escape").exists());
        assert!(!dir.path().join("parts").join("escape").exists());
    }

    #[test]
    fn absolute_vendor_names_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let result = scaffold(dir.path(), "/tmp/escape", "PART");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod rebuild_tests {
    use super::*;

    #[test]
    fn scaffold_writes_a_package_local_rebuild_script() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();

        scaffold(dir.path(), "newvendor", "PART").unwrap();

        let build_rs = dir.path().join("parts/newvendor/build.rs");
        assert!(build_rs.exists(), "scaffolded crate needs a build.rs");
        let content = std::fs::read_to_string(&build_rs).unwrap();
        assert!(content.contains("rerun-if-changed"), "{content}");

        let root = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
        let parsed: toml::Table = toml::from_str(&root).unwrap();
        let members = parsed["workspace"]["members"].as_array().unwrap();
        assert!(
            members
                .iter()
                .any(|member| member.as_str() == Some("parts/newvendor")),
            "{root}"
        );
    }

    #[test]
    fn empty_members_array_remains_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cargo = dir.path().join("Cargo.toml");
        std::fs::write(&cargo, "[workspace]\nmembers = []\n").unwrap();

        add_workspace_member(&cargo, "parts/vendor").unwrap();

        let content = std::fs::read_to_string(&cargo).unwrap();
        let parsed: toml::Table = toml::from_str(&content).unwrap();
        let members = parsed["workspace"]["members"].as_array().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].as_str(), Some("parts/vendor"));
    }

    #[test]
    fn wurth_crate_has_a_rebuild_hook() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../parts/wurth-elektronik/build.rs");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("rerun-if-changed"), "{content}");
    }
}
