fn main() {
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");

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
