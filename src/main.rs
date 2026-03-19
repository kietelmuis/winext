use winfsp::{
    host::{DebugMode, FileSystemHost, FileSystemParams, VolumeParams},
    service::FileSystemServiceBuilder,
    winfsp_init,
};

use crate::fs::system::{GoonContext, GoonFs};

mod fs;

fn main() {
    println!("starting winext...");

    let service = FileSystemServiceBuilder::<GoonFs>::new()
        .with_start(|| {
            let mut volume_params = VolumeParams::new();
            volume_params.filesystem_name("ext4");

            let context = GoonContext {};

            let mut host = FileSystemHost::new(volume_params, context).map_err(|e| {
                eprintln!("failed to create fs host: {:?}", e);
                e
            })?;
            host.mount("Z:").unwrap();

            let fs = GoonFs::new(host);
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
