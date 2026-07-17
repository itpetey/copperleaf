use crate::{Board, NetHandle, PinHandle, compile::CompileError};

/// Connect a list of pins into a single net and return a handle to it.
///
/// # Panics
/// This function panics if number of `pins` is less than 2.
pub fn join(board: &mut Board, pins: &[PinHandle]) -> Result<NetHandle, CompileError> {
    assert!(pins.len() >= 2, "need at least two pins to form a net");

    let first = board.connect(pins[0], pins[1])?;
    for window in pins.windows(3) {
        board.connect(window[1], window[2])?;
    }
    Ok(first)
}

/// Create a power net from a single power pin by self-connecting it.
pub fn pwr_net(board: &mut Board, pin: PinHandle) -> Result<NetHandle, CompileError> {
    Ok(board.connect(pin, pin)?)
}
