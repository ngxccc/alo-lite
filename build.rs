//! Build script for Alolite Linux.

fn main() {
    println!("cargo:rerun-if-changed=src/assets/app_icon.png");
}
