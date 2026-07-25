//! Registry download → tarball install (`sak364-b`).

use std::fs;
use std::sync::Mutex;

use flate2::write::GzEncoder;
use flate2::Compression;
use module_registry::{install_from_registry, FakeRegistryClient, ModuleRuntime, ResolvedModule};
use tar::Builder;

static LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn registry_download_install_invoke() {
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
    let body = fs::read(&tar_path).unwrap();

    let client = FakeRegistryClient {
        resolved: Some(ResolvedModule {
            id: "community.echo".into(),
            version: "0.1.0".into(),
            download_url: "https://example.com/echo.tgz".into(),
            sha256: None,
        }),
        body,
    };

    let installed = install_from_registry(&client, "community.echo", "0.1.0")
        .await
        .expect("install");
    assert_eq!(installed.manifest.id, "community.echo");
    let payload = installed.root.join(&installed.manifest.payload);
    let sum = ModuleRuntime::new()
        .invoke_add(&payload, 20, 22)
        .expect("add");
    assert_eq!(sum, 42);

    std::env::remove_var("CONFIG_DIR");
}
