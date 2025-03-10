fn main() {
    if cfg!(not(feature = "unit-test")) {
        println!("cargo:rustc-link-arg=-nostartfiles");
        println!("cargo:rustc-link-arg=-static");
    }
}
