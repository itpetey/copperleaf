use std::fmt;

use thiserror::Error;

use crate::net::{Constraint, Net, NetKind};
use crate::pin::Pin;
use crate::units::Diagnostic;

#[derive(Clone, Debug)]
pub struct CompiledComponent {
    pub refdes: String,
    pub pins: Vec<Pin>,
    pub constraints: Vec<Constraint>,
    pub symbol: Option<String>,
    pub footprint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub component: usize,
    pub pin: String,
    pub net: NetId,
}

use crate::net::NetId;

/// An immutable structure representing a finished [`Board`](crate::Board) that is ready for export.
#[derive(Clone, Debug)]
pub struct CompiledBoard {
    pub components: Vec<CompiledComponent>,
    pub nets: Vec<Net>,
    pub connections: Vec<Connection>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub struct NetInfo {
    pub name: String,
    pub kind: NetKind,
    pub pin_count: usize,
}

#[derive(Clone, Debug)]
pub struct SynthCap {
    pub refdes: String,
    pub value: crate::units::Qty<crate::units::Farad>,
    pub net: String,
    pub source_component: String,
    pub source_pin: String,
}

#[derive(Clone, Debug)]
pub struct CompileSummary {
    pub nets: Vec<NetInfo>,
    pub caps_synthesised: Vec<SynthCap>,
    pub pin_count: usize,
    pub component_count: usize,
}

#[derive(Clone, Debug)]
pub struct CompileReport {
    pub board: CompiledBoard,
    pub warnings: Vec<Diagnostic>,
    pub summary: CompileSummary,
}

#[derive(Clone, Debug, Error)]
pub struct CompileError {
    pub errors: Vec<Diagnostic>,
}

impl CompileError {
    pub fn new(errors: Vec<Diagnostic>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.errors {
            writeln!(f, "[{:?}] {} — {}", e.severity, e.code, e.message)?;
            if let Some(hint) = &e.hint {
                writeln!(f, "         hint: {}", hint)?;
            }
        }
        Ok(())
    }
}
