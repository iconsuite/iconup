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
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use suppaftp::native_tls::TlsConnector;
use suppaftp::{FtpError, FtpStream, NativeTlsConnector, NativeTlsFtpStream, Status};
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
    #[serde(default)]
    pub product: String,
    #[serde(rename = "customPath", default)]
    pub custom_path: String,
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
// UPLOAD (SHARED)
// =====================

// Parallel connections per upload. Extra ones are optional.
const UPLOAD_CONNECTIONS: usize = 3;

struct UploadJob {
    local: PathBuf,
    remote: String,
    name: String,
}

struct UploadQueue {
    app: AppHandle,
    jobs: Vec<UploadJob>,
    pending: Mutex<VecDeque<usize>>,
    done: Mutex<u32>,
}

impl UploadQueue {
    fn new(app: AppHandle, jobs: Vec<UploadJob>) -> Self {
        let pending = (0..jobs.len()).collect();
        UploadQueue {
            app,
            jobs,
            pending: Mutex::new(pending),
            done: Mutex::new(0),
        }
    }

    fn take(&self) -> Option<usize> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).pop_front()
    }

    // Connection lost: the file goes back to the other connections.
    fn give_back(&self, index: usize) {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).push_front(index);
    }

    fn report(&self, index: usize, status: &str) {
        let mut done = self.done.lock().unwrap_or_else(|e| e.into_inner());
        *done += 1;
        let _ = self.app.emit("upload-progress", UploadProgress {
            current: *done,
            total: self.jobs.len() as u32,
            filename: self.jobs[index].name.clone(),
            status: status.to_string(),
        });
    }

    // Files left when every connection is gone count as errors.
    fn fail_remaining(&self) {
        while let Some(index) = self.take() {
            self.report(index, "error");
        }
    }

    fn extra_connections(&self) -> usize {
        UPLOAD_CONNECTIONS.min(self.jobs.len()).saturating_sub(1)
    }
}

// Files to upload, plus every remote directory to create, each listed once.
fn collect_jobs(config: &UploadConfig) -> Result<(Vec<UploadJob>, Vec<String>), String> {
    let files: Vec<_> = WalkDir::new(&config.local_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let mut jobs = Vec::new();
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

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
            let mut current_path = String::new();
            for part in parent_str.split('/').filter(|s| !s.is_empty()) {
                current_path.push('/');
                current_path.push_str(part);
                if seen.insert(current_path.clone()) {
                    dirs.push(current_path.clone());
                }
            }
        }

        jobs.push(UploadJob {
            local: local_file_path.to_path_buf(),
            remote: remote_file_path,
            name: relative_path.display().to_string(),
        });
    }

    Ok((jobs, dirs))
}

// =====================
// FTP / FTPS UPLOAD
// =====================

trait FtpConn {
    fn enter(&mut self, path: &str) -> bool;
    fn make_dir(&mut self, path: &str);
    fn send(&mut self, path: &str, content: &[u8]) -> Result<(), FtpError>;
    fn remote_size(&mut self, path: &str) -> Option<usize>;
    fn close(&mut self);
}

macro_rules! impl_ftp_conn {
    ($stream:ty, $send:ident) => {
        impl FtpConn for $stream {
            fn enter(&mut self, path: &str) -> bool {
                self.cwd(path).is_ok()
            }
            fn make_dir(&mut self, path: &str) {
                let _ = self.mkdir(path);
            }
            fn send(&mut self, path: &str, content: &[u8]) -> Result<(), FtpError> {
                $send(self, path, content)
            }
            fn remote_size(&mut self, path: &str) -> Option<usize> {
                self.size(path).ok()
            }
            fn close(&mut self) {
                let _ = self.quit();
            }
        }
    };
}

impl_ftp_conn!(FtpStream, send_plain);
impl_ftp_conn!(NativeTlsFtpStream, send_tls);

fn send_plain(ftp: &mut FtpStream, path: &str, mut content: &[u8]) -> Result<(), FtpError> {
    ftp.put_file(path, &mut content).map(|_| ())
}

// TLS 1.3 servers send data that nobody reads here. Closing over it
// resets the connection and truncates the file: read it all first.
fn send_tls(ftp: &mut NativeTlsFtpStream, path: &str, content: &[u8]) -> Result<(), FtpError> {
    let mut stream = ftp.put_with_stream(path)?;
    stream.write_all(content).map_err(FtpError::ConnectionError)?;

    let tcp = stream.get_ref().try_clone();
    drop(stream);

    if let Ok(mut tcp) = tcp {
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = tcp.shutdown(Shutdown::Write);
        let mut buf = [0u8; 4096];
        while matches!(tcp.read(&mut buf), Ok(n) if n > 0) {}
    }

    ftp.finalize_put_stream(std::io::sink())
}

fn open_ftp(config: &UploadConfig) -> Result<FtpStream, String> {
    let address = format!("{}:{}", config.host, config.port);

    let mut ftp = FtpStream::connect(&address)
        .map_err(|e| format!("Connessione fallita: {}", e))?;

    ftp.login(&config.username, &config.password)
        .map_err(|e| format!("Login fallito: {}", e))?;

    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .map_err(|e| format!("Errore impostazione modalità: {}", e))?;

    Ok(ftp)
}

