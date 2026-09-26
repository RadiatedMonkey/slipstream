#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slipstream::error::SlipstreamResult;

fn main() -> SlipstreamResult<()> {
    slipstream::run()
}
