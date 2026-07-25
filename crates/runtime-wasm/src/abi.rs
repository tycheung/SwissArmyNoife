//! Wasm compile / instantiate helpers and smoke WAT (`sak354`).

use std::path::Path;

use types::ErrorCode;
use wasmtime::{Engine, Linker, Module, Store};

/// Load wasm or WAT from disk and call exported `add(i32,i32)->i32`.
///
/// # Errors
/// [`ErrorCode::ModuleIncompatible`] / [`ErrorCode::SchemaInvalid`] on load/call failure.
pub fn call_add(wasm_or_wat_path: &Path, a: i32, b: i32) -> Result<i32, ErrorCode> {
    crate::handle::WasmHandle::load(wasm_or_wat_path)?.call_add(a, b)
}

/// Read `.wasm` bytes or compile `.wat` text.
///
/// # Errors
/// I/O → incompatible; WAT parse → schema invalid.
pub fn load_module_bytes(path: &Path) -> Result<Vec<u8>, ErrorCode> {
    let raw = std::fs::read(path).map_err(|_| ErrorCode::ModuleIncompatible)?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wat"))
    {
        let text = String::from_utf8(raw).map_err(|_| ErrorCode::SchemaInvalid)?;
        return compile_wat(&text);
    }
    Ok(raw)
}

/// Same as [`call_add`] from raw wasm bytes.
///
/// # Errors
/// [`ErrorCode::ModuleIncompatible`] on compile/instantiate/call failure.
pub fn call_add_bytes(wasm_bytes: &[u8], a: i32, b: i32) -> Result<i32, ErrorCode> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes).map_err(|_| ErrorCode::ModuleIncompatible)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    let add = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "add")
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    add.call(&mut store, (a, b))
        .map_err(|_| ErrorCode::ModuleIncompatible)
}

/// Read `sak_abi_version` export (must equal host [`ABI_VERSION`](crate::handle::ABI_VERSION)).
///
/// # Errors
/// Missing export or wrong version → incompatible.
pub fn abi_version_bytes(wasm_bytes: &[u8]) -> Result<i32, ErrorCode> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes).map_err(|_| ErrorCode::ModuleIncompatible)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    let f = instance
        .get_typed_func::<(), i32>(&mut store, "sak_abi_version")
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    f.call(&mut store, ())
        .map_err(|_| ErrorCode::ModuleIncompatible)
}

/// Compile WAT text to wasm bytes.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on WAT parse failure.
pub fn compile_wat(wat: &str) -> Result<Vec<u8>, ErrorCode> {
    wat::parse_str(wat).map_err(|_| ErrorCode::SchemaInvalid)
}

/// Canonical smoke WAT used by the community echo template.
pub const SMOKE_ADD_WAT: &str = r#"
(module
  (func (export "sak_abi_version") (result i32)
    i32.const 1)
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::ABI_VERSION;

    #[test]
    fn smoke_add_and_abi() {
        let bytes = compile_wat(SMOKE_ADD_WAT).expect("wat");
        assert_eq!(abi_version_bytes(&bytes).expect("abi"), ABI_VERSION);
        assert_eq!(call_add_bytes(&bytes, 2, 3).expect("call"), 5);
    }
}
