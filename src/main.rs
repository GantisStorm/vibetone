mod audio;
mod device;
mod gui;

use anyhow::Result;

fn main() -> Result<()> {
    gui::run()
}
