use std::fs::exists;

use crc32_v2::crc32;
use ext4_lwext4::{FileBlockDevice, MkfsOptions, mkfs};
use log::info;

use crate::fs::system::{WinExtContext, WinExtFs};

mod fs;

const CRC32_INIT: u32 = 0;

fn main() {
    env_logger::Builder::new()
        .filter_module("winext", log::LevelFilter::Debug)
        .init();

    info!("initializing");

    if !exists("disk.img").unwrap() {
        let device = FileBlockDevice::create("disk.img", 1000 * 1024 * 1024).unwrap();
        mkfs(device, &MkfsOptions::default()).unwrap();
    }

    let data = std::fs::read("disk.img").unwrap();

    let device = FileBlockDevice::open("disk.img").unwrap();
    let context = WinExtContext::new(device);

    let mut host = WinExtFs::new(context, crc32(CRC32_INIT, &data) as u32);
    host.host.mount("Z:").unwrap();
    info!("mounting");

    host.host.start().unwrap();
    info!("starting");

    loop {}
}
