// build.rs - Windows resource embedding for single EXE distribution

fn main() {
    // Rerun if assets change
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=build.rs");

    // Windows specific: embed application icon and metadata
    #[cfg(target_os = "windows")]
    windows_resources();
}

#[cfg(target_os = "windows")]
fn windows_resources() {
    use winresource::WindowsResource;
    
    let mut res = WindowsResource::new();
    res.set_icon("assets/app.ico")
        .set("ProductName", "PDF Viewer")
        .set("FileDescription", "PDF Editor Application")
        .set("LegalCopyright", "Copyright 2026")
        .set("ProductVersion", env!("CARGO_PKG_VERSION"));
    
    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=Failed to compile Windows resources: {}", e);
    }
}
