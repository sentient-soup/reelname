fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "ReelName");
        res.set("FileDescription", "Media file scanner, matcher, and organizer");
        res.set(
            "LegalCopyright",
            &format!("Copyright © {}", chrono::Utc::now().format("%Y")),
        );
        res.compile().expect("Failed to compile Windows resources");
    }
}
