# sigspec-presets Specification

## Purpose
TBD - created by archiving change analysis-stdlib. Update Purpose after archive.
## Requirements
### Requirement: SigSpec preset factory methods
`SigSpec` SHALL provide associated functions `::spi(bw_mhz: f64)`, `::spi_clk(bw_mhz: f64)`, `::control()`, and `::rf_50ohm()` that construct common signal specifications.

#### Scenario: SPI preset
- **WHEN** `SigSpec::spi(50.0)` is called
- **THEN** the result has `kind == SigKind::Generic`, `bandwidth` representing 50 MHz period, and `target_impedance == 50.0.ohm()`

#### Scenario: SPI clock preset
- **WHEN** `SigSpec::spi_clk(50.0)` is called
- **THEN** the result has `kind == SigKind::Clock` and `bandwidth` representing 50 MHz

#### Scenario: Control preset
- **WHEN** `SigSpec::control()` is called
- **THEN** the result has `kind == SigKind::Generic` with `bandwidth == None` and `target_impedance == None`

#### Scenario: RF 50 ohm preset
- **WHEN** `SigSpec::rf_50ohm()` is called
- **THEN** the result has `kind == SigKind::AnalogLowNoise` and `target_impedance == Some(50.0.ohm())`

