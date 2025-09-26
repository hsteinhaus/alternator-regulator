use std::env;
use std::path::{Path, PathBuf};
use cc::Build;
use cmake;
use cmake::Config;

fn main() {
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
    cmake_lvgl();
    compile_lvgl_inline_wrappers();
}

fn compile_lvgl_inline_wrappers() {
    let project_dir = canonicalize(PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()));
    let x = project_dir.join("src").join("ui").join("lvgl_buffers.c");

    let mut cfg = Build::new();
    cfg.compiler("xtensa-esp32-elf-gcc")
        .include("./lvgl_rust_sys/lvgl")
        .file("/tmp/bindgen/extern.c")
        .file(x)
        .compile("lvgl-inline-wrappers");
    println!(
        "cargo:info=Building for target {:?} using compiler {:?}",
        std::env::var("TARGET"),
        cfg.get_compiler().path()
    );
}

fn cmake_lvgl() {
    let dst = Config::new("lvgl_rust_sys/lvgl")
        .define("CMAKE_C_COMPILER", "xtensa-esp32-elf-gcc")
        .define("CMAKE_CXX_COMPILER", "xtensa-esp32-elf-g++")
        .define("CMAKE_C_COMPILER_ID", "gnu")
        .define("CMAKE_CXX_COMPILER_ID", "gnu")
        .cflag("-mlongcalls")
        .cflag("-fkeep-inline-functions")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=lvgl");
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_defmt_timestamp" => {
                    eprintln!();
                    eprintln!("💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`");
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                "esp_wifi_preempt_enable"
                | "esp_wifi_preempt_yield_task"
                | "esp_wifi_preempt_task_create" => {
                    eprintln!();
                    eprintln!("💡 `esp-wifi` has no scheduler enabled. Make sure you have the `builtin-scheduler` feature enabled, or that you provide an external scheduler.");
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!("💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests");
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}


fn canonicalize(path: impl AsRef<Path>) -> PathBuf {
    let canonicalized = path.as_ref().canonicalize().unwrap();
    let canonicalized = &*canonicalized.to_string_lossy();

    PathBuf::from(canonicalized.strip_prefix(r"\\?\").unwrap_or(canonicalized))
}
