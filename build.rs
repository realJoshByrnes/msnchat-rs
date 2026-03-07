use pelite::pe32::{Pe, PeFile};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/vendor/microsoft/MsnChat45.ocx");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let tlb_path = PathBuf::from(&out_dir).join("MsnChat45.tlb");

    // Extract TYPELIB from OCX
    let ocx_path = "assets/vendor/microsoft/MsnChat45.ocx";
    if let Ok(ocx_data) = fs::read(ocx_path)
        && let Ok(pe) = PeFile::from_bytes(&ocx_data)
        && let Ok(res) = pe.resources()
    {
        // In COM DLLs, TYPELIB is usually stored under the string type "TYPELIB" with ID 1
        let mut found = false;
        if let Ok(bytes) = res.find_resource(&[
            pelite::resources::Name::Str("TYPELIB"),
            pelite::resources::Name::Id(1),
        ]) {
            fs::write(&tlb_path, bytes).expect("Failed to write extracted TLB");
            found = true;
        }

        if found {
            let target = env::var("TARGET").unwrap_or_default();
            let mut res = winres::WindowsResource::new();

            // winres defaults to plain "windres"/"ar" on Unix, but cross toolchains
            // usually provide target-prefixed binaries.
            if target.contains("windows-gnu") {
                if target.starts_with("i686-") {
                    res.set_windres_path("i686-w64-mingw32-windres")
                        .set_ar_path("i686-w64-mingw32-ar");
                } else if target.starts_with("x86_64-") {
                    res.set_windres_path("x86_64-w64-mingw32-windres")
                        .set_ar_path("x86_64-w64-mingw32-ar");
                }
            }

            res.append_rc_content(&format!(
                "LANGUAGE 0, 0\n\
                1 TYPELIB \"{}\"\n",
                tlb_path.display().to_string().replace("\\", "/")
            ));
            if let Err(err) = res.compile() {
                println!("cargo:warning=Failed to compile Windows resources: {}", err);
            } else if target.contains("windows-gnu") {
                // GNU linkers may ignore archive members that do not satisfy undefined symbols.
                // resource.o only carries .rsrc, so force-link the object file explicitly.
                let resource_obj = PathBuf::from(&out_dir).join("resource.o");
                println!("cargo:rustc-link-arg={}", resource_obj.display());
            }
        } else {
            println!("cargo:warning=Failed to find TYPELIB in MsnChat45.ocx");
        }
    }
}
