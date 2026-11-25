use anyhow::{anyhow, Result};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
const VERB: &str = "TerminiOpenHere";

pub fn register(label: &str) -> Result<()> {
    register_os(label)
}

pub fn unregister() -> Result<()> {
    unregister_os()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn known_labels() -> &'static [&'static str] {
    super::all_menu_labels()
}

fn exe_str() -> Result<String> {
    let exe = std::env::current_exe().map_err(|error| anyhow!("current exe: {error}"))?;
    exe.to_str().map(str::to_string).ok_or_else(|| anyhow!("exe path is not utf8"))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(target_os = "windows")]
fn register_os(label: &str) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::*;
    let exe = exe_str()?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey_with_flags("Software\\Classes", KEY_READ | KEY_WRITE)?;
    let command = format!("\"{exe}\" --open-here \"%V\"");
    for root in ["Directory", "Directory\\Background", "Drive", "Drive\\Background"] {
        let path = format!("{root}\\shell\\{VERB}");
        let (key, _) = classes.create_subkey(&path)?;
        key.set_value("MUIVerb", &label)?;
        key.set_value("Icon", &exe)?;
        let (cmd, _) = key.create_subkey("command")?;
        cmd.set_value("", &command)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_os() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::*;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey_with_flags("Software\\Classes", KEY_READ | KEY_WRITE)?;
    for root in ["Directory", "Directory\\Background", "Drive", "Drive\\Background"] {
        delete_reg_key(&classes, &format!("{root}\\shell\\{VERB}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_reg_key(classes: &winreg::RegKey, path: &str) -> Result<()> {
    match classes.open_subkey(path) {
        Ok(_) => classes.delete_subkey_all(path).map_err(|error| anyhow!("delete {path}: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow!("open {path}: {error}")),
    }
}

#[cfg(target_os = "linux")]
fn register_os(label: &str) -> Result<()> {
    let exe = exe_str()?;
    let data = data_home()?;
    unregister_os()?;
    let exec = if exe.chars().any(char::is_whitespace) { format!("\"{}\"", exe.replace('"', "\\\"")) } else { exe.clone() };
    write_executable(
        &data.join("nautilus/scripts").join(label),
        &format!(
            "#!/bin/sh\nEXE={}\nif [ -n \"$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS\" ]; then\n  printf '%s\\n' \"$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS\" | while IFS= read -r p; do\n    [ -n \"$p\" ] || continue\n    \"$EXE\" --open-here \"$p\" &\n  done\n  exit 0\nfi\nfor p in \"$@\"; do\n  \"$EXE\" --open-here \"$p\" &\ndone\n",
            sh_single_quote(&exe)
        ),
    )?;
    std::fs::create_dir_all(data.join("kio/servicemenus"))?;
    std::fs::write(
        data.join("kio/servicemenus/termini-open-here.desktop"),
        format!(
            "[Desktop Entry]\nType=Service\nX-KDE-ServiceTypes=KonqPopupMenu/Plugin\nMimeType=inode/directory;\nActions=openHere;\nX-KDE-Priority=TopLevel\n\n[Desktop Action openHere]\nName={label}\nIcon=utilities-terminal\nExec={exec} --open-here %f\n"
        ),
    )?;
    run_if_present("kbuildsycoca6")?;
    run_if_present("kbuildsycoca5")?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn unregister_os() -> Result<()> {
    let data = data_home()?;
    for label in known_labels() {
        remove_if_exists(&data.join("nautilus/scripts").join(label))?;
    }
    remove_if_exists(&data.join("kio/servicemenus/termini-open-here.desktop"))
}

#[cfg(target_os = "linux")]
fn data_home() -> Result<PathBuf> {
    match std::env::var("XDG_DATA_HOME") {
        Ok(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        Ok(_) | Err(_) => Ok(home_dir()?.join(".local/share")),
    }
}

#[cfg(target_os = "linux")]
fn write_executable(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_if_present(name: &str) -> Result<()> {
    match std::process::Command::new(name).status() {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow!("{name}: {error}")),
    }
}

#[cfg(target_os = "macos")]
fn register_os(label: &str) -> Result<()> {
    let exe = exe_str()?;
    let exe_json = serde_json::to_string(&exe).map_err(|error| anyhow!("exe json: {error}"))?;
    unregister_os()?;
    let workflow = home_dir()?.join("Library/Services").join(format!("{label}.workflow"));
    let contents = workflow.join("Contents");
    std::fs::create_dir_all(&contents)?;
    std::fs::write(contents.join("Info.plist"), mac_info_plist(label))?;
    std::fs::write(contents.join("document.wflow"), mac_document_wflow(&exe_json))?;
    let status = std::process::Command::new("/System/Library/CoreServices/pbs").arg("-flush").status()?;
    if !status.success() {
        return Err(anyhow!("pbs -flush failed: {status}"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn unregister_os() -> Result<()> {
    let services = home_dir()?.join("Library/Services");
    for label in known_labels() {
        remove_if_exists(&services.join(format!("{label}.workflow")))?;
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn home_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn remove_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|error| anyhow!("remove {}: {error}", path.display()))
    } else {
        std::fs::remove_file(path).map_err(|error| anyhow!("remove {}: {error}", path.display()))
    }
}

#[cfg(target_os = "macos")]
fn mac_info_plist(label: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>{}</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
				<string>public.folder</string>
				<string>public.item</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
"#,
        xml_escape(label)
    )
}

#[cfg(target_os = "macos")]
fn mac_document_wflow(exe_json: &str) -> String {
    let script = format!(
        "python3 -c {}",
        sh_single_quote(&format!(
            "import subprocess,sys\nexe={exe_json}\ndef launch(p):\n    marker='/Contents/MacOS/'\n    if marker in exe:\n        app=exe[:exe.index(marker)]+'.app'\n        subprocess.Popen(['open','-na',app,'--args','--open-here',p])\n    else:\n        subprocess.Popen([exe,'--open-here',p])\nfor p in sys.argv[1:]:\n    launch(p)\n"
        ))
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>ActionBundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>{}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>1</integer>
					<key>shell</key>
					<string>/bin/bash</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
			</dict>
		</dict>
	</array>
	<key>workflowTypeIdentifier</key>
	<string>com.apple.Automator.servicesMenu</string>
</dict>
</plist>
"#,
        xml_escape(&script)
    )
}
