//! Windows executable metadata: the displayed name is "Pinewatch".

fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set("ProductName", "Pinewatch")
            .set("FileDescription", "Pinewatch")
            .set("InternalName", "pinewatch")
            .set("OriginalFilename", "pinewatch.exe");
        if let Err(error) = resource.compile() {
            println!("cargo::warning=Windows resource compile failed, executable will lack its display name: {error}");
        }
    }
}
