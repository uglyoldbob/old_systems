use std::{env, fs, path::Path};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.compile().unwrap();
    }
    if cfg!(target_os = "windows") {
        let out_dir = env::var("OUT_DIR").unwrap();

        let target_dir = Path::new(&out_dir)
            .ancestors()
            .nth(3)
            .expect("could not find target directory");

        let gst_bin = Path::new(r"C:\Program Files\gstreamer\1.0\msvc_x86_64\bin");

        for entry in fs::read_dir(gst_bin).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().and_then(|x| x.to_str()) == Some("dll") {
                let dst = target_dir.join(path.file_name().unwrap());

                fs::copy(&path, &dst).unwrap_or_else(|e| panic!("failed to copy {path:?}: {e}"));
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}
