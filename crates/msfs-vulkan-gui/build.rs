fn main() {
    if cfg!(target_os = "windows") {
        let res = winres::WindowsResource::new();
        // Automatically injects a manifest to enable Windows Common Controls v6 (Visual Styles)
        res.compile().unwrap();
    }
}
