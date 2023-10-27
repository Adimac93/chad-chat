use std::process::Command;

fn main() {
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir("../frontend")
        .status()
        .expect("failed to build frontend");

    assert!(status.success())
}
