use anyhow::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-env-changed=GIT_RELEASE_TAG");

    let target_family = std::env::var("CARGO_CFG_TARGET_FAMILY")?;

    if target_family != "wasm" {
        println!("cargo:rustc-cfg=feature=\"local_fs\"");
    }

    Ok(())
}
