use std::process::Command;

fn main() {
    let status = Command::new("typeshare")
        .arg(".")
        .arg("--lang=typescript")
        .arg("--output-file=../frontend/src/lib/typeshare.ts")
        .status()
        .expect("failed to generate typeshare interfaces");

    assert!(status.success());
}
