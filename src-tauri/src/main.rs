// iCONup - Rust Backend
// Secure FTP/FTPS/SFTP upload with encrypted profile management

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use suppaftp::native_tls::TlsConnector;
use suppaftp::{FtpStream, NativeTlsConnector, NativeTlsFtpStream};
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

// =====================
// TYPES
// =====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub protocol: String,
    #[serde(rename = "basePath")]
    pub base_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredProfiles {
    version: u32,
    nonce: String,
    profiles: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub protocol: String,
    pub remote_path: String,
    pub local_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UploadProgress {
    pub current: u32,
    pub total: u32,
    pub filename: String,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UploadComplete {
    pub total_files: u32,
    pub remote_path: String,
}

// =====================
// ENCRYPTION (AES-256-GCM)
// =====================

const ENCRYPTION_KEY: &[u8; 32] = b"iCONup_AES256_Key_YMEDIA_2024!!X";

fn encrypt_data(data: &str) -> Result<(String, String), String> {
    let cipher = Aes256Gcm::new_from_slice(ENCRYPTION_KEY)
        .map_err(|e| format!("Encryption init error: {}", e))?;

    let mut rng = rand::thread_rng();
    let nonce_bytes: [u8; 12] = rng.gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("Encryption error: {}", e))?;

    Ok((BASE64.encode(nonce_bytes), BASE64.encode(ciphertext)))
}

fn decrypt_data(nonce_b64: &str, ciphertext_b64: &str) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(ENCRYPTION_KEY)
        .map_err(|e| format!("Decryption init error: {}", e))?;

    let nonce_bytes = BASE64
        .decode(nonce_b64)
        .map_err(|e| format!("Nonce decode error: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| format!("Ciphertext decode error: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption error: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF8 error: {}", e))
}

// =====================
// FILE PATHS
// =====================

fn get_profiles_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("iCONup");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("profiles.dat")
}

// =====================
// TAURI COMMANDS
// =====================

#[tauri::command]
fn load_profiles() -> Result<Vec<Profile>, String> {
    let path = get_profiles_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))?;
    let stored: StoredProfiles =
        serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let decrypted = decrypt_data(&stored.nonce, &stored.profiles)?;
    let profiles: Vec<Profile> =
        serde_json::from_str(&decrypted).map_err(|e| format!("JSON error: {}", e))?;
    Ok(profiles)
}

#[tauri::command]
fn save_profiles(profiles: Vec<Profile>) -> Result<(), String> {
    let path = get_profiles_path();
    let json = serde_json::to_string(&profiles).map_err(|e| format!("Serialize error: {}", e))?;
    let (nonce, encrypted) = encrypt_data(&json)?;
    let stored = StoredProfiles {
        version: 1,
        nonce,
        profiles: encrypted,
    };
    let content =
        serde_json::to_string_pretty(&stored).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn export_profiles(profiles: Vec<Profile>, file_path: String) -> Result<(), String> {
    let json = serde_json::to_string(&profiles).map_err(|e| format!("Serialize error: {}", e))?;
    let (nonce, encrypted) = encrypt_data(&json)?;
    let stored = StoredProfiles {
        version: 1,
        nonce,
        profiles: encrypted,
    };
    let content =
        serde_json::to_string_pretty(&stored).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&file_path, content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn import_profiles(file_path: String) -> Result<Vec<Profile>, String> {
    let content = fs::read_to_string(&file_path).map_err(|e| format!("Read error: {}", e))?;
    let stored: StoredProfiles =
        serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let decrypted = decrypt_data(&stored.nonce, &stored.profiles)?;
    let profiles: Vec<Profile> =
        serde_json::from_str(&decrypted).map_err(|e| format!("JSON error: {}", e))?;
    Ok(profiles)
}

#[tauri::command]
fn list_folder_contents(path: String) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Ok(relative) = entry.path().strip_prefix(&path) {
            files.push(relative.display().to_string());
        }
    }
    Ok(files)
}

