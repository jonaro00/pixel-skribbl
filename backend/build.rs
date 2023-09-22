fn main() {
    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        if !std::process::Command::new("trunk")
            .args(["build", "--release"])
            .current_dir("../frontend")
            .env(
                "CARGO_TARGET_DIR",
                "/opt/shuttle/shuttle-builds/extratarget",
            )
            .status()
            .expect("failed to run trunk")
            .success()
        {
            panic!("trunk did not succeed")
        }
    }
}
