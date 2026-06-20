use apatheia_engine::{RuntimePool, ExecutionResult};
use std::time::Instant;

#[tokio::test]
async fn test_concurrent_fetch() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let wasm_dir = std::path::Path::new(&manifest_dir).parent().unwrap().join("wasm-runtimes");
    std::env::set_var("WASM_BINARY_DIR", wasm_dir.to_str().unwrap());
    
    std::env::set_var("FETCH_ENABLED", "true");
    
    // Initialize the pool
    let pool = RuntimePool::init().await.unwrap();
    let pool = std::sync::Arc::new(pool);
    
    // We want 20 concurrent executions using JavaScript Reactor.
    let concurrency = 20;
    
    let js_code = r#"
        let html = fetch("http://example.com");
        if (!html.includes("Example Domain")) {
            throw new Error("Fetch failed or invalid content");
        }
        console.log("Success");
    "#;
    
    let runtime = apatheia_engine::Runtime::JavaScript;
    
    let start_time = Instant::now();
    let mut futures = Vec::new();
    
    for _ in 0..concurrency {
        let pool_clone = pool.clone();
        let code = js_code.to_string();
        let rt = runtime.clone();
        
        futures.push(tokio::spawn(async move {
            pool_clone.execute(&rt, &code, 100_000_000, 5000, 64).await
        }));
    }
    
    let mut results = Vec::new();
    for handle in futures {
        results.push(handle.await.unwrap());
    }
    
    let elapsed = start_time.elapsed();
    
    let mut success_count = 0;
    for res in results {
        let ex_res = res.expect("Task panicked");
        if ex_res.error.is_none() && ex_res.stdout.trim() == "Success" {
            success_count += 1;
        } else {
            panic!("Expected Success, got error: {:?}, stdout: {}", ex_res.error, ex_res.stdout);
        }
    }
    
    assert_eq!(success_count, concurrency);
    
    println!("--- CONCURRENCY TEST REPORT ---");
    println!("Total Wall-Clock Time for {} concurrent requests: {:?}", concurrency, elapsed);
    println!("If the time is roughly equivalent to a single HTTP request (e.g., 100-500ms), then genuine parallelism is achieved via the tokio blocking pool.");
    println!("If the time scales linearly (e.g., 20 * 300ms = 6+ seconds), the execution is secretly serial.");
    
    // Typically a single request to example.com takes ~100-300ms.
    // We expect 20 concurrent requests to take LESS than 2 seconds.
    // If it were secretly serial, it would take ~4-6 seconds minimum.
    assert!(elapsed.as_secs_f64() < 3.0, "Execution appears serialized! Took {:?}", elapsed);
}
