use apatheia_engine::{Runtime, RuntimePool};

async fn get_python_pool() -> RuntimePool {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let base_dir = std::path::PathBuf::from(manifest_dir).parent().unwrap().join("wasm-runtimes");
    std::env::set_var("WASM_BINARY_DIR", &base_dir);
    eprintln!("WASM_BINARY_DIR set to: {:?}", base_dir);
    RuntimePool::init().await.expect("Failed to init pool")
}

#[tokio::test]
async fn test_python_basic_stdout() {
    let pool = get_python_pool().await;
    let result = pool.execute(&Runtime::Python, "print(2+2)", 1_000_000, 1000, 64).await.unwrap();
    assert_eq!(result.status_code, 0);
    assert_eq!(result.stdout, "4\n");
    assert_eq!(result.stderr, "");
}

#[tokio::test]
async fn test_python_sandbox_isolation() {
    let pool = get_python_pool().await;
    let result = pool.execute(&Runtime::Python, "open('/etc/passwd')", 1_000_000, 1000, 64).await.unwrap();
    // MicroPython may not have 'open' or it raises OSError
    assert_ne!(result.status_code, 0);
    assert!(result.stderr.contains("Error") || result.stderr.contains("Exception"));
}

#[tokio::test]
async fn test_python_runtime_error() {
    let pool = get_python_pool().await;
    let result = pool.execute(&Runtime::Python, "[1].bad()", 1_000_000, 1000, 64).await.unwrap();
    assert_ne!(result.status_code, 0);
    assert!(result.stderr.contains("AttributeError"));
    let err = result.error.expect("Should have error");
    assert_eq!(err.error_type, apatheia_engine::error::JsErrorType::Runtime);
}

#[tokio::test]
async fn test_python_syntax_error() {
    let pool = get_python_pool().await;
    let result = pool.execute(&Runtime::Python, "def bad( return", 1_000_000, 1000, 64).await.unwrap();
    assert_ne!(result.status_code, 0);
    assert!(result.stderr.contains("SyntaxError"));
}

#[tokio::test]
async fn test_python_fuel_exhaustion() {
    let pool = get_python_pool().await;
    // MicroPython at 370KB has a lot of startup code.
    // 1_000_000 is enough to start, but will trap on infinite loop.
    let err = pool.execute(&Runtime::Python, "while True: pass", 1_000_000, 1000, 64).await.unwrap_err();
    match err {
        apatheia_engine::error::EngineError::FuelExhausted { .. } => {}
        _ => panic!("Expected FuelExhausted"),
    }
}

#[tokio::test]
async fn test_python_execution_metrics_and_notes() {
    let pool = get_python_pool().await;
    let result = pool.execute(&Runtime::Python, "print(100)", 1_000_000, 1000, 64).await.unwrap();
    assert!(result.runtime_notes.unwrap().contains("MicroPython"));
    let m = result.metrics;
    assert!(m.total_time_us >= m.execution_time_us + m.instance_clone_time_us);
}