fn open_ftps(config: &UploadConfig) -> Result<NativeTlsFtpStream, String> {
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

    Ok(ftp)
}

fn do_upload_ftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    upload_over_ftp(app, config, open_ftp)
}

fn do_upload_ftps(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    upload_over_ftp(app, config, open_ftps)
}

fn upload_over_ftp<C: FtpConn>(
    app: AppHandle,
    config: UploadConfig,
    open: fn(&UploadConfig) -> Result<C, String>,
) -> Result<(), String> {
    let mut first = open(&config)?;

    let (jobs, dirs) = collect_jobs(&config)?;
    let total = jobs.len() as u32;

    // Directories first, each one once, on a single connection.
    for dir in &dirs {
        if !first.enter(dir) {
            first.make_dir(dir);
        }
    }

    let queue = UploadQueue::new(app.clone(), jobs);

    // Each extra connection is opened inside its own thread.
    std::thread::scope(|scope| {
        for _ in 0..queue.extra_connections() {
            scope.spawn(|| {
                if let Ok(mut conn) = open(&config) {
                    ftp_worker(&mut conn, &queue);
                    conn.close();
                }
            });
        }
        ftp_worker(&mut first, &queue);
    });

    first.close();
    queue.fail_remaining();

    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

// Socket error, closed control connection or 421 from the server.
fn ftp_connection_lost(error: &FtpError) -> bool {
    match error {
        FtpError::ConnectionError(_) | FtpError::BadResponse => true,
        FtpError::UnexpectedResponse(response) => response.status == Status::NotAvailable,
        _ => false,
    }
}

fn ftp_worker<C: FtpConn>(conn: &mut C, queue: &UploadQueue) {
    conn.enter("/");

    while let Some(index) = queue.take() {
        let job = &queue.jobs[index];

        let content = match fs::read(&job.local) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Read error for {}: {}", job.name, e);
                queue.report(index, "error");
                continue;
            }
        };

        let status = match conn.send(&job.remote, &content) {
            Ok(_) => match conn.remote_size(&job.remote) {
                Some(remote_size) if remote_size == content.len() => "success",
                Some(remote_size) => {
                    eprintln!("Size mismatch for {}: local={} remote={}", job.name, content.len(), remote_size);
                    "error"
                }
                None => "success",
            },
            Err(e) if ftp_connection_lost(&e) => {
                eprintln!("Connection lost on {}: {}", job.name, e);
                queue.give_back(index);
                return;
            }
            Err(e) => {
                eprintln!("Upload error for {}: {}", job.name, e);
                "error"
            }
        };

        queue.report(index, status);
    }
}

// =====================
// SFTP UPLOAD
// =====================

fn open_sftp(config: &UploadConfig) -> Result<(Session, ssh2::Sftp), String> {
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

    Ok((session, sftp))
}

fn do_upload_sftp(app: AppHandle, config: UploadConfig) -> Result<(), String> {
    let (_session, sftp) = open_sftp(&config)?;

    let (jobs, dirs) = collect_jobs(&config)?;
    let total = jobs.len() as u32;

    // Directories first, each one once, on a single connection.
    for dir in &dirs {
        let _ = sftp.mkdir(Path::new(dir), 0o755);
    }

    let queue = UploadQueue::new(app.clone(), jobs);

    // Each extra connection is opened inside its own thread.
    std::thread::scope(|scope| {
        for _ in 0..queue.extra_connections() {
            scope.spawn(|| {
                if let Ok((_session, sftp)) = open_sftp(&config) {
                    sftp_worker(&sftp, &queue);
                }
            });
        }
        sftp_worker(&sftp, &queue);
    });

    queue.fail_remaining();

    let _ = app.emit("upload-complete", UploadComplete {
        total_files: total,
        remote_path: config.remote_path,
    });

    Ok(())
}

fn sftp_worker(sftp: &ssh2::Sftp, queue: &UploadQueue) {
    while let Some(index) = queue.take() {
        let job = &queue.jobs[index];

        let content = match fs::read(&job.local) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Read error for {}: {}", job.name, e);
                queue.report(index, "error");
                continue;
            }
        };
        let local_size = content.len() as u64;

        let status = match sftp.create(Path::new(&job.remote)) {
            Ok(mut remote_file) => match remote_file.write_all(&content) {
                Ok(_) => {
                    match sftp.stat(Path::new(&job.remote)) {
                        Ok(stat) if stat.size == Some(local_size) => "success",
                        Ok(stat) => {
                            eprintln!("Size mismatch for {}: local={} remote={:?}", job.name, local_size, stat.size);
                            "error"
                        }
                        Err(_) => "success"
                    }
                }
                Err(_) => "error",
            },
            Err(e) if matches!(e.code(), ssh2::ErrorCode::Session(_)) => {
                eprintln!("Connection lost on {}: {}", job.name, e);
                queue.give_back(index);
                return;
            }
            Err(_) => "error",
        };

        queue.report(index, status);
    }
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

            // Auto-size window to 95% of screen height
            if let Some(window) = app.get_webview_window("main") {
                if let Some(monitor) = window.current_monitor().ok().flatten() {
                    let screen_height = monitor.size().height as f64 / monitor.scale_factor();
                    let new_height = (screen_height * 0.85) as u32;
                    let _ = window.set_size(tauri::LogicalSize::new(600, new_height));
                }
            }

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
