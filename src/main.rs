use clap::Parser;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

/// Command-line arguments structure using `clap`.
#[derive(Parser, Debug)]
#[command(author = "Astra1", version, about = "Comically Hyper-Optimizing All Docs", long_about = None)]
struct Args {
    /// Source directory (defaults to 'docs')
    #[arg(default_value = "docs")]
    source: String,

    /// Output directory (defaults to 'site')
    #[arg(short = 'd', long = "site-dir", default_value = "site")]
    output: String,

    /// Enable watch mode
    #[arg(short = 'w', long = "watch")]
    watch: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Ensure output folder exists
    if !Path::new(&args.output).exists() {
        fs::create_dir(&args.output)?;
    }

    if args.watch {
        println!(
            "Watching for changes in '{}'. Press Ctrl+C to stop.",
            args.source
        );
        watch_mode(&args.source, &args.output)?;
    } else {
        process_files(&args.source, &args.output)?;
        println!("Process complete!");
    }

    Ok(())
}

/// Process `.md` files in the given source directory
fn process_files(input_dir: &str, output_dir: &str) -> io::Result<()> {
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    // Convert `.md` files to `.html`
                    process_markdown_file(&path, output_dir)?;
                } else {
                    // Copy other files as is
                    copy_file(&path, output_dir)?;
                }
            } else {
                // Files without extensions, copy as is
                copy_file(&path, output_dir)?;
            }
        } else if path.is_dir() {
            // Recursively process subdirectories
            let dir_name = match path.file_name() {
                Some(name) => name,
                None => {
                    eprintln!("Warning: Invalid directory name for {}", path.display());
                    continue;
                }
            };

            let new_output_dir = PathBuf::from(output_dir).join(dir_name);

            if !new_output_dir.exists() {
                fs::create_dir(&new_output_dir)?;
            }

            let new_input_dir = match path.to_str() {
                Some(dir) => dir,
                None => {
                    eprintln!("Warning: Invalid path encoding for {}", path.display());
                    continue;
                }
            };

            let new_output_dir = match new_output_dir.to_str() {
                Some(dir) => dir,
                None => {
                    eprintln!(
                        "Warning: Invalid output path encoding for {}",
                        new_output_dir.display()
                    );
                    continue;
                }
            };

            process_files(new_input_dir, new_output_dir)?;
        }
    }

    Ok(())
}

/// Process a single markdown file, converting it to HTML
fn process_markdown_file(path: &Path, output_dir: &str) -> io::Result<()> {
    let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(stem) => stem,
        None => {
            eprintln!("Warning: Invalid file name for {}", path.display());
            return Ok(());
        }
    };

    let output_path = Path::new(output_dir).join(format!("{}.html", file_stem));

    // Read the contents of the markdown file
    let content = fs::read_to_string(path)?;

    // Convert markdown links to HTML links
    let html_content = format!("<pre>{}</pre>", convert_markdown_links(&content));

    // Write the converted content to the output file
    fs::write(&output_path, html_content)?;

    println!("Converted: {} -> {}", path.display(), output_path.display());

    Ok(())
}

/// Copy a file from source to destination directory
fn copy_file(path: &Path, output_dir: &str) -> io::Result<()> {
    let file_name = match path.file_name() {
        Some(name) => name,
        None => {
            eprintln!("Warning: Invalid file name for {}", path.display());
            return Ok(());
        }
    };

    let output_path = Path::new(output_dir).join(file_name);

    fs::copy(path, &output_path)?;
    println!("Copied: {} -> {}", path.display(), output_path.display());

    Ok(())
}

