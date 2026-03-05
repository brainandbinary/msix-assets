use anyhow::{Context, Result};
use clap::{App, Arg};
use colored::*;
use image::{
    imageops, DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Rgba,
};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let matches = App::new("MSIX Icon Generator")
        .version("1.0")
        .author("Your Name")
        .about("Generates icons of specific sizes from a high-resolution source image")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .value_name("FILE")
                .help("High-resolution source image")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .value_name("FOLDER")
                .help("Folder containing reference images (with desired names and sizes)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FOLDER")
                .help("Output folder (default: ./generated_icons)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .value_name("FORMAT")
                .help("Output image format (png, jpg, etc.) – if not set, preserves original extension")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .possible_values(&["contain", "cover", "stretch"])
                .default_value("contain")
                .help("How to fit the source image into target dimensions (contain = pad with transparency, cover = crop, stretch = distort)")
                .takes_value(true),
        )
        .get_matches();

    let source_path = Path::new(matches.value_of("source").unwrap());
    let target_path = Path::new(matches.value_of("target").unwrap());
    let output_path = match matches.value_of("output") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from("./generated_icons"),
    };
    let output_format_override = matches.value_of("format").map(|s| s.to_lowercase());
    let mode = matches.value_of("mode").unwrap(); // "contain", "cover", or "stretch"

    // Load source image
    println!("{} Loading source image...", "[1/5]".bright_green());
    let source_img = image::open(source_path)
        .with_context(|| format!("Failed to open source image: {}", source_path.display()))?;
    println!(
        "Source image: {} x {}",
        source_img.width().to_string().bright_yellow(),
        source_img.height().to_string().bright_yellow()
    );

    // Create output directory
    println!("{} Creating output directory...", "[2/5]".bright_green());
    fs::create_dir_all(&output_path)
        .with_context(|| format!("Failed to create output directory: {}", output_path.display()))?;

    // Scan target folder for images
    println!("{} Scanning target folder...", "[3/5]".bright_green());
    let image_files = scan_images(target_path)?;
    if image_files.is_empty() {
        println!("{} No image files found in target folder.", "ERROR".bright_red());
        return Ok(());
    }

    // Collect and display image info
    println!("\n{} Found {} reference images:", "[4/5]".bright_green(), image_files.len());
    let mut images_info = Vec::new();
    for (i, path) in image_files.iter().enumerate() {
        let img = image::open(path)?;
        let dims = (img.width(), img.height());
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        images_info.push((path.clone(), name.clone(), dims));
        println!(
            "  {}. {} - {} x {}",
            i + 1,
            name.bright_cyan(),
            dims.0.to_string().bright_yellow(),
            dims.1.to_string().bright_yellow()
        );
    }

    // Show the selected mode
    println!("\nMode: {} (use --mode to change)", mode.bright_cyan());

    // Ask for confirmation
    println!("\n{} Generate these icons? (y/n): ", "[5/5]".bright_green());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        println!("{} Operation cancelled.", "CANCELLED".bright_yellow());
        return Ok(());
    }

    // Generate icons
    println!("\n{} Generating icons...", "Progress".bright_green());
    generate_icons(
        &source_img,
        &images_info,
        &output_path,
        output_format_override.as_deref(),
        mode,
    )?;

    println!(
        "\n{} Done! Icons saved to: {}",
        "SUCCESS".bright_green(),
        output_path.display().to_string().bright_cyan()
    );

    Ok(())
}

/// Scan a folder for image files (common extensions)
fn scan_images(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut images = Vec::new();
    let valid_extensions = ["png", "jpg", "jpeg", "bmp", "gif", "ico", "tiff", "tif", "webp"];

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if valid_extensions.contains(&ext.to_lowercase().as_str()) {
                images.push(path);
            }
        }
    }
    Ok(images)
}

