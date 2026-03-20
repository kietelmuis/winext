use std::time::{SystemTime, UNIX_EPOCH};

use crc32_v2::crc32;
use ext4_lwext4::{Ext4Fs, FileBlockDevice, MkfsOptions, mkfs};
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

    let service = FileSystemServiceBuilder::<WinExtFs>::new()
        .with_start(|| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let windows_filetime = (now + 11644473600) * 10000000;

            let mut volume_params = VolumeParams::new();
            volume_params.filesystem_name("ext4");
            volume_params.volume_creation_time(windows_filetime);

            let device = FileBlockDevice::create("disk.img", 100 * 1024 * 1024).unwrap();
            let data = std::fs::read("disk.img").unwrap();
            volume_params.volume_serial_number(crc32(CRC32_INIT, &data) as u32);

            mkfs(device, &MkfsOptions::default()).unwrap();

            let device = FileBlockDevice::open("disk.img").unwrap();
            let context = WinExtContext::new(device);

            let mut host = FileSystemHost::new(volume_params, context).map_err(|e| {
                eprintln!("failed to create fs host: {:?}", e);
                e
            })?;
            host.mount("Z:").unwrap();

            let fs = WinExtFs::new(host);
            Ok(fs)
        })
        .with_stop(|_context| {
            println!("shutting down winext");
            Ok(())
        })
        .build("winext", winfsp_init().unwrap())
        .unwrap();

    let handle = service.start();
    println!("service running!");

    handle.join().unwrap().expect("failed to run winext");
}
