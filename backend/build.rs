use std::process::Command;

fn main() {
    let status = Command::new("typeshare")
        .arg(".")
        .arg("--lang=typescript")
        .arg("--output-file=../frontend/src/lib/typeshare.ts")
        .status()
        .expect("failed to run typeshare");

    assert!(status.success());

    let status = Command::new("pnpm")
        .arg("build")
        .current_dir("../frontend")
        .status()
        .expect("failed to run pnpm build");

    assert!(status.success())
}

// jest pewnien problem z build.rs, ponieważ nie wykonuje się on za każdym `cargo run`, a jedynie wtedy, kiedy pod spodem wykona się `cargo build`
