fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo::rerun-if-changed=res/resource.rc");
        println!("cargo::rerun-if-changed=res/app.manifest");
        println!("cargo::rerun-if-changed=res/shinobu.ico");
        embed_resource::compile("res/resource.rc", &["RT_MANIFEST=24"])
            .manifest_required()
            .unwrap();
    }
}
