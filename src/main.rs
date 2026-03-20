use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crc32_v2::crc32;
use ext4_lwext4::{FileBlockDevice, MkfsOptions, mkfs};
use winfsp::{
    host::{FileSystemHost, VolumeParams},
    service::FileSystemServiceBuilder,
    winfsp_init,
};

use crate::fs::system::{WinExtContext, WinExtFs};

mod fs;

const CRC32_INIT: u32 = 0;

fn main() {
    println!("starting winext...");

    let device = FileBlockDevice::create("disk.img", 100 * 1024 * 1024).unwrap();
    let data = std::fs::read("disk.img").unwrap();
    mkfs(device, &MkfsOptions::default()).unwrap();

    let device = FileBlockDevice::open("disk.img").unwrap();
    let context = WinExtContext::new(device);

    let mut host = WinExtFs::new(context, crc32(CRC32_INIT, &data) as u32);
    host.host.mount("Z:").unwrap();
    host.host.start().unwrap();

    loop {}
}
