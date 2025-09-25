use cmake;
use cmake::Config;



fn main() {
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");

    // Builds the project in the directory located in `libfoo`, installing it
    // into $OUT_DIR
//    let dst = cmake::build("3rdparty/lvgl");

    // set(CMAKE_C_COMPILER arm-linux-gnueabi-gcc)
    // set(CMAKE_CXX_COMPILER arm-linux-gnueabi-g++)
    // set(CMAKE_C_COMPILER_ID GNU)  #Add these
    // set(CMAKE_CXX_COMPILER_ID GNU)

    let dst = Config::new("3rdparty/lvgl")
        .define("CMAKE_C_COMPILER", "xtensa-esp32-elf-gcc")
        .define("CMAKE_CXX_COMPILER", "xtensa-esp32-elf-g++")
        .define("CMAKE_C_COMPILER_ID", "gnu")
        .define("CMAKE_CXX_COMPILER_ID", "gnu")
//        .cflag("-mtext-section-literals")
        .cflag("-mlongcalls")
        .cxxflag("-mlongcalls")
        .build();

    // println!("cargo:warning=!!!!!!!!!!!!!!!!!!!!");
    // println!("cargo:warning={}", dst.display());
    // println!("cargo:warning=!!!!!!!!!!!!!!!!!!!!");

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
