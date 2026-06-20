use wasmtime::*;
fn main() {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "wasm-runtimes/micropython-wasi.wasm").unwrap();
    for import in module.imports() {
        println!("{}:{} {:?}", import.module(), import.name(), import.ty());
    }
}
