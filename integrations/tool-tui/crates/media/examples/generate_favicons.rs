//! Generate favicons from logo.svg

use dx_media::tools::image::svg::generate_web_icons;

fn main() -> std::io::Result<()> {
    let logo = "apps/www/public/logo.svg";
    let output_dir = "apps/www/public";

    println!("Generating favicons from {}...", logo);

    let result = generate_web_icons(logo, output_dir)?;

    println!("{}", result.message);
    println!("Generated {} files", result.output_paths.len());

    for path in &result.output_paths {
        println!("  - {}", path.display());
    }

    Ok(())
}
