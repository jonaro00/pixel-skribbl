fn main() {
    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        if !std::process::Command::new("trunk")
            .args(["build", "--release"])
            .current_dir("frontend")
            .status()
            .expect("failed to run trunk")
            .success()
        {
            panic!("failed to run trunk")
        }
    }
}
