use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// Helper function to set up a test directory with markdown files
fn setup_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create some test markdown files
    let test_md = dir.path().join("test.md");
    let mut file = fs::File::create(&test_md).unwrap();
    writeln!(
        file,
        "# Test Document\n\nThis is a [link](other.md) to another document."
    )
    .unwrap();

    let other_md = dir.path().join("other.md");
    let mut file = fs::File::create(&other_md).unwrap();
    writeln!(
        file,
        "# Other Document\n\nThis is a [link](test.md) back to the first document."
    )
    .unwrap();

    // Create a subdirectory with more files
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let sub_md = subdir.join("sub.md");
    let mut file = fs::File::create(&sub_md).unwrap();
    writeln!(
        file,
        "# Subdirectory Document\n\nThis is a [link](../test.md) to the root document."
    )
    .unwrap();

    // Create a non-markdown file
    let text_file = dir.path().join("text.txt");
    let mut file = fs::File::create(&text_file).unwrap();
    writeln!(file, "This is a plain text file.").unwrap();

    dir
}

// Helper function to verify the output directory has the expected files
fn verify_output_dir(output_dir: &Path) {
    // Check HTML conversions
    assert!(output_dir.join("test.html").exists());
    assert!(output_dir.join("other.html").exists());
    assert!(output_dir.join("subdir").exists());
    assert!(output_dir.join("subdir/sub.html").exists());

    // Check non-markdown file copying
    assert!(output_dir.join("text.txt").exists());

    // Check content of converted files
    let test_content = fs::read_to_string(output_dir.join("test.html")).unwrap();
    assert!(test_content.contains("<a href=\"other.html\">link</a>"));

    let other_content = fs::read_to_string(output_dir.join("other.html")).unwrap();
    assert!(other_content.contains("<a href=\"test.html\">link</a>"));

    let sub_content = fs::read_to_string(output_dir.join("subdir/sub.html")).unwrap();
    assert!(sub_content.contains("<a href=\"../test.html\">link</a>"));

    let text_content = fs::read_to_string(output_dir.join("text.txt")).unwrap();
    assert_eq!(text_content, "This is a plain text file.\n");
}

// Test running the command with default options
#[test]
fn test_default_run() {
    // Set up test directory
    let input_dir = setup_test_dir();
    let output_dir = TempDir::new().unwrap();

    // Build the project (this assumes we're running tests with cargo)
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build the project");

    assert!(status.success());

    // Run the program
    let status = Command::new("target/debug/choad")
        .args([
            input_dir.path().to_str().unwrap(),
            "-d",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run the program");

    assert!(status.success());

    // Verify output
    verify_output_dir(output_dir.path());
}

// Test with custom options
#[test]
fn test_custom_source_and_output() {
    // Set up test directory
    let input_dir = setup_test_dir();
    let output_dir = TempDir::new().unwrap();

    // Build the project
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build the project");

    assert!(status.success());

    // Run the program with custom source and output
    let status = Command::new("target/debug/choad")
        .args([
            input_dir.path().to_str().unwrap(),
            "-d",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run the program");

    assert!(status.success());

    // Verify output
    verify_output_dir(output_dir.path());
}

// Note: Testing the watch mode is more complex and typically requires
// a different approach, as it runs in an infinite loop.
// A complete test would use a mock file watcher or a timeout mechanism.