#[tauri::command]
async fn check_remote_dir(config: UploadConfig) -> Result<bool, String> {
    match config.protocol.as_str() {
        "ftp" => check_ftp_dir(&config),
        "ftps" => check_ftps_dir(&config),
        "sftp" => check_sftp_dir(&config),
        _ => Err("Protocollo non supportato.".to_string()),
    }
}

fn check_ftp_dir(config: &UploadConfig) -> Result<bool, String> {
    let address = format!("{}:{}", config.host, config.port);
    let mut ftp = FtpStream::connect(&address)
        .map_err(|e| format!("Connessione fallita: {}", e))?;
    ftp.login(&config.username, &config.password)
        .map_err(|e| format!("Login fallito: {}", e))?;
    let exists = ftp.cwd(&config.remote_path).is_ok();
    let _ = ftp.quit();
    Ok(exists)
}

fn check_ftps_dir(config: &UploadConfig) -> Result<bool, String> {
    let address = format!("{}:{}", config.host, config.port);
    let ftp_stream = NativeTlsFtpStream::connect(&address)
        .map_err(|e| format!("Connessione fallita: {}", e))?;
    let ctx = NativeTlsConnector::from(TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Errore TLS: {}", e))?);
    let mut ftp = ftp_stream.into_secure(ctx, &config.host)
        .map_err(|e| format!("Upgrade TLS fallito: {}", e))?;
    ftp.login(&config.username, &config.password)
        .map_err(|e| format!("Login fallito: {}", e))?;
    let exists = ftp.cwd(&config.remote_path).is_ok();
    let _ = ftp.quit();
    Ok(exists)
}

fn check_sftp_dir(config: &UploadConfig) -> Result<bool, String> {
    let address = format!("{}:{}", config.host, config.port);
    let sock_addr = std::net::ToSocketAddrs::to_socket_addrs(&address.as_str())
        .map_err(|e| format!("Impossibile risolvere l'indirizzo: {}", e))?
        .next()
        .ok_or("Impossibile risolvere l'indirizzo del server".to_string())?;
    let tcp = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(10))
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut {
                "Connessione SFTP scaduta. Verifica che il server supporti SSH oppure usa FTP.".to_string()
            } else {
                format!("Connessione fallita: {}", e)
            }
        })?;
    let mut session = Session::new().map_err(|e| format!("Errore sessione SSH: {}", e))?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| format!("Handshake SSH fallito: {}", e))?;
    session.userauth_password(&config.username, &config.password)
        .map_err(|e| format!("Autenticazione fallita: {}", e))?;
    if !session.authenticated() {
        return Err("Autenticazione SFTP fallita".to_string());
    }
    let sftp = session.sftp().map_err(|e| format!("Errore SFTP: {}", e))?;
    let exists = sftp.stat(Path::new(&config.remote_path)).is_ok();
    Ok(exists)
}

#[tauri::command]
async fn upload_folder(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    match config.protocol.as_str() {
        "ftp" => do_upload_ftp(app, config),
        "ftps" => do_upload_ftps(app, config),
        "sftp" => do_upload_sftp(app, config),
        _ => Err("Protocollo non supportato. Usa FTP, FTPS o SFTP.".to_string()),
    }
}

// =====================
// FTP UPLOAD
// =====================

