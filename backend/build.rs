use std::{env::var, process::Command};

fn main() {
    if let Some(true) = var("APP_ENVIRONMENT").ok().map(|v| &v == "production") {
        let status = Command::new("npm")
            .arg("install")
            .arg("vite")
            .current_dir("../frontend")
            .status()
            .expect("failed to install Vite");

        assert!(status.success())
    }
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir("../frontend")
        .status()
        .expect("failed to build frontend");

    assert!(status.success())
}
