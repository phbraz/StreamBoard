fn main() {
    println!("cargo:rerun-if-changed=../assets/streamboard.ico");
    #[cfg(target_os = "windows")]
    embed_app_icon();
}

#[cfg(target_os = "windows")]
fn embed_app_icon() {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../assets/streamboard.ico");
    resource.compile().expect("embedding the StreamBoard icon");
}
