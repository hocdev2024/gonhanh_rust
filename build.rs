fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon/app.ico");
        res.compile().unwrap();
    }
}
