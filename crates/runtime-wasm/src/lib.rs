//! Wasmtime loader + ABI handle with hot-reload (`sak354` / `sak360`).

mod abi;
mod handle;

pub use abi::{
    abi_version_bytes, call_add, call_add_bytes, compile_wat, load_module_bytes, SMOKE_ADD_WAT,
};
pub use handle::{payload_fingerprint, WasmHandle, ABI_VERSION};
