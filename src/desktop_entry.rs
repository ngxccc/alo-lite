//! Desktop integration for Linux: registers `.desktop` file and application icon in XDG environment.

use anyhow::Result;
use std::path::PathBuf;

/// Dynamically generate and register the .desktop file and application icon in the current user's XDG environment.
pub fn install_desktop_entry() -> Result<()> {
    let exe = std::env::current_exe()?
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_exe().unwrap());
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // 1. Install application icon to ~/.local/share/icons/hicolor/256x256/apps/alolite.png
    let icon_dir = home.join(".local/share/icons/hicolor/256x256/apps");
    std::fs::create_dir_all(&icon_dir)?;
    let icon_path = icon_dir.join("alolite.png");
    let icon_bytes = include_bytes!("assets/app_icon.png");
    std::fs::write(&icon_path, icon_bytes)?;

    // 2. Dynamically generate ~/.local/share/applications/alolite.desktop with exact absolute paths
    let app_dir = home.join(".local/share/applications");
    std::fs::create_dir_all(&app_dir)?;
    let desktop_path = app_dir.join("alolite.desktop");

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Alolite\n\
         GenericName=Instant Messaging Client\n\
         Comment=Lightweight native messaging client in Rust (Zero Node.js, Zero Electron)\n\
         Exec=\"{}\" --gui\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=Network;InstantMessaging;Chat;\n\
         StartupWMClass=alolite\n",
        exe.display(),
        icon_path.display()
    );
    std::fs::write(&desktop_path, desktop_content)?;

    // 3. Trigger desktop database update if tool exists
    if let Ok(mut child) = std::process::Command::new("update-desktop-database")
        .arg(&app_dir)
        .spawn()
    {
        let _ = child.wait();
    }

    println!("✓ Đã cài đặt desktop entry động thành công:");
    println!("  Binary thực thi: {}", exe.display());
    println!("  Icon ứng dụng:   {}", icon_path.display());
    println!("  Desktop entry:   {}", desktop_path.display());
    println!("Biểu tượng Alolite hiện đã sẵn sàng trong App Menu và Dock hệ thống!");
    Ok(())
}

/// Remove the dynamically generated .desktop file and icon.
pub fn uninstall_desktop_entry() -> Result<()> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let desktop_path = home.join(".local/share/applications/alolite.desktop");
    let icon_path = home.join(".local/share/icons/hicolor/256x256/apps/alolite.png");
    if desktop_path.exists() {
        let _ = std::fs::remove_file(&desktop_path);
        println!("✓ Đã gỡ bỏ: {}", desktop_path.display());
    }
    if icon_path.exists() {
        let _ = std::fs::remove_file(&icon_path);
        println!("✓ Đã gỡ bỏ: {}", icon_path.display());
    }
    let app_dir = home.join(".local/share/applications");
    if let Ok(mut child) = std::process::Command::new("update-desktop-database")
        .arg(&app_dir)
        .spawn()
    {
        let _ = child.wait();
    }
    println!("✓ Đã gỡ bỏ desktop entry khỏi hệ thống thành công!");
    Ok(())
}