fn do_upload_ftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    let address = format!("{}:{}", config.host, config.port);

    let mut ftp = FtpStream::connect(&address)
        .map_err(|e| format!("Connessione fallita: {}", e))?;

    ftp.login(&config.username, &config.password)
        .map_err(|e| format!("Login fallito: {}", e))?;

    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .map_err(|e| format!("Errore impostazione modalità: {}", e))?;

    let files: Vec<_> = WalkDir::new(&config.local_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let total = files.len() as u32;
    let mut current: u32 = 0;

    for entry in files {
        let local_file_path = entry.path();
        let relative_path = local_file_path
            .strip_prefix(&config.local_path)
            .map_err(|e| format!("Errore percorso: {}", e))?;

        let remote_file_path = format!(
            "{}/{}",
            config.remote_path.trim_end_matches('/'),
            relative_path.display()
        )
        .replace("\\", "/");

        if let Some(parent) = Path::new(&remote_file_path).parent() {
            let parent_str = parent.display().to_string().replace("\\", "/");
            create_ftp_dirs(&mut ftp, &parent_str)?;
        }

        let file = File::open(local_file_path).map_err(|e| format!("Errore lettura file: {}", e))?;
        let mut reader = BufReader::new(file);
        let mut content = Vec::new();
        reader.read_to_end(&mut content).map_err(|e| format!("Errore lettura file: {}", e))?;

        let filename = relative_path.display().to_string();
        let status = match ftp.put_file(&remote_file_path, &mut content.as_slice()) {
            Ok(_) => "success",
            Err(e) => {
                eprintln!("Upload error for {}: {}", filename, e);
                "error"
            }
        };

        current += 1;
        let _ = app.emit("upload-progress", UploadProgress {
            current,
            total,
            filename: filename.clone(),
            status: status.to_string(),
        });
    }

    let _ = ftp.quit();
    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

fn create_ftp_dirs(ftp: &mut FtpStream, path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current_path = String::new();
    for part in parts {
        current_path.push('/');
        current_path.push_str(part);
        if ftp.cwd(&current_path).is_err() {
            let _ = ftp.mkdir(&current_path);
        }
    }
    let _ = ftp.cwd("/");
    Ok(())
}

// =====================
// FTPS UPLOAD (FTP + TLS)
// =====================

fn do_upload_ftps(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    let address = format!("{}:{}", config.host, config.port);

    let ftp_stream = NativeTlsFtpStream::connect(&address)
        .map_err(|e| format!("Connessione fallita: {}", e))?;

    let ctx = NativeTlsConnector::from(TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Errore TLS: {}", e))?);

    let mut ftp = ftp_stream.into_secure(ctx, &config.host)
        .map_err(|e| format!("Upgrade TLS fallito: {}", e))?;

    ftp.login(&config.username, &config.password)
        .map_err(|e| format!("Login fallito: {}", e))?;

    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .map_err(|e| format!("Errore impostazione modalità: {}", e))?;

    let files: Vec<_> = WalkDir::new(&config.local_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let total = files.len() as u32;
    let mut current: u32 = 0;

    for entry in files {
        let local_file_path = entry.path();
        let relative_path = local_file_path
            .strip_prefix(&config.local_path)
            .map_err(|e| format!("Errore percorso: {}", e))?;

        let remote_file_path = format!(
            "{}/{}",
            config.remote_path.trim_end_matches('/'),
            relative_path.display()
        )
        .replace("\\", "/");

        if let Some(parent) = Path::new(&remote_file_path).parent() {
            let parent_str = parent.display().to_string().replace("\\", "/");
            create_ftps_dirs(&mut ftp, &parent_str)?;
        }

        let file = File::open(local_file_path).map_err(|e| format!("Errore lettura file: {}", e))?;
        let mut reader = BufReader::new(file);
        let mut content = Vec::new();
        reader.read_to_end(&mut content).map_err(|e| format!("Errore lettura file: {}", e))?;

        let filename = relative_path.display().to_string();
        let status = match ftp.put_file(&remote_file_path, &mut content.as_slice()) {
            Ok(_) => "success",
            Err(e) => {
                eprintln!("Upload error for {}: {}", filename, e);
                "error"
            }
        };

        current += 1;
        let _ = app.emit("upload-progress", UploadProgress {
            current,
            total,
            filename: filename.clone(),
            status: status.to_string(),
        });
    }

    let _ = ftp.quit();
    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

fn create_ftps_dirs(ftp: &mut NativeTlsFtpStream, path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current_path = String::new();
    for part in parts {
        current_path.push('/');
        current_path.push_str(part);
        if ftp.cwd(&current_path).is_err() {
            let _ = ftp.mkdir(&current_path);
        }
    }
    let _ = ftp.cwd("/");
    Ok(())
}

// =====================
// SFTP UPLOAD
// =====================

fn do_upload_sftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    let address = format!("{}:{}", config.host, config.port);
    let sock_addr = std::net::ToSocketAddrs::to_socket_addrs(&address.as_str())
        .map_err(|e| format!("Impossibile risolvere l'indirizzo: {}", e))?
        .next()
        .ok_or("Impossibile risolvere l'indirizzo del server".to_string())?;
    let tcp = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(10))
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut {
                "Connessione SFTP scaduta. Verifica che il server supporti SSH oppure usa FTP.".to_string()
            } else {
                format!("Connessione fallita: {}", e)
            }
        })?;

    let mut session = Session::new().map_err(|e| format!("Errore sessione SSH: {}", e))?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| format!("Handshake SSH fallito: {}", e))?;
    session.userauth_password(&config.username, &config.password)
        .map_err(|e| format!("Autenticazione fallita: {}", e))?;

    if !session.authenticated() {
        return Err("Autenticazione SFTP fallita".to_string());
    }

    let sftp = session.sftp().map_err(|e| format!("Errore SFTP: {}", e))?;

    let files: Vec<_> = WalkDir::new(&config.local_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let total = files.len() as u32;
    let mut current: u32 = 0;

    for entry in files {
        let local_file_path = entry.path();
        let relative_path = local_file_path
            .strip_prefix(&config.local_path)
            .map_err(|e| format!("Errore percorso: {}", e))?;

        let remote_file_path = format!(
            "{}/{}",
            config.remote_path.trim_end_matches('/'),
            relative_path.display()
        )
        .replace("\\", "/");

        if let Some(parent) = Path::new(&remote_file_path).parent() {
            create_sftp_dirs(&sftp, &parent.display().to_string())?;
        }

        let content = fs::read(local_file_path).map_err(|e| format!("Errore lettura file: {}", e))?;

        let filename = relative_path.display().to_string();
        let status = match sftp.create(Path::new(&remote_file_path)) {
            Ok(mut remote_file) => match remote_file.write_all(&content) {
                Ok(_) => "success",
                Err(_) => "error",
            },
            Err(_) => "error",
        };

        current += 1;
        let _ = app.emit("upload-progress", UploadProgress {
            current,
            total,
            filename: filename.clone(),
            status: status.to_string(),
        });
    }

    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

fn create_sftp_dirs(sftp: &ssh2::Sftp, path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current_path = String::new();
    for part in parts {
        current_path.push('/');
        current_path.push_str(part);
        let _ = sftp.mkdir(Path::new(&current_path), 0o755);
    }
    Ok(())
}

// =====================
// MAIN
// =====================

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let help_page = MenuItemBuilder::new("iCONup help")
                .id("help-page")
                .build(app)?;

            let app_submenu = SubmenuBuilder::new(app, "iCONup")
                .about(None)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let file_submenu = SubmenuBuilder::new(app, "File")
                .close_window()
                .build()?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let help_submenu = SubmenuBuilder::new(app, "Help")
                .item(&help_page)
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&app_submenu, &file_submenu, &edit_submenu, &help_submenu])
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(move |_app, event| {
                if event.id() == "help-page" {
                    let _ = open::that("https://www.iconsuite.it/iconup");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_folder_contents,
            upload_folder,
            check_remote_dir,
            load_profiles,
            save_profiles,
            export_profiles,
            import_profiles
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        let error_msg = format!("iCONup startup error: {}", e);
        eprintln!("{}", error_msg);
        // Write crash log to config dir
        if let Some(config_dir) = dirs::config_dir() {
            let log_dir = config_dir.join("iCONup");
            let _ = fs::create_dir_all(&log_dir);
            let _ = fs::write(
                log_dir.join("crash.log"),
                format!("{}\nTimestamp: {:?}", error_msg, std::time::SystemTime::now()),
            );
        }
        std::process::exit(1);
    }
}
