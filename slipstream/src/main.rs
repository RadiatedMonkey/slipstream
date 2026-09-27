#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slipstream_shared::error::SlipstreamResult;

fn main() -> SlipstreamResult<()> {
    slipstream::run()
}
