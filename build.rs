use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:rustc-link-lib=zip");
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut config = cmake::Config::new("libzip");
    config.define("CMAKE_LINK_DEPENDS_USE_LINKER", "0");
    config.define("ENABLE_NETTLE", "OFF");
    config.define("ENABLE_GNUTLS", "OFF");
    config.define("ENABLE_MBEDTLS", "OFF");
    config.define("BUILD_TOOLS", "OFF");
    config.define("BUILD_REGRESS", "OFF");
    config.define("BUILD_EXAMPLES", "OFF");
    config.define("BUILD_DOC", "OFF");
    config.pic(true);
    config.register_dep("z");
    config.always_configure(true);

    if env::var("CARGO_FEATURE_STATIC").is_ok() {
        if target_os != "windows" {
            config.register_dep("ssl");
        }

        config.define("ENABLE_BZIP2", "OFF");
        config.define("ENABLE_LZMA", "OFF");
        config.define("ENABLE_ZSTD", "OFF");
        config.define("BUILD_SHARED_LIBS", "OFF");
        config.define(
            "ENABLE_OPENSSL",
            if target_os == "windows" { "OFF" } else { "ON" },
        );

        if let Ok(root) = env::var("DEP_OPENSSL_ROOT") {
            config.define("OPENSSL_ROOT_DIR", root);
        } else if let Ok(root) = env::var("OPENSSL_DIR") {
            config.define("OPENSSL_ROOT_DIR", root);
        }
    }

    println!("Configuring and compiling zip");
    let dst = config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:include={}/include", dst.display());

    if env::var("CARGO_FEATURE_STATIC").is_ok() {
        if target_os == "windows" {
            println!("cargo:rustc-link-lib=static=zlibstatic");
        } else {
            println!("cargo:rustc-link-lib=static=z");
        }
        println!("cargo:rustc-link-lib=static=zip");
    } else {
        if target_os != "windows" {
            println!("cargo:rustc-link-lib=ssl");
            println!("cargo:rustc-link-lib=crypto");
        }
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=zip");
    }
}
