// iCONup - Rust Backend
// Secure FTP/SFTP upload with encrypted profile management
// Uses russh (pure Rust) instead of ssh2 (C bindings) for cross-platform compatibility

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;
use russh::client;
use russh_sftp::client::SftpSession;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use suppaftp::FtpStream;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
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
// SSH CLIENT HANDLER
// =====================

struct SshHandler;

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys (like ssh -o StrictHostKeyChecking=no)
        Ok(true)
    }
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
async fn upload_folder(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    match config.protocol.as_str() {
        "ftp" => do_upload_ftp(app, config).await,
        "sftp" => do_upload_sftp(app, config).await,
        _ => Err("Protocollo non supportato. Usa FTP o SFTP.".to_string()),
    }
}

// =====================
// FTP UPLOAD
// =====================

async fn do_upload_ftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
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
        
        let local_size = content.len();
        let filename = relative_path.display().to_string();
        
        // Upload file
        let status = match ftp.put_file(&remote_file_path, &mut content.as_slice()) {
            Ok(_) => {
                // Verify file size after upload
                match ftp.size(&remote_file_path) {
                    Ok(remote_size) => {
                        if remote_size == local_size {
                            "success"
                        } else {
                            eprintln!("Size mismatch for {}: local={} remote={}", filename, local_size, remote_size);
                            "error"
                        }
                    }
                    Err(e) => {
                        // SIZE command failed, but upload succeeded - warn but don't fail
                        eprintln!("Could not verify size for {}: {}", filename, e);
                        "success"
                    }
                }
            }
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
// SFTP UPLOAD (russh)
// =====================

async fn do_upload_sftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    // SSH connection config
    let ssh_config = client::Config::default();
    let handler = SshHandler;

    // Connect to SSH server
    let mut session = client::connect(
        Arc::new(ssh_config),
        (config.host.as_str(), config.port),
        handler,
    )
    .await
    .map_err(|e| format!("Connessione SSH fallita: {}", e))?;

    // Authenticate with password
    let auth_result = session
        .authenticate_password(&config.username, &config.password)
        .await
        .map_err(|e| format!("Autenticazione fallita: {}", e))?;

    match auth_result {
        client::AuthResult::Success => {}
        client::AuthResult::Failure { .. } => {
            return Err("Autenticazione SFTP fallita: credenziali non valide".to_string());
        }
    }

    // Open SFTP channel
    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| format!("Errore apertura canale: {}", e))?;

    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("Errore richiesta SFTP: {}", e))?;

    // Create SFTP session
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("Errore inizializzazione SFTP: {}", e))?;

    // Collect files to upload
    let files: Vec<_> = WalkDir::new(&config.local_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let total = files.len() as u32;
    let mut current: u32 = 0;

    // Upload each file
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

        // Create parent directories
        if let Some(parent) = Path::new(&remote_file_path).parent() {
            let parent_str = parent.display().to_string().replace("\\", "/");
            create_sftp_dirs(&sftp, &parent_str).await?;
        }

        // Read local file
        let content = fs::read(local_file_path)
            .map_err(|e| format!("Errore lettura file: {}", e))?;

        let local_size = content.len() as u64;
        let filename = relative_path.display().to_string();

        // Upload file
        let status = match sftp.create(&remote_file_path).await {
            Ok(mut remote_file) => {
                match remote_file.write_all(&content).await {
                    Ok(_) => {
                        let _ = remote_file.shutdown().await;
                        // Verify file size after upload
                        match sftp.metadata(&remote_file_path).await {
                            Ok(metadata) => {
                                if metadata.len() == Some(local_size) {
                                    "success"
                                } else {
                                    eprintln!("Size mismatch for {}: local={} remote={:?}", filename, local_size, metadata.len());
                                    "error"
                                }
                            }
                            Err(e) => {
                                // metadata failed, but upload succeeded - warn but don't fail
                                eprintln!("Could not verify size for {}: {}", filename, e);
                                "success"
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Write error for {}: {}", filename, e);
                        "error"
                    }
                }
            }
            Err(e) => {
                eprintln!("Create error for {}: {}", filename, e);
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

    // Close SFTP session
    sftp.close().await.map_err(|e| format!("Errore chiusura SFTP: {}", e))?;

    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

async fn create_sftp_dirs(sftp: &SftpSession, path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current_path = String::new();
    for part in parts {
        current_path.push('/');
        current_path.push_str(part);
        // Try to create directory, ignore errors if it exists
        let _ = sftp.create_dir(&current_path).await;
    }
    Ok(())
}

// =====================
// MAIN
// =====================

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_folder_contents,
            upload_folder,
            load_profiles,
            save_profiles,
            export_profiles,
            import_profiles
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