/// Generate resized icons according to the chosen mode
fn generate_icons(
    source: &DynamicImage,
    images_info: &[(PathBuf, String, (u32, u32))],
    output_dir: &Path,
    output_format_override: Option<&str>,
    mode: &str,
) -> Result<()> {
    for (i, (original_path, name, (target_w, target_h))) in images_info.iter().enumerate() {
        println!(
            "  Generating {} ({}/{}) → {} x {}  [mode: {}]",
            name.bright_cyan(),
            i + 1,
            images_info.len(),
            target_w.to_string().bright_yellow(),
            target_h.to_string().bright_yellow(),
            mode.bright_magenta()
        );

        let result_img = match mode {
            "stretch" => {
                // Just stretch to exact dimensions (may distort)
                source.resize_exact(*target_w, *target_h, imageops::FilterType::Lanczos3)
            }
            "contain" => {
                // Resize to fit inside target, preserving aspect ratio, then paste onto transparent canvas
                let scale = f64::min(
                    *target_w as f64 / source.width() as f64,
                    *target_h as f64 / source.height() as f64,
                );
                let new_w = (source.width() as f64 * scale).round() as u32;
                let new_h = (source.height() as f64 * scale).round() as u32;

                // Resize source to the scaled dimensions
                let resized = source.resize_exact(new_w, new_h, imageops::FilterType::Lanczos3);

                // Create a transparent canvas of the target size
                let mut canvas = ImageBuffer::from_pixel(*target_w, *target_h, Rgba([0, 0, 0, 0]));

                // Calculate position to center the resized image
                let x = (*target_w - new_w) / 2;
                let y = (*target_h - new_h) / 2;

                // Overlay the resized image onto the canvas – cast coordinates to i64
                imageops::overlay(&mut canvas, &resized.to_rgba8(), x.into(), y.into());

                // Convert canvas back to DynamicImage
                DynamicImage::ImageRgba8(canvas)
            }
            "cover" => {
                // Resize to cover target dimensions (scale then crop)
                let scale = f64::max(
                    *target_w as f64 / source.width() as f64,
                    *target_h as f64 / source.height() as f64,
                );
                let new_w = (source.width() as f64 * scale).round() as u32;
                let new_h = (source.height() as f64 * scale).round() as u32;

                // Resize source to the scaled dimensions
                let resized = source.resize_exact(new_w, new_h, imageops::FilterType::Lanczos3);

                // Calculate crop region (center)
                let x = (new_w - *target_w) / 2;
                let y = (new_h - *target_h) / 2;

                // Crop to target size
                resized.crop_imm(x, y, *target_w, *target_h)
            }
            _ => unreachable!(), // clap restricts to these three
        };

        // Determine output format
        let format = if let Some(fmt) = output_format_override {
            match fmt {
                "png" => ImageFormat::Png,
                "jpg" | "jpeg" => ImageFormat::Jpeg,
                "bmp" => ImageFormat::Bmp,
                "gif" => ImageFormat::Gif,
                "ico" => ImageFormat::Ico,
                "tiff" => ImageFormat::Tiff,
                "webp" => ImageFormat::WebP,
                _ => {
                    eprintln!("Warning: Unsupported format '{}', falling back to PNG", fmt);
                    ImageFormat::Png
                }
            }
        } else {
            // Preserve original format based on file extension
            let ext = original_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_lowercase();
            match ext.as_str() {
                "png" => ImageFormat::Png,
                "jpg" | "jpeg" => ImageFormat::Jpeg,
                "bmp" => ImageFormat::Bmp,
                "gif" => ImageFormat::Gif,
                "ico" => ImageFormat::Ico,
                "tiff" | "tif" => ImageFormat::Tiff,
                "webp" => ImageFormat::WebP,
                _ => ImageFormat::Png, // fallback
            }
        };

        // Construct output path
        let output_file = if output_format_override.is_some() {
            let stem = original_path.file_stem().unwrap().to_string_lossy();
            output_dir.join(format!("{}.{}", stem, output_format_override.unwrap()))
        } else {
            output_dir.join(name)
        };

        result_img.save_with_format(&output_file, format)?;
    }
    Ok(())
}