use forge_demo_app::greet;

fn main() {
    println!("Welcome to Forge Demo!");
    greet("World");
    
    // Demonstrate some basic functionality
    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum of {:?} = {}", numbers, sum);
    
    println!("\nThis file is version-controlled by Forge!");
    println!("All changes are stored in Cloudflare R2 as compressed blobs.");
}
