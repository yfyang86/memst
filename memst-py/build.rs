fn main() {
    println!("cargo:rerun-if-changed=pyproject.toml");

    // On macOS, Python extension modules typically rely on symbols provided by the
    // host Python runtime. Allow unresolved Python C-API symbols at link time.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
}
