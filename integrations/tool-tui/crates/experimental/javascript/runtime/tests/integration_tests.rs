//! Comprehensive integration tests for dx-js-runtime

#[test]
fn test_standard_library_integration() {
    // RegExp
    use dx_js_runtime::runtime::regexp::RegExp;
    let re = RegExp::new("test", "gi").unwrap();
    assert!(re.test("This is a TEST"));

    // DateTime
    use dx_js_runtime::runtime::datetime::DateTime;
    let now = DateTime::now();
    assert!(now.get_year() >= 2025);

    // URL
    use dx_js_runtime::runtime::url::URL;
    let url = URL::new("https://example.com:8080/path?key=value#hash").unwrap();
    assert_eq!(url.protocol, "https:");
    assert_eq!(url.hostname, "example.com");
    assert_eq!(url.port, "8080");
    assert_eq!(url.pathname, "/path");
    assert_eq!(url.search, "?key=value");
    assert_eq!(url.hash, "#hash");
}

#[test]
fn test_streams_integration() {
    use dx_js_runtime::runtime::streams::*;

    let mut readable = ReadableStream::new();
    readable.push(b"Hello ");
    readable.push(b"World");
    readable.push_end();

    let mut writable = WritableStream::new();
    readable.pipe(&mut writable).unwrap();

    assert_eq!(writable.get_buffer(), b"Hello World");
}

#[test]
fn test_events_integration() {
    use dx_js_runtime::runtime::events::EventEmitter;
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut emitter = EventEmitter::new();
    let counter = Rc::new(RefCell::new(0));
    let counter_clone = counter.clone();

    emitter.on("test", move |_data| {
        *counter_clone.borrow_mut() += 1;
    });

    emitter.emit("test", b"data");
    emitter.emit("test", b"data");

    assert_eq!(*counter.borrow(), 2);
}

#[test]
fn test_child_process_integration() {
    use dx_js_runtime::runtime::child_process::ChildProcess;

    let result = ChildProcess::exec_sync("echo integration_test").unwrap();
    assert!(result.contains("integration_test"));
}

#[test]
fn test_url_search_params() {
    use dx_js_runtime::runtime::url::URL;

    let url = URL::new("https://api.example.com/search?q=rust&page=1&limit=10").unwrap();
    let params = url.search_params();

    assert_eq!(params.get("q"), Some(&"rust".to_string()));
    assert_eq!(params.get("page"), Some(&"1".to_string()));
    assert_eq!(params.get("limit"), Some(&"10".to_string()));
    assert!(params.has("q"));
    assert!(!params.has("missing"));
}

#[test]
fn test_regexp_operations() {
    use dx_js_runtime::runtime::regexp::RegExp;

    let re = RegExp::new("world", "gi").unwrap();
    let result = re.replace("Hello world! World again!", "Rust");
    assert!(result.contains("Rust"));

    let parts = re.split("hello world test world end");
    assert!(parts.len() >= 2);
}

#[test]
fn test_datetime_operations() {
    use dx_js_runtime::runtime::datetime::DateTime;

    let dt = DateTime::from_timestamp(1700000000000);
    let iso = dt.to_iso_string();
    assert!(iso.contains("T"));
    assert!(iso.contains("Z"));

    assert!(dt.get_year() >= 2023);
    assert!(dt.get_month() < 12);
    assert!(dt.get_date() <= 31);
}

#[test]
fn test_transform_stream() {
    use dx_js_runtime::runtime::streams::Transform;

    let mut transform =
        Transform::new(|data: &[u8]| data.iter().map(|b| b.to_ascii_uppercase()).collect());

    let input = b"hello world";
    let output = transform.process(input);

    assert_eq!(output, b"HELLO WORLD");
}

#[test]
fn test_event_emitter_once() {
    use dx_js_runtime::runtime::events::EventEmitter;
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut emitter = EventEmitter::new();
    let counter = Rc::new(RefCell::new(0));
    let counter_clone = counter.clone();

    emitter.once("once_event", move |_data| {
        *counter_clone.borrow_mut() += 1;
    });

    emitter.emit("once_event", b"data");
    emitter.emit("once_event", b"data");

    // Should only increment once
    assert_eq!(*counter.borrow(), 1);
}

#[test]
fn test_util_format() {
    use dx_js_runtime::runtime::util::Util;

    let result = Util::format("Hello %s!", &[&"World"]);
    assert_eq!(result, "Hello World!");
}
