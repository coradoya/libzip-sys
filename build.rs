use std::{env, path::PathBuf};

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn build_libzip() {
    println!("cargo:rustc-link-lib=zip");
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut config = cmake::Config::new("libzip");
    config.define("ENABLE_NETTLE", "OFF");
    config.define("ENABLE_GNUTLS", "OFF");
    config.define("ENABLE_MBEDTLS", "OFF");
    config.define("BUILD_TOOLS", "OFF");
    config.define("BUILD_REGRESS", "OFF");
    config.define("BUILD_EXAMPLES", "OFF");
    config.define("BUILD_DOC", "OFF");
    config.pic(true);
    config.register_dep("z");

    #[cfg(feature = "static")]
    {
        config.define("ENABLE_BZIP2", "OFF");
        config.define("ENABLE_LZMA", "OFF");
        config.define("ENABLE_ZSTD", "OFF");
        config.define("BUILD_SHARED_LIBS", "OFF");
    }

    #[cfg(feature = "static")]
    {
        println!("Configuring and compiling zip");
        let dst = config.build();

        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        println!(
            "cargo:rustc-link-search={}/lib",
            env::var("DEP_Z_ROOT").unwrap()
        );

        #[cfg(all(windows, target_env = "gnu"))]
        println!("cargo:rustc-link-lib=static=zlib");

        #[cfg(not(all(windows, target_env = "gnu")))]
        println!("cargo:rustc-link-lib=static=z");

        println!("cargo:rustc-link-lib=static=zip");

        bindgen::Builder::default()
            .clang_arg(format!("-I{}/include/", out_dir().display()))
            .clang_arg(format!("-I{}", dst.as_path().display()))
            .header("wrapper.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_dir().join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }

    #[cfg(not(feature = "static"))]
    {
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=zip");

        bindgen::Builder::default()
            .header("wrapper.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_dir().join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}

fn main() {
    build_libzip();
}
