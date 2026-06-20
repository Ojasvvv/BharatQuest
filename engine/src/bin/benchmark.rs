use apatheia_engine::sandbox::{SandboxConfig, SandboxEngine};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let wasm_path = PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .join("quickjs-wasm/build/quickjs.wasm");

    let config = SandboxConfig {
        wasm_path,
        fuel_limit: 50_000_000,
        wall_clock_timeout: Duration::from_secs(5),
        max_memory_bytes: 256 * 1024 * 1024,
    };

    let engine = SandboxEngine::new(config).expect("Failed to initialize engine");

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
            let result = engine.execute(source).await.expect("Execution failed");
            let m = result.metrics;
            println!(
                "{:<20} | {:<5} | {:<18} | {:<18} | {:<15} | {:<15}",
                name, run, m.instantiation_us, m.eval_us, m.total_request_us, m.fuel_consumed
            );
        }
        println!("----------------------------------------------------------------------------------------------------------------------");
    }
}
