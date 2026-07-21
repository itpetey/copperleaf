//! LLM-assisted datasheet-to-TOML generation using the local `opencode` CLI.
//!
//! This module extracts text from a PDF datasheet, prompts a locally-running
//! LLM via `opencode run`, and parses the resulting TOML into a Copperleaf
//! part manifest.

use copperleaf::{Diagnostic, Severity};
use copperleaf_part_codegen::Manifest;
use serde::Deserialize;

use crate::{CliError, NewArgs, UpdateArgs};

/// Parsed fragment of a `text` event emitted by `opencode run --format json`.
#[derive(Debug, Deserialize)]
struct OpencodeTextPart {
    text: String,
}

/// Top-level event emitted by `opencode run --format json`.
#[derive(Debug, Deserialize)]
struct OpencodeEvent {
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    part: Option<OpencodeTextPart>,
}

/// Generate a brand-new part manifest from a PDF datasheet.
pub fn new_from_datasheet(path: &str, args: &NewArgs) -> Result<Manifest, CliError> {
    let text = extract_pdf_text(path)?;
    let prompt = new_prompt(args.title.as_deref(), args.description.as_deref());
    let raw = call_opencode(&prompt, &[&text], &args.model)?;
    let toml = extract_toml(&raw, path)?;
    let manifest = crate::manifest::deserialise(&toml)?;
    Ok(manifest)
}

/// Enrich an existing part manifest from a PDF datasheet.
pub fn update_from_datasheet(
    path: &str,
    args: &UpdateArgs,
    existing: &Manifest,
) -> Result<Manifest, CliError> {
    let _ = args;
    let text = extract_pdf_text(path)?;
    let existing_toml = crate::manifest::serialise(existing);
    let prompt = update_prompt();
    let raw = call_opencode(&prompt, &[&existing_toml, &text], &args.model)?;
    let toml = extract_toml(&raw, path)?;
    let manifest = crate::manifest::deserialise(&toml)?;
    Ok(manifest)
}

/// Invoke `opencode run` with the supplied prompt and file attachments.
///
/// The files are written to a temporary directory which is also passed as
/// `--dir` so that `opencode` does not attempt to index the project
/// workspace (which produces noisy progress output and can be slow).
fn call_opencode(prompt: &str, file_contents: &[&str], model: &str) -> Result<String, CliError> {
    let dir = tempfile::tempdir()?;

    let mut file_paths = Vec::new();
    for (i, content) in file_contents.iter().enumerate() {
        let name = match i {
            0 => "existing.toml",
            1 => "datasheet.txt",
            _ => "extra.txt",
        };
        let path = dir.path().join(name);
        std::fs::write(&path, content)?;
        file_paths.push(path);
    }

    let mut cmd = std::process::Command::new("opencode");
    cmd.arg("run")
        .arg(prompt)
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model)
        .arg("--dangerously-skip-permissions")
        .arg("--dir")
        .arg(dir.path());

    for path in &file_paths {
        cmd.arg("--file").arg(path);
    }

    let output = cmd.output().map_err(|e| {
        CliError::Diagnostic(Diagnostic {
            code: "CLI:LLM_SPAWN".into(),
            severity: Severity::Error,
            message: format!("Failed to run `opencode`: {e}"),
            entities: vec![],
            hint: Some("Make sure the `opencode` CLI is installed and on PATH".into()),
        })
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut text = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<OpencodeEvent>(line)
            && event.ty == "text"
            && let Some(part) = event.part
        {
            text.push_str(&part.text);
        }
    }

    if text.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = if !output.status.success() {
            format!(
                "`opencode` exited with status {}: {}",
                output.status, stderr
            )
        } else {
            "`opencode` succeeded but returned no text output".to_string()
        };
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:LLM_EMPTY".into(),
            severity: Severity::Error,
            message,
            entities: vec![],
            hint: Some(
                "Check that `opencode` is configured with a provider and the prompt is valid"
                    .into(),
            ),
        }));
    }

    Ok(text)
}

/// Extract plain text from a PDF file.
fn extract_pdf_text(path: &str) -> Result<String, CliError> {
    pdf_extract::extract_text(path).map_err(|e| {
        CliError::Diagnostic(Diagnostic {
            code: "CLI:PDF_EXTRACT".into(),
            severity: Severity::Error,
            message: format!("Failed to extract text from PDF '{path}': {e}"),
            entities: vec![path.into()],
            hint: Some("Ensure the file is a readable PDF".into()),
        })
    })
}

