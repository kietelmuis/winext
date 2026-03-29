use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use log::{error, info};

use crate::{
    disk::DriveBlockDevice,
    fs::system::{WinExtContext, WinExtFs},
};

mod disk;
mod fs;
mod util;

fn main() {
    env_logger::Builder::new()
        .filter_module("winext", log::LevelFilter::Debug)
        .init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    info!("initializing");

    let device = match DriveBlockDevice::open("\\\\.\\Harddisk0Partition2") {
        Ok(dev) => Arc::new(dev),
        Err(e) => {
            error!("failed to open device: {:?}", e);
            return;
        }
    };
    let context = WinExtContext::new(device);

    info!("mounting...");
    let mut host = WinExtFs::new(context);
    match host.host.mount("Z:") {
        Ok(_) => info!("mounted!"),
        Err(e) => error!("mount failed: {:?}", e),
    }

    info!("starting...");
    match host.host.start() {
        Ok(_) => info!("started!"),
        Err(e) => error!("start failed: {:?}", e),
    }

    ctrlc::set_handler(move || {
        info!("shutting down...");
        host.host.unmount();
        host.host.stop();

        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}
}
