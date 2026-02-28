use dx_js_runtime::{DxRuntime, Value};

#[test]
fn test_arithmetic() {
    let mut runtime = DxRuntime::new().unwrap();

    // Addition
    let result = runtime.run_sync("10 + 20", "test.js").unwrap();
    assert_eq!(result, Value::Number(30.0));

    // Subtraction
    let result = runtime.run_sync("100 - 40", "test.js").unwrap();
    assert_eq!(result, Value::Number(60.0));

    // Multiplication
    let result = runtime.run_sync("5 * 6", "test.js").unwrap();
    assert_eq!(result, Value::Number(30.0));

    // Division
    let result = runtime.run_sync("20 / 4", "test.js").unwrap();
    assert_eq!(result, Value::Number(5.0));
}

#[test]
fn test_logic() {
    let mut runtime = DxRuntime::new().unwrap();

    // Less than
    let result = runtime.run_sync("10 < 20", "test.js").unwrap();
    assert_eq!(result, Value::Number(1.0)); // True

    // Greater than
    let result = runtime.run_sync("10 > 20", "test.js").unwrap();
    assert_eq!(result, Value::Number(0.0)); // False
}

#[test]
fn test_variables() {
    let mut runtime = DxRuntime::new().unwrap();

    let src = "
        var a = 10;
        var b = 20;
        a + b
    ";
    let result = runtime.run_sync(src, "test.js").unwrap();
    assert_eq!(result, Value::Number(30.0));
}

#[test]
fn test_math_builtins() {
    let mut runtime = DxRuntime::new().unwrap();

    let result = runtime.run_sync("Math.floor(42.7)", "test.js").unwrap();
    assert_eq!(result, Value::Number(42.0));

    let result = runtime.run_sync("Math.sqrt(16)", "test.js").unwrap();
    assert_eq!(result, Value::Number(4.0));
}
