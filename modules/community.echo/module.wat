;; Community wasm template — ABI v1 + add (sak359 / sak354-b).
(module
  (func (export "sak_abi_version") (result i32)
    i32.const 1)
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
