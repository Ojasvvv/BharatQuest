use apatheia_engine::{Runtime, RuntimePool};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let base_dir = PathBuf::from(manifest_dir).parent().unwrap().join("wasm-runtimes");
    std::env::set_var("WASM_BINARY_DIR", base_dir);
    
    // Also support quickjs in the original build location temporarily for JS tests
    std::env::set_var("QUICKJS_WASM_PATH", PathBuf::from(manifest_dir).parent().unwrap().join("quickjs-wasm/build/quickjs.wasm"));

    let pool = RuntimePool::init().await.expect("Failed to initialize engine");
    if let Some(module) = pool.python_module.as_ref() {
        println!("PYTHON IMPORTS:");
        for imp in module.imports() {
            println!("  {}:{} {:?}", imp.module(), imp.name(), imp.ty());
        }
    }

    let engine = pool.get(&Runtime::JavaScript).expect("JavaScript runtime not found");

    let inputs = [
        ("Empty string", ""),
        ("Literal", "1"),
        ("Trivial expression", "1+1"),
        ("Console.log", "console.log('hello')"),
        (
            "Loop work",
            "let s=0; for(let i=0;i<10000;i++){s+=i;} console.log(s);",
        ),
    ];

    println!("======================================================================================================================");
    println!("{:<20} | {:<5} | {:<18} | {:<18} | {:<15} | {:<15}", "Input Type", "Run", "Clone Time (us)", "Eval Time (us)", "Total Time (us)", "Fuel Consumed");
    println!("----------------------------------------------------------------------------------------------------------------------");

    for (name, source) in inputs {
        for run in 1..=5 {
            let result = engine.execute(source, 50_000_000, 5000, 256).await.expect("Execution failed");
            let m = result.metrics;
            println!(
                "{:<20} | {:<5} | {:<18} | {:<18} | {:<15} | {:<15}",
                name, run, m.instance_clone_time_us, m.execution_time_us, m.total_time_us, m.fuel_consumed
            );
        }
        println!("----------------------------------------------------------------------------------------------------------------------");
    }
}
