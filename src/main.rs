use std::fs::exists;

use crc32_v2::crc32;
use ext4_lwext4::{BlockDevice, FileBlockDevice, MkfsOptions, mkfs};
use log::info;

use crate::{
    disk::DriveBlockDevice,
    fs::system::{WinExtContext, WinExtFs},
};

mod disk;
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

    let device = DriveBlockDevice::open("\\\\.\\Harddisk0Partition2").unwrap();
    let mut buf = vec![0u8; 4096];
    device.read_blocks(0, &mut buf).unwrap();
    let compat = u32::from_le_bytes(buf[1024 + 92..1024 + 96].try_into().unwrap());
    let incompat = u32::from_le_bytes(buf[1024 + 96..1024 + 100].try_into().unwrap());
    let ro_compat = u32::from_le_bytes(buf[1024 + 100..1024 + 104].try_into().unwrap());
    println!(
        "compat={:032b} incompat={:032b} ro_compat={:032b}",
        compat, incompat, ro_compat
    );

    let context = WinExtContext::new(device);

    let mut host = WinExtFs::new(context, crc32(CRC32_INIT, &data) as u32);
    host.host.mount("Z:").unwrap();
    info!("mounting");

    host.host.start().unwrap();
    info!("starting");

    loop {}
}
