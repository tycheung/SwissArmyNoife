//! `module invoke` — cached wasm handle + hot-reload (`sak360-b`).

use module_registry::{get_installed, ModuleRuntime};
use types::ErrorCode;

pub(super) fn invoke_add(id: &str, a: i32, b: i32) -> Result<i32, ErrorCode> {
    let installed = get_installed(id, None)?;
    let payload = installed.root.join(&installed.manifest.payload);
    ModuleRuntime::new().invoke_add(&payload, a, b)
}
