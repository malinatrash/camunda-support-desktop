fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icons/icon.ico");
        resource.set("ProductName", "Camunda Support");
        resource.set("FileDescription", "Camunda Support");
        resource.compile().expect("ресурсы Windows");
    }
}
