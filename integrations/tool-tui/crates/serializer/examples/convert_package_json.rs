/// Practical example: Convert package.json to DX ULTRA
use serializer::json_to_dx;

const PACKAGE_JSON: &str = r#"{
  "name": "awesome-app",
  "version": "2.0.1",
  "description": "My awesome application",
  "author": "John Doe <john@example.com>",
  "license": "MIT",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "zustand": "^4.5.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "vite": "^5.0.0",
    "vitest": "^1.0.0"
  }
}"#;

fn main() {
    println!("===========================================");
    println!("  DX ULTRA: JSON → DX SINGULARITY");
    println!("===========================================\n");

    println!("INPUT (package.json) - {} bytes:", PACKAGE_JSON.len());
    println!("{}\n", PACKAGE_JSON);

    match json_to_dx(PACKAGE_JSON) {
        Ok(dx) => {
            println!("OUTPUT (package.dx) - {} bytes:", dx.len());
            println!("{}\n", dx);

            let savings = PACKAGE_JSON.len() - dx.len();
            let percent = (savings as f64 / PACKAGE_JSON.len() as f64) * 100.0;

            println!("===========================================");
            println!("  COMPRESSION RESULTS");
            println!("===========================================");
            println!("Original (JSON):  {} bytes", PACKAGE_JSON.len());
            println!("DX ULTRA:         {} bytes", dx.len());
            println!("Savings:          {} bytes ({:.1}%)", savings, percent);
            println!(
                "\n✨ DX ULTRA is {:.1}x smaller!",
                PACKAGE_JSON.len() as f64 / dx.len() as f64
            );
        }
        Err(e) => {
            eprintln!("❌ Conversion failed: {}", e);
        }
    }
}
