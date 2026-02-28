//! Example: Basic usage of dx-serializer

use serializer::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer Example ===\n");

    // Example 1: Parse simple DX
    println!("1. Parsing Simple DX:");
    let simple_dx = b"name:Alice
age:30
email:alice@example.com
active:+";

    let parsed = parse(simple_dx)?;
    println!("Parsed: {:?}\n", parsed);

    // Example 2: Parse table with schema
    println!("2. Parsing Table with Schema:");
    let table_dx = b"users=id%i name%s score%f active%b
1 Alice 95.5 +
2 Bob 87.3 -
3 Charlie 92.0 +";

    let table_parsed = parse(table_dx)?;
    println!("Parsed table: {:?}\n", table_parsed);

    // Example 3: Human-readable formatting
    println!("3. Human-Readable Format:");
    let human = format_human(&table_parsed)?;
    println!("{}\n", human);

    // Example 4: Round-trip (parse -> encode -> parse)
    println!("4. Round-Trip Test:");
    let encoded = encode(&table_parsed)?;
    println!("Encoded:\n{}", String::from_utf8_lossy(&encoded));
    let reparsed = parse(&encoded)?;
    println!("Round-trip successful: {}\n", table_parsed == reparsed);

    // Example 5: Complex nested structure
    println!("5. Complex Structure with Aliases:");
    let complex_dx = b"$c=config
$c.database.host:localhost
$c.database.port:5432
$c.server.host:0.0.0.0
team>alice|bob|charlie
tasks=id%i name%s hours%f
1 Implementation 12.5
2 Testing 8.0
_ Documentation 6.5";

    let complex = parse(complex_dx)?;
    println!("Complex parsed: {:?}\n", complex);

    let complex_human = format_human(&complex)?;
    println!("Human view:\n{}", complex_human);

    // Example 6: Stream arrays
    println!("\n6. Stream Arrays:");
    let stream_dx = b"tags>rust|wasm|performance|llm
priorities>high|medium|low";

    let streams = parse(stream_dx)?;
    println!("Streams: {:?}\n", streams);

    // Example 7: Ditto compression
    println!("7. Ditto Compression:");
    let ditto_dx = b"logs=time%i status%s code%i
1000 active 200
1001 active 200
1002 active 200";

    let ditto = parse(ditto_dx)?;
    let ditto_encoded = encode(&ditto)?;
    println!("Original: {} bytes", ditto_dx.len());
    println!("Encoded:  {} bytes", ditto_encoded.len());
    println!(
        "Compression: {:.1}%\n",
        (1.0 - ditto_encoded.len() as f64 / ditto_dx.len() as f64) * 100.0
    );

    println!("Encoded output:\n{}", String::from_utf8_lossy(&ditto_encoded));

    Ok(())
}
