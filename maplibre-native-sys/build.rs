//! Build script for maplibre-native-sys
//!
//! This build script compiles the C++ wrapper and links to MapLibre GL Native.

use std::env;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Tell cargo to rerun if these files change
    println!("cargo:rerun-if-changed=cpp/maplibre_c.h");
    println!("cargo:rerun-if-changed=cpp/maplibre_c.cpp");
    println!("cargo:rerun-if-changed=cpp/maplibre_c_stub.c");
    println!("cargo:rerun-if-changed=build.rs");

    // Check if the native libraries are built
    // Try platform-specific build directories
    #[cfg(target_os = "macos")]
    let build_dir_name = "build-macos-metal";
    #[cfg(target_os = "linux")]
    let build_dir_name = "build-linux";
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let build_dir_name = "build";

    let maplibre_build_dir = manifest_dir
        .join("vendor/maplibre-native")
        .join(build_dir_name);
    let mbgl_core = maplibre_build_dir.join("libmbgl-core.a");

    if mbgl_core.exists() {
        println!("cargo:warning=Building with real MapLibre Native renderer");
        build_with_maplibre_native(&manifest_dir, &out_dir, &maplibre_build_dir);
    } else {
        println!("cargo:warning=MapLibre Native not built - using stub implementation");
        println!("cargo:warning=To build MapLibre Native, run:");
        #[cfg(target_os = "macos")]
        {
            println!("cargo:warning=  cd maplibre-native-sys/vendor/maplibre-native");
            println!("cargo:warning=  cmake --preset macos-metal");
            println!(
                "cargo:warning=  cmake --build build-macos-metal --target mbgl-core mlt-cpp -j8"
            );
        }
        #[cfg(target_os = "linux")]
        {
            println!("cargo:warning=  cd maplibre-native-sys/vendor/maplibre-native");
            println!("cargo:warning=  cmake -B build-linux -G Ninja -DCMAKE_BUILD_TYPE=Release");
            println!(
                "cargo:warning=  cmake --build build-linux --target mbgl-core mlt-cpp -j$(nproc)"
            );
        }
        build_stub(&out_dir);
    }
}

fn build_with_maplibre_native(manifest_dir: &Path, out_dir: &Path, maplibre_build_dir: &Path) {
    let maplibre_src = manifest_dir.join("vendor/maplibre-native");

    // Build our C++ wrapper
    let mut build = cc::Build::new();

    build
        .cpp(true)
        .file("cpp/maplibre_c.cpp")
        .include("cpp")
        // MapLibre Native include paths
        .include(maplibre_src.join("include"))
        .include(maplibre_src.join("platform/default/include"))
        .include(maplibre_src.join("src"))
        .include(maplibre_src.join("vendor/maplibre-native-base/extras/expected-lite/include"))
        .include(maplibre_src.join("vendor/maplibre-native-base/include"))
        .include(maplibre_src.join("vendor/maplibre-native-base/deps/geojson.hpp/include"))
        .include(maplibre_src.join("vendor/maplibre-native-base/deps/geometry.hpp/include"))
        .include(maplibre_src.join("vendor/maplibre-native-base/deps/variant/include"))
        .include(maplibre_src.join("vendor/maplibre-native-base/deps/optional/include"))
        .include(maplibre_src.join("vendor/rapidjson/include"))
        .flag("-std=c++20")
        .flag("-fPIC")
        .flag("-fvisibility=hidden")
        .warnings(false); // Suppress warnings from MapLibre Native headers

    // Platform-specific include paths and settings
    #[cfg(target_os = "macos")]
    {
        build.include(maplibre_src.join("platform/darwin/include"));
        build.flag("-mmacosx-version-min=14.3");
    }

    #[cfg(target_os = "linux")]
    {
        build.include(maplibre_src.join("platform/linux/include"));
    }

    build.compile("maplibre_c");

    // Link our wrapper
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=maplibre_c");

    // Link MapLibre Native libraries
    println!(
        "cargo:rustc-link-search=native={}",
        maplibre_build_dir.display()
    );

    // MLT (MapLibre Tiles) library is in a subdirectory
    println!(
        "cargo:rustc-link-search=native={}",
        maplibre_build_dir
            .join("vendor/maplibre-tile-spec/cpp")
            .display()
    );

    // Core MapLibre libraries (common to all platforms)
    println!("cargo:rustc-link-lib=static=mbgl-core");
    println!("cargo:rustc-link-lib=static=mlt-cpp");
    println!("cargo:rustc-link-lib=static=mbgl-freetype");
    println!("cargo:rustc-link-lib=static=mbgl-harfbuzz");
    println!("cargo:rustc-link-lib=static=mbgl-vendor-csscolorparser");
    println!("cargo:rustc-link-lib=static=mbgl-vendor-parsedate");

    // Link system libraries required by MapLibre Native
    #[cfg(target_os = "macos")]
    {
        // macOS uses vendored ICU
        println!("cargo:rustc-link-lib=static=mbgl-vendor-icu");

        // macOS frameworks
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ImageIO");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=SystemConfiguration");
        println!("cargo:rustc-link-lib=framework=CoreServices");

        // System libraries
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=sqlite3");

        // libuv (installed via homebrew)
        if let Ok(output) = Command::new("brew").args(["--prefix", "libuv"]).output() {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-link-search=native={}/lib", prefix);
                println!("cargo:rustc-link-lib=uv");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux uses system ICU instead of vendored
        println!("cargo:rustc-link-lib=icuuc");
        println!("cargo:rustc-link-lib=icui18n");
        println!("cargo:rustc-link-lib=icudata");

        // Additional vendored libraries on Linux
        println!("cargo:rustc-link-lib=static=mbgl-vendor-nunicode");
        println!("cargo:rustc-link-lib=static=mbgl-vendor-sqlite");

        // System libraries
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=curl");
        println!("cargo:rustc-link-lib=png");
        println!("cargo:rustc-link-lib=jpeg");
        println!("cargo:rustc-link-lib=webp");
        println!("cargo:rustc-link-lib=uv");

        // OpenGL/X11
        println!("cargo:rustc-link-lib=GL");
        println!("cargo:rustc-link-lib=EGL");
        println!("cargo:rustc-link-lib=X11");
    }
}

fn build_stub(out_dir: &Path) {
    let mut build = cc::Build::new();

    build
        .file("cpp/maplibre_c_stub.c")
        .include("cpp")
        .warnings(true)
        .extra_warnings(true)
        .opt_level(2);

    // Platform-specific settings
    #[cfg(target_os = "macos")]
    {
        build.flag("-std=c11");
    }

    #[cfg(target_os = "linux")]
    {
        build.flag("-std=c11");
        build.flag("-D_GNU_SOURCE");
    }

    build.compile("maplibre_c_stub");

    // The library will be in OUT_DIR
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=maplibre_c_stub");
}
