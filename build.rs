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
            let rc_content = format!(
                "LANGUAGE 0, 0\n\
                1 TYPELIB \"{}\"\n",
                tlb_path.display().to_string().replace("\\", "/")
            );
            let rc_path = PathBuf::from(&out_dir).join("msnchat.rc");
            fs::write(&rc_path, rc_content).expect("Failed to write RC");
            let _ = embed_resource::compile(&rc_path, &[""]);
        } else {
            println!("cargo:warning=Failed to find TYPELIB in MsnChat45.ocx");
        }
    }
}
