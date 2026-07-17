fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");
    // manifest_optional: keep the build working even without the rc tool
    // (comctl v6 / PerMonitorV2 have code-side fallbacks).
    embed_resource::compile("app.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
