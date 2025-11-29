fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set("FileDescription", "ImageBox")
            .set("ProductName", "ImageBox")
            .set("LegalCopyright", "© 2025 XeF2")
            .set_icon("assets/favicon.ico");
        res.compile()?;
    }
    Ok(())
}
