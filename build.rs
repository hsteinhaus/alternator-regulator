use cc::Build;
use cmake::Config;

fn main() {
    linker_be_nice();
    #[cfg(feature = "defmt")]
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
    cmake_lvgl();
    compile_lvgl_inline_wrappers();
}

fn compile_lvgl_inline_wrappers() {
    let mut cfg = Build::new();
    cfg.compiler("xtensa-esp32-elf-gcc")
        .include("./lvgl_rust_sys/lvgl")
        .file("/tmp/bindgen/extern.c")
        .flag("-Ofast")
        .flag("-flto")
        .flag("-ftree-vectorize")
        .flag("-fno-strict-aliasing")
        .flag("-fdata-sections")
        .flag("-ffunction-sections")
        .compile("lvgl-inline-wrappers");
    println!(
        "cargo:info=Building for target {:?} using compiler {:?}",
        std::env::var("TARGET"),
        cfg.get_compiler().path()
    );
}

fn cmake_lvgl() {
    let dst = Config::new("lvgl_rust_sys/lvgl")
        .define("CMAKE_TOOLCHAIN_FILE", "../../toolchain-esp32.cmake")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_VERBOSE_MAKEFILE", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("LV_LVGL_H_INCLUDE_SIMPLE", "ON")
        .define("LV_CONF_INCLUDE_SIMPLE", "ON")
        .define("LV_CONF_PATH", "lv_conf.h")
        .define("LV_CONF_ERROR_STR", "NULL")
        .define("LV_CONF_ERROR_INCLUDE_SIMPLE", "ON")
        .define("LV_CONF_ERROR_THROW", "ON")
        .cflag("-mlongcalls")
        .cflag("-Ofast")
        .cflag("-flto")
        .cflag("-ftree-vectorize")
        .cflag("-fno-strict-aliasing")
        .cflag("-fdata-sections")
        .cflag("-ffunction-sections")
//        .cflag("-fkeep-inline-functions")
        .profile("Release")
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
                "esp_wifi_preempt_enable" | "esp_wifi_preempt_yield_task" | "esp_wifi_preempt_task_create" => {
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
