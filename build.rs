extern crate cc;
use std::env;
use std::path::Path;

fn main() {
    if cfg!(feature = "unsafe_pointer") {
        println!("cargo:allow=dropping_references");
    }

    // even if the blossom V library exists, sometimes we don't want to compile it
    let mut try_include_blossom_v = true;
    if cfg!(feature = "remove_blossom_v") {
        try_include_blossom_v = false;
    }

    if try_include_blossom_v && Path::new("./blossomV/PerfectMatching.h").exists() {
        println!("cargo:rustc-cfg=feature=\"blossom_v\"");

        let target_os = env::var("CARGO_CFG_TARGET_OS");

        let mut build = cc::Build::new();

        build
            .cpp(true)
            .file("./blossomV/blossomV.cpp")
            .file("./blossomV/PMinterface.cpp")
            .file("./blossomV/PMduals.cpp")
            .file("./blossomV/PMexpand.cpp")
            .file("./blossomV/PMinit.cpp")
            .file("./blossomV/PMmain.cpp")
            .file("./blossomV/PMrepair.cpp")
            .file("./blossomV/PMshrink.cpp")
            .file("./blossomV/misc.cpp")
            .file("./blossomV/MinCost/MinCost.cpp");

        if target_os != Ok("macos".to_string()) {
            // exclude from macOS
            build.cpp_link_stdlib("stdc++"); // use libstdc++
            build.flag("-Wno-unused-but-set-variable"); // this option is not available in clang
        }

        // ignore warnings from blossom library
        build
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-variable")
            .flag("-Wno-reorder-ctor")
            .flag("-Wno-reorder")
            .compile("blossomV");

        println!("cargo:rerun-if-changed=./blossomV/blossomV.cpp");
        println!("cargo:rerun-if-changed=./blossomV/PerfectMatching.h");

        println!("cargo:rustc-link-lib=static=blossomV");

        if target_os != Ok("macos".to_string()) {
            // exclude from macOS
            // println!("cargo:rustc-link-lib=static=stdc++");  // have to add this to compile c++ (new, delete operators)
            println!("cargo:rustc-link-lib=dylib=stdc++"); // NOTE: this MUST be put after "cargo:rustc-link-lib=static=blossomV"
        }
    }
}
