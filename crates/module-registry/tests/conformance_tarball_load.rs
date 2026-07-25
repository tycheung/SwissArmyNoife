//! Conformance: tarball install → wasm add (`sak361-b`).

use std::fs;
use std::sync::Mutex;

use flate2::write::GzEncoder;
use flate2::Compression;
use module_registry::install_tarball_and_pin;
use runtime_wasm::WasmHandle;
use tar::Builder;

static LOCK: Mutex<()> = Mutex::new(());

#[test]
fn tarball_install_then_invoke_add() {
    let _g = LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("CONFIG_DIR", tmp.path().join("cfg"));

    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/community.echo");
    let tar_path = tmp.path().join("echo.tar.gz");
    {
        let file = fs::File::create(&tar_path).unwrap();
        let enc = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(enc);
        builder.append_dir_all("community.echo", &src).expect("tar");
        builder.finish().unwrap();
    }

    let installed = install_tarball_and_pin(&tar_path).expect("install");
    assert_eq!(installed.manifest.id, "community.echo");
    let payload = installed.root.join(&installed.manifest.payload);
    let sum = WasmHandle::load(&payload)
        .expect("load")
        .call_add(20, 22)
        .expect("add");
    assert_eq!(sum, 42);

    std::env::remove_var("CONFIG_DIR");
}
