// msnchat-rs
// Copyright (C) 2025 Joshua Byrnes
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{env, fs, path::PathBuf};
use winres::WindowsResource;

fn main() {
    embed_manifest();
    copy_dependency("deps/MsnChat45.ocx");

    let profile = std::env::var("PROFILE").unwrap();
    if profile == "release" {
        println!("cargo:rustc-link-arg=/EMITPOGOPHASEINFO");
    }
}

fn embed_manifest() {
    let manifest_path = "./src/manifest.template.xml";
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("manifest.xml");
    let version = format!("{}.0", env::var("CARGO_PKG_VERSION").unwrap());

    let manifest = fs::read_to_string(manifest_path).expect("Read template failed");
    let versioned_manifest = manifest.replace("@VERSION@", &version);

    fs::write(&out_path, versioned_manifest).expect("Write manifest failed");

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path);

    // Tell winres to use the generated manifest
    let mut res = WindowsResource::new();
    res.set_manifest_file(out_path.to_str().unwrap());
    res.compile().expect("Resource compile failed");
}

fn copy_dependency(relative_source: &str) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let source_file = manifest_dir.join(relative_source);

    let target_file = manifest_dir
        .join("target")
        .join(
            (target != host)
                .then_some(&target)
                .unwrap_or(&"".to_string()),
        )
        .join(&profile)
        .join(source_file.file_name().unwrap());

    eprintln!("📁 build.rs: Copying {:?} → {:?}", source_file, target_file);

    println!("cargo:rerun-if-changed={}", source_file.display());
    fs::copy(&source_file, &target_file)
        .unwrap_or_else(|e| panic!("Failed to copy {}: {e}", source_file.display()));
}
