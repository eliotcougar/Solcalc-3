use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-changed=solcalc2/solcalc.ico");
    println!("cargo:rerun-if-changed=assets/icon-small.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("solcalc.res");
    let status = Command::new(env::var_os("RC").unwrap_or_else(|| "rc.exe".into()))
        .arg("/nologo")
        .arg("/fo")
        .arg(&output)
        .arg("assets/app.rc")
        .status()
        .expect("Windows resource compiler not found. Build in a VS Native Tools shell or use build.ps1.");
    assert!(status.success(), "Could not compile Windows resources");
    println!("cargo:rustc-link-arg={}", output.display());
}