/// Convert Markdown links [text](url) to HTML <a href="url">text</a> tags
fn convert_markdown_links(content: &str) -> String {
    // Regular expression to match markdown links: [text](url) or ![text](url)
    // Note: In a production app, this regex should be compiled once using
    // the lazy_static or once_cell crate for better performance
    let re = Regex::new(r"\!?\[([^\]]+)\]\(([^)]+)\)").unwrap();

    // Replace each markdown link with an HTML link
    // Convert .md extensions to .html in the URLs
    re.replace_all(content, |caps: &regex::Captures| {
        let text = &caps[1];
        let url = &caps[2];

        // Check if the URL ends with .md and replace with .html if it does
        let final_url = if url.ends_with(".md") {
            format!("{}.html", &url[0..url.len() - 3])
        } else {
            url.to_string()
        };

        format!(r#"<a href="{}">{}</a>"#, final_url, text)
    })
    .to_string()
}

/// Watch mode to detect file changes
fn watch_mode(input_dir: &str, output_dir: &str) -> io::Result<()> {
    let (tx, rx) = channel();

    // Better error handling for watcher creation
    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error creating file watcher: {:?}", e);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Watcher creation failed",
            ));
        }
    };

    // Better error handling for setting up the watcher
    if let Err(e) = watcher.watch(Path::new(input_dir), RecursiveMode::Recursive) {
        eprintln!("Error watching directory: {:?}", e);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Directory watch failed",
        ));
    }

    for res in rx {
        match res {
            Ok(event) => {
                let Event { paths, .. } = event;
                for path in paths {
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                        match process_files(input_dir, output_dir) {
                            Ok(_) => println!("Processed changes triggered by: {}", path.display()),
                            Err(e) => eprintln!("Error processing {}: {}", path.display(), e),
                        }

                        // Break after processing once to avoid multiple processing for the same set of changes
                        break;
                    }
                }
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    // Test the markdown link conversion function
    #[test]
    fn test_convert_markdown_links() {
        // Test basic link conversion
        let input = "This is a [link](https://example.com) in text.";
        let expected = "This is a <a href=\"https://example.com\">link</a> in text.";
        assert_eq!(convert_markdown_links(input), expected);

        // Test .md to .html conversion
        let input = "Check [this page](other.md) for more information.";
        let expected = "Check <a href=\"other.html\">this page</a> for more information.";
        assert_eq!(convert_markdown_links(input), expected);

        // Test image links (should be converted too as per current implementation)
        let input = "An image: ![alt text](image.png)";
        let expected = "An image: <a href=\"image.png\">alt text</a>";
        assert_eq!(convert_markdown_links(input), expected);

        // Test multiple links
        let input = "[Link 1](url1.md) and [Link 2](url2)";
        let expected = "<a href=\"url1.html\">Link 1</a> and <a href=\"url2\">Link 2</a>";
        assert_eq!(convert_markdown_links(input), expected);
    }

    // Test processing a markdown file
    #[test]
    fn test_process_markdown_file() -> io::Result<()> {
        // Create temp directories
        let input_dir = tempdir()?;
        let output_dir = tempdir()?;

        // Create a temp markdown file
        let md_file_path = input_dir.path().join("test.md");
        let mut md_file = fs::File::create(&md_file_path)?;
        writeln!(md_file, "# Test\n\nThis is a [link](other.md).")?;

        // Process the file
        process_markdown_file(&md_file_path, output_dir.path().to_str().unwrap())?;

        // Check the output file
        let output_path = output_dir.path().join("test.html");
        assert!(output_path.exists());

        let content = fs::read_to_string(output_path)?;
        assert!(content.contains("<a href=\"other.html\">link</a>"));

        Ok(())
    }

    // Test copying a non-markdown file
    #[test]
    fn test_copy_file() -> io::Result<()> {
        // Create temp directories
        let input_dir = tempdir()?;
        let output_dir = tempdir()?;

        // Create a temp file
        let file_path = input_dir.path().join("test.txt");
        let mut file = fs::File::create(&file_path)?;
        writeln!(file, "This is a test file.")?;

        // Copy the file
        copy_file(&file_path, output_dir.path().to_str().unwrap())?;

        // Check the output file
        let output_path = output_dir.path().join("test.txt");
        assert!(output_path.exists());

        let content = fs::read_to_string(output_path)?;
        assert_eq!(content, "This is a test file.\n");

        Ok(())
    }

    // Test processing a directory structure
    #[test]
    fn test_process_files() -> io::Result<()> {
        // Create temp directories
        let input_dir = tempdir()?;
        let output_dir = tempdir()?;

        // Create a nested directory structure
        let nested_dir = input_dir.path().join("nested");
        fs::create_dir(&nested_dir)?;

        // Create files
        let md_file = input_dir.path().join("test.md");
        let mut file = fs::File::create(&md_file)?;
        writeln!(file, "# Test\n\nThis is a [link](other.md).")?;

        let txt_file = input_dir.path().join("test.txt");
        let mut file = fs::File::create(&txt_file)?;
        writeln!(file, "This is a text file.")?;

        let nested_md_file = nested_dir.join("nested.md");
        let mut file = fs::File::create(&nested_md_file)?;
        writeln!(file, "# Nested\n\nThis is a [link](../test.md).")?;

        // Process the directory
        process_files(
            input_dir.path().to_str().unwrap(),
            output_dir.path().to_str().unwrap(),
        )?;

        // Check output structure
        assert!(output_dir.path().join("test.html").exists());
        assert!(output_dir.path().join("test.txt").exists());
        assert!(output_dir.path().join("nested").exists());
        assert!(output_dir.path().join("nested/nested.html").exists());

        // Check file content
        let content = fs::read_to_string(output_dir.path().join("test.html"))?;
        assert!(content.contains("<a href=\"other.html\">link</a>"));

        let content = fs::read_to_string(output_dir.path().join("nested/nested.html"))?;
        assert!(content.contains("<a href=\"../test.html\">link</a>"));

        Ok(())
    }
}
