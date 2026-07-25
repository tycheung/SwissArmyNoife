//! Conformance: path install → load wasm → call export (`sak361`).

use std::path::PathBuf;
use std::sync::Mutex;

use module_registry::install_and_pin;
use runtime_wasm::call_add;

static LOCK: Mutex<()> = Mutex::new(());

#[test]
fn install_community_echo_and_call_add() {
    let _g = LOCK.lock().expect("lock");
    let tmp = tempfile::tempdir().expect("tmp");
    std::env::set_var("CONFIG_DIR", tmp.path());

    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/community.echo");
    assert!(
        template.join("manifest.toml").is_file(),
        "missing template at {}",
        template.display()
    );

    let installed = install_and_pin(&template, "path").expect("install");
    assert_eq!(installed.manifest.id, "community.echo");
    assert_eq!(installed.manifest.runtime.as_str(), "wasm");

    let payload = installed.root.join(&installed.manifest.payload);
    let sum = call_add(&payload, 10, 32).expect("wasm add");
    assert_eq!(sum, 42);

    std::env::remove_var("CONFIG_DIR");
}
