use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    if target_os != "macos" && target_os != "ios" {
        return;
    }
    if env::consts::OS != "macos" {
        return;
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=swift/store_bridge.swift");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let target = env::var("TARGET").expect("TARGET not set");
    let arch = match target.split('-').next().expect("TARGET is empty") {
        "aarch64" => "arm64",
        other => other,
    };
    let abi = env::var("CARGO_CFG_TARGET_ABI").unwrap_or_default();
    let (sdk, triple) = match target_os.as_str() {
        "macos" => ("macosx", format!("{arch}-apple-macosx12.0")),
        _ if abi == "sim" || target.ends_with("-sim") => ("iphonesimulator", format!("{arch}-apple-ios15.0-simulator")),
        _ => ("iphoneos", format!("{arch}-apple-ios15.0")),
    };
    let sdk_path = capture("xcrun", &["--sdk", sdk, "--show-sdk-path"]);
    let object = out_dir.join("store_bridge.o");
    run("swiftc", &["-parse-as-library", "-O", "-emit-object", "swift/store_bridge.swift", "-target", &triple, "-sdk", &sdk_path, "-o", object.to_str().expect("object path is not UTF-8")]);
    let library = out_dir.join("libstore_bridge.a");
    run("libtool", &["-static", "-o", library.to_str().expect("library path is not UTF-8"), object.to_str().expect("object path is not UTF-8")]);
    let swiftc_path = capture("xcrun", &["--find", "swiftc"]);
    let toolchain_lib = PathBuf::from(&swiftc_path).parent().and_then(|bin| bin.parent()).expect("unexpected swiftc path").join("lib").join("swift").join(sdk);
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=store_bridge");
    println!("cargo:rustc-link-search=native={sdk_path}/usr/lib/swift");
    println!("cargo:rustc-link-search=native={}", toolchain_lib.display());
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
}

fn capture(program: &str, args: &[&str]) -> String {
    let output = Command::new(program).args(args).output().unwrap_or_else(|error| panic!("failed to run {program}: {error}"));
    if !output.status.success() {
        panic!("{program} exited with error: {}", String::from_utf8_lossy(&output.stderr));
    }
    String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("{program} output is not UTF-8: {error}")).trim().to_string()
}

fn run(program: &str, args: &[&str]) {
    let output = Command::new(program).args(args).output().unwrap_or_else(|error| panic!("failed to run {program}: {error}"));
    if !output.status.success() {
        panic!("{program} failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
