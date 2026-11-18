fn main() {
    // If frontend assets are missing, build them automatically
    if !std::path::Path::new("ui/dist/index.html").exists() {
        println!("cargo:warning=Building UI frontend assets via npm run build...");
        let npm_cmd = if cfg!(target_os = "windows") { "npm.cmd" } else { "npm" };
        let status = std::process::Command::new(npm_cmd)
            .args(["run", "build", "--prefix", "ui"])
            .status();
        if let Err(e) = status {
            println!("cargo:warning=Failed to invoke npm run build: {}", e);
        }
    }
    tauri_build::build()
}
