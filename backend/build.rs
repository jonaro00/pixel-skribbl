fn main() {
    if std::env::var("SHUTTLE").is_ok() {
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