/// Strip markdown fences and return the TOML payload.
fn extract_toml(raw: &str, source_path: &str) -> Result<String, CliError> {
    // Prefer a fenced ```toml block.
    if let Some(start) = raw.find("```toml") {
        let rest = &raw[start + "```toml".len()..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }

    // Fall back to the first generic fenced block.
    if let Some(start) = raw.find("```") {
        let rest = &raw[start + "```".len()..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }

    // No fences: assume the entire output is TOML.
    let trimmed = raw.trim();
    if trimmed.starts_with('[') {
        return Ok(trimmed.to_string());
    }

    Err(CliError::Diagnostic(Diagnostic {
        code: "CLI:LLM_TOML_NOT_FOUND".into(),
        severity: Severity::Error,
        message: format!("LLM response did not contain a valid TOML block for '{source_path}'"),
        entities: vec![source_path.into()],
        hint: Some("The model may have returned explanatory text instead of TOML".into()),
    }))
}

/// Prompt used when creating a new manifest from a datasheet.
fn new_prompt(title: Option<&str>, description: Option<&str>) -> String {
    let title_hint = title
        .map(|t| format!("\nUse this component title: {t}."))
        .unwrap_or_default();
    let description_hint = description
        .map(|d| format!("\nUse this component description: {d}."))
        .unwrap_or_default();

    format!(
        r#"You are a hardware engineer creating a Copperleaf part manifest TOML from a component datasheet.
The datasheet text is attached as `datasheet.txt`.
Read it carefully and produce a complete, valid TOML manifest.

Schema:

[component]
name = "PascalCaseName"        # Rust struct name for the generated code
title = "Manufacturer PartType Description (Package)"  # e.g. "Texas Instruments TPS63031 Buck-Boost Converter with 1-A Switches (QFN-10)"
description = "..."            # Optional one-line summary
datasheet = "..."              # URL to the component datasheet
lib_id = "..."                 # Library identifier used in KiCad

[[pin]]
num = 1
name = "..."
purpose = "..."                # Short functional description (e.g. "GPIO", "Supply", "Ground")
notes = "..."                  # Optional extra context
kind = "..."                   # Required: one of gnd, dio, analog_in, analog_rf, clk, spi, pwr, pwr_fixed, pwr_out
bw_mhz = 25.0                    # Required for kind=clk or kind=spi
v = 3.3                          # Required for kind=pwr_fixed or kind=pwr_out
v_min = 1.8                      # Required for kind=pwr
v_max = 3.3                      # Required for kind=pwr
i = 0.1                          # Required for kind=pwr_fixed or kind=pwr_out
i_max = 0.1                      # Required for kind=pwr
nc = false                       # Optional: true if this pin must not be connected

[[constraint]]
type = "..."                   # Exactly one of: Decoupling, LengthMatch, MaxJunction
values = ["100nF"]              # Required for Decoupling: capacitor values as unit strings
per_pin = false                 # Optional for Decoupling: true if each pin needs its own cap
group = "..."                   # Required for LengthMatch: net group name
skew_ps = 0.0                   # Required for LengthMatch: max skew in picoseconds
temp = "125C"                   # Required for MaxJunction: max junction temperature

Rules:
1. Use the exact pin numbering and names from the datasheet.
2. Format the title as "Manufacturer PartNumber Description (Package)" using the manufacturer name, part number, functional description, and package type from the datasheet. Include the datasheet URL in the datasheet field.
3. Choose the correct kind for each pin:
   - gnd: ground / VSS pins
   - pwr: supply input with a voltage range (requires v_min, v_max, i_max)
   - pwr_fixed: fixed-voltage regulator output or fixed supply (requires v, i)
   - pwr_out: power output (requires v, i)
   - dio: general digital I/O
   - analog_in: analog input
   - analog_rf: RF / high-speed analog differential pairs
   - clk: clock input/output (requires bw_mhz)
   - spi: SPI bus pins (requires bw_mhz)
4. For power pins include the required electrical fields; never leave them blank.
5. For clocks and SPI set a sensible bw_mhz based on the datasheet max frequency.
6. Add brief notes for ambiguous pins (e.g. "do not connect", "analog 3.3V", "active-low reset").
7. Only use these constraint types with their exact fields:
   - Decoupling: values (array of strings like "100nF"), per_pin (bool, optional)
   - LengthMatch: group (string), skew_ps (number)
   - MaxJunction: temp (string like "125C")
   Do NOT invent constraint types or fields not listed above.
8. Do NOT invent pins or values not present in the datasheet.
9. Output ONLY the TOML inside a single fenced code block (` ```toml ... ``` `). No explanatory text.{title_hint}{description_hint}"#,
        title_hint = title_hint,
        description_hint = description_hint,
    )
}

/// Prompt used when enriching an existing manifest from a datasheet.
fn update_prompt() -> String {
    r#"You are a hardware engineer updating a Copperleaf part manifest TOML from a component datasheet.
Two files are attached:

- `existing.toml` is the current manifest.
- `datasheet.txt` is the datasheet text.

Read both files and produce an updated, valid TOML manifest.

Rules:
1. Preserve every pin and every existing field unless the datasheet clearly contradicts it.
2. Enrich the title with manufacturer name, part number, functional description, and package type if not already descriptive. Format: "Manufacturer PartNumber Description (Package)".
3. Add the datasheet URL in the datasheet field if missing.
4. Enrich pins with purpose, notes, and electrical specs (v_min, v_max, i_max, v, i, bw_mhz) where the datasheet provides them.
5. Preserve the existing pin kind unless the datasheet clearly contradicts it.
6. Only add new pins if the datasheet explicitly lists them and they are missing from the existing manifest.
7. Use the Copperleaf pin kinds: gnd, dio, analog_in, analog_rf, clk, spi, pwr, pwr_fixed, pwr_out.
8. For kind=pwr include v_min, v_max, i_max. For kind=pwr_fixed or pwr_out include v and i. For kind=clk or spi include bw_mhz.
9. Only use these constraint types with their exact fields:
   - Decoupling: values (array of strings like "100nF"), per_pin (bool, optional)
   - LengthMatch: group (string), skew_ps (number)
   - MaxJunction: temp (string like "125C")
   Do NOT invent constraint types or fields not listed above.
10. Do NOT invent information not present in the datasheet.
11. Output ONLY the updated TOML inside a single fenced code block (` ```toml ... ``` `). No explanatory text."#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_toml_prefers_toml_fence() {
        let raw = r#"Here is the manifest:

```toml
[component]
name = "Test"
```

```
some other block
```
"#;
        let toml = extract_toml(raw, "test").unwrap();
        assert_eq!(toml, "[component]\nname = \"Test\"");
    }

    #[test]
    fn extract_toml_falls_back_to_first_fence() {
        let raw = r#"```
[component]
name = "Test"
```"#;
        let toml = extract_toml(raw, "test").unwrap();
        assert_eq!(toml, "[component]\nname = \"Test\"");
    }

    #[test]
    fn extract_toml_uses_whole_output_when_unfenced() {
        let raw = r#"[component]
name = "Test"

[[pin]]
num = 1
name = "A"
kind = "dio""#;
        let toml = extract_toml(raw, "test").unwrap();
        assert_eq!(toml, raw);
    }

    #[test]
    fn extract_toml_errors_on_non_toml_output() {
        let raw = "I cannot generate that file.";
        assert!(extract_toml(raw, "test").is_err());
    }

    #[test]
    fn parse_opencode_json_text_events() {
        let stdout = r#"{"type":"step_start","sessionID":"s"}
{"type":"text","sessionID":"s","part":{"text":"hello "}}
{"type":"text","sessionID":"s","part":{"text":"world"}}
{"type":"step_finish","sessionID":"s"}
"#;
        let text = parse_opencode_stdout(stdout);
        assert_eq!(text, "hello world");
    }

    fn parse_opencode_stdout(stdout: &str) -> String {
        let mut text = String::new();
        for line in stdout.lines() {
            let line = line.trim();
            if !line.starts_with('{') {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<OpencodeEvent>(line) {
                if event.ty == "text" {
                    if let Some(part) = event.part {
                        text.push_str(&part.text);
                    }
                }
            }
        }
        text
    }
}
