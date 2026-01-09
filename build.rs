fn main() {
    if cfg!(target_os = "windows") {
        let res = winres::WindowsResource::new();

        res.compile().unwrap();
        
        embed_resource::compile("icons.rc", embed_resource::NONE);
    }
}
