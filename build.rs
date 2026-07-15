//! Embeds the application icon into the Windows executable so it shows in
//! Explorer, the taskbar, and Alt-Tab. Non-Windows builds do nothing.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // Don't fail the build if the resource compiler is unavailable —
            // the app still runs, just without an embedded .exe icon.
            println!("cargo:warning=could not embed the app icon: {e}");
        }
    }
}
