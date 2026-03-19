// iCONup - Frontend Logic

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;

let currentStep = 1;
const totalSteps = 4;
let profiles = [];
let currentProfileId = null;
let profileToDelete = null;

let config = {
    ftp: { host: '', username: '', password: '', port: 21, protocol: 'ftps', basePath: '', acceptInvalidCerts: false },
    product: '',
    remotePath: '',
    localFolder: ''
};

const productPaths = { iconcms: '/iconcms/', iconstat: '/iconstat/', iconblog: '/iconblog/', iconvert: '/iconvert/', custom: '' };

const panels = document.querySelectorAll('.panel');
const steps = document.querySelectorAll('.step');
const btnPrev = document.getElementById('btn-prev');
const btnNext = document.getElementById('btn-next');

document.addEventListener('DOMContentLoaded', async () => {
    await loadProfilesFromBackend();
    initEventListeners();
    initTauriListeners();
    updateNavigation();
    renderProfileSelect();
    updateSslOptionsVisibility();
});

function updateSslOptionsVisibility() {
    const protocol = document.getElementById('ftp-protocol').value;
    const sslOptions = document.getElementById('ssl-options');
    sslOptions.style.display = protocol === 'ftps' ? 'block' : 'none';
    if (protocol !== 'ftps') document.getElementById('accept-invalid-certs').checked = false;
}

async function loadProfilesFromBackend() {
    try { profiles = await invoke('load_profiles'); }
    catch (e) { console.error('Error loading profiles:', e); profiles = []; }
}

async function saveProfilesToBackend() {
    try { await invoke('save_profiles', { profiles }); }
    catch (e) { console.error('Error saving profiles:', e); }
}

function renderProfileSelect() {
    const select = document.getElementById('profile-select');
    select.innerHTML = '<option value="">— Nuovo / Manuale —</option>';
    profiles.forEach(p => {
        const opt = document.createElement('option');
        opt.value = p.id;
        opt.textContent = p.name;
        select.appendChild(opt);
    });
}

function renderProfileList() {
    const list = document.getElementById('profile-list');
    const empty = document.getElementById('profile-empty');
    if (profiles.length === 0) { list.style.display = 'none'; empty.style.display = 'block'; return; }
    list.style.display = 'flex'; empty.style.display = 'none';
    list.innerHTML = profiles.map(p => `
        <div class="profile-item" data-id="${p.id}">
            <div class="profile-item-info">
                <div class="profile-item-name">${escapeHtml(p.name)}</div>
                <div class="profile-item-host">${escapeHtml(p.host)} • ${p.protocol.toUpperCase()}</div>
            </div>
            <div class="profile-item-actions">
                <button type="button" class="btn-duplicate" title="Duplica">📋</button>
                <button type="button" class="btn-delete" title="Elimina">🗑️</button>
            </div>
        </div>
    `).join('');
    list.querySelectorAll('.btn-duplicate').forEach(btn => btn.addEventListener('click', e => duplicateProfile(e.target.closest('.profile-item').dataset.id)));
    list.querySelectorAll('.btn-delete').forEach(btn => btn.addEventListener('click', e => confirmDeleteProfile(e.target.closest('.profile-item').dataset.id)));
}

function escapeHtml(text) { const d = document.createElement('div'); d.textContent = text; return d.innerHTML; }
function generateId() { return Date.now().toString(36) + Math.random().toString(36).substr(2); }

function loadProfileIntoForm(id) {
    const p = profiles.find(x => x.id === id);
    if (!p) return;
    currentProfileId = id;
    document.getElementById('profile-name').value = p.name;
    document.getElementById('ftp-host').value = p.host;
    document.getElementById('ftp-user').value = p.username;
    document.getElementById('ftp-pass').value = p.password;
    document.getElementById('ftp-port').value = p.port;
    document.getElementById('ftp-protocol').value = p.protocol;
    document.getElementById('ftp-basepath').value = p.basePath;
    document.getElementById('accept-invalid-certs').checked = p.acceptInvalidCerts || false;
    updateSslOptionsVisibility();
}

function clearForm() {
    currentProfileId = null;
    document.getElementById('profile-name').value = '';
    document.getElementById('ftp-host').value = '';
    document.getElementById('ftp-user').value = '';
    document.getElementById('ftp-pass').value = '';
    document.getElementById('ftp-port').value = '21';
    document.getElementById('ftp-protocol').value = 'ftps';
    document.getElementById('ftp-basepath').value = '';
    document.getElementById('accept-invalid-certs').checked = false;
    updateSslOptionsVisibility();
}

async function saveCurrentProfile() {
    const name = document.getElementById('profile-name').value.trim();
    const host = document.getElementById('ftp-host').value.trim();
    const username = document.getElementById('ftp-user').value.trim();
    const password = document.getElementById('ftp-pass').value;
    const port = parseInt(document.getElementById('ftp-port').value) || 21;
    const protocol = document.getElementById('ftp-protocol').value;
    const basePath = document.getElementById('ftp-basepath').value.trim();
    const acceptInvalidCerts = document.getElementById('accept-invalid-certs').checked;
    if (!name) { alert('Inserisci un nome per il profilo'); return; }
    if (!host || !username) { alert('Inserisci almeno host e username'); return; }
    const data = { name, host, username, password, port, protocol, basePath, acceptInvalidCerts };
    if (currentProfileId) {
        const i = profiles.findIndex(x => x.id === currentProfileId);
        if (i !== -1) profiles[i] = { ...profiles[i], ...data };
    } else {
        const np = { id: generateId(), ...data };
        profiles.push(np);
        currentProfileId = np.id;
    }
    await saveProfilesToBackend();
    renderProfileSelect();
    document.getElementById('profile-select').value = currentProfileId;
    alert('Profilo salvato!');
}

function duplicateProfile(id) {
    const orig = profiles.find(x => x.id === id);
    if (!orig) return;
    profiles.push({ ...orig, id: generateId(), name: orig.name + ' (copia)' });
    saveProfilesToBackend();
    renderProfileSelect();
    renderProfileList();
}

function confirmDeleteProfile(id) {
    const p = profiles.find(x => x.id === id);
    if (!p) return;
    profileToDelete = id;
    document.getElementById('confirm-profile-name').textContent = p.name;
    document.getElementById('modal-confirm').style.display = 'flex';
}

async function deleteProfile() {
    if (!profileToDelete) return;
    profiles = profiles.filter(x => x.id !== profileToDelete);
    await saveProfilesToBackend();
    if (currentProfileId === profileToDelete) { clearForm(); document.getElementById('profile-select').value = ''; }
    profileToDelete = null;
    document.getElementById('modal-confirm').style.display = 'none';
    renderProfileSelect();
    renderProfileList();
}

async function exportProfiles() {
    if (profiles.length === 0) { alert('Nessun profilo da esportare'); return; }
    try {
        const filePath = await save({ defaultPath: 'iconup-profiles.json', filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (filePath) { await invoke('export_profiles', { profiles, filePath }); alert('Profili esportati!'); }
    } catch (e) { alert('Errore: ' + e); }
}

async function importProfiles() {
    try {
        const filePath = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (filePath) {
            const imported = await invoke('import_profiles', { filePath });
            if (imported && imported.length > 0) {
                imported.forEach(p => { p.id = generateId(); profiles.push(p); });
                await saveProfilesToBackend();
                renderProfileSelect();
                renderProfileList();
                alert(`Importati ${imported.length} profili!`);
            } else alert('Nessun profilo trovato');
        }
    } catch (e) { alert('Errore: ' + e); }
}

function initEventListeners() {
    btnPrev.addEventListener('click', prevStep);
    btnNext.addEventListener('click', nextStep);
    document.getElementById('profile-select').addEventListener('change', e => e.target.value ? loadProfileIntoForm(e.target.value) : clearForm());
    document.getElementById('btn-save-profile').addEventListener('click', saveCurrentProfile);
    document.querySelectorAll('input[name="product"]').forEach(r => r.addEventListener('change', e => {
        config.product = e.target.value;
        config.remotePath = productPaths[e.target.value];
        document.querySelector('.custom-path-group').style.display = e.target.value === 'custom' ? 'block' : 'none';
    }));
    document.getElementById('custom-path').addEventListener('input', e => config.remotePath = e.target.value);
    document.getElementById('select-folder').addEventListener('click', selectFolder);
    document.getElementById('ftp-protocol').addEventListener('change', e => {
        document.getElementById('ftp-port').value = e.target.value === 'sftp' ? 22 : 21;
        updateSslOptionsVisibility();
    });
    document.getElementById('btn-manage-profiles').addEventListener('click', () => { renderProfileList(); document.getElementById('modal-profiles').style.display = 'flex'; });
    document.getElementById('modal-close').addEventListener('click', () => document.getElementById('modal-profiles').style.display = 'none');
    document.getElementById('btn-close-modal').addEventListener('click', () => document.getElementById('modal-profiles').style.display = 'none');
    document.getElementById('btn-import').addEventListener('click', importProfiles);
    document.getElementById('btn-export').addEventListener('click', exportProfiles);
    document.getElementById('btn-cancel-delete').addEventListener('click', () => { profileToDelete = null; document.getElementById('modal-confirm').style.display = 'none'; });
    document.getElementById('btn-confirm-delete').addEventListener('click', deleteProfile);
    document.querySelectorAll('.modal-overlay').forEach(o => o.addEventListener('click', e => { if (e.target === o) { o.style.display = 'none'; profileToDelete = null; } }));
}

async function initTauriListeners() {
    await listen('upload-progress', e => updateProgress(e.payload.current, e.payload.total, e.payload.filename, e.payload.status));
    await listen('upload-complete', e => showUploadComplete(e.payload));
    await listen('upload-error', e => showUploadError(e.payload));
}

function updateNavigation() {
    panels.forEach(p => { p.classList.remove('active'); if (parseInt(p.dataset.panel) === currentStep) p.classList.add('active'); });
    steps.forEach(s => { const n = parseInt(s.dataset.step); s.classList.remove('active', 'completed'); if (n === currentStep) s.classList.add('active'); else if (n < currentStep) s.classList.add('completed'); });
    btnPrev.disabled = currentStep === 1;
    if (currentStep === totalSteps) btnNext.style.display = 'none';
    else { btnNext.style.display = 'inline-flex'; btnNext.textContent = currentStep === 3 ? 'Inizia Upload →' : 'Avanti →'; }
}

function prevStep() { if (currentStep > 1) { currentStep--; updateNavigation(); } }

async function nextStep() {
    if (!validateStep(currentStep)) return;
    if (currentStep === 3) { currentStep++; updateNavigation(); await startUpload(); return; }
    if (currentStep < totalSteps) { currentStep++; updateNavigation(); }
}

function validateStep(step) {
    if (step === 1) return validateFtpConfig();
    if (step === 2) return validateProductSelection();
    if (step === 3) return validateFolderSelection();
    return true;
}

function validateFtpConfig() {
    const host = document.getElementById('ftp-host').value.trim();
    const user = document.getElementById('ftp-user').value.trim();
    const pass = document.getElementById('ftp-pass').value;
    const port = document.getElementById('ftp-port').value;
    const protocol = document.getElementById('ftp-protocol').value;
    const basePath = document.getElementById('ftp-basepath').value.trim();
    const acceptInvalidCerts = document.getElementById('accept-invalid-certs').checked;
    if (!host) { alert('Inserisci l\'indirizzo del server'); return false; }
    if (!user) { alert('Inserisci il nome utente'); return false; }
    if (!pass) { alert('Inserisci la password'); return false; }
    config.ftp = { host, username: user, password: pass, port: parseInt(port) || 21, protocol, basePath, acceptInvalidCerts };
    return true;
}

function validateProductSelection() {
    if (!config.product) { alert('Seleziona un prodotto'); return false; }
    if (config.product === 'custom' && !config.remotePath) { alert('Inserisci il percorso personalizzato'); return false; }
    return true;
}

function validateFolderSelection() {
    if (!config.localFolder) { alert('Seleziona la cartella da caricare'); return false; }
    return true;
}

async function selectFolder() {
    try {
        const selected = await open({ directory: true, multiple: false, title: 'Seleziona la cartella da caricare' });
        if (selected) {
            config.localFolder = selected;
            const fd = document.getElementById('selected-folder');
            fd.textContent = selected;
            fd.classList.add('has-folder');
            const files = await invoke('list_folder_contents', { path: selected });
            showFolderPreview(files);
        }
    } catch (e) { console.error('Error selecting folder:', e); }
}

function showFolderPreview(files) {
    document.getElementById('folder-preview').style.display = 'block';
    const fl = document.getElementById('file-list');
    const display = files.slice(0, 20);
    fl.innerHTML = display.map(f => `<div>📄 ${f}</div>`).join('');
    if (files.length > 20) fl.innerHTML += `<div style="color:var(--text-muted)">... e altri ${files.length - 20} file</div>`;
    document.getElementById('file-count').textContent = `Totale: ${files.length} file`;
}

async function startUpload() {
    const statusEl = document.getElementById('upload-status');
    const logEl = document.getElementById('upload-log');
    statusEl.textContent = 'Connessione al server...';
    logEl.innerHTML = '';
    addLog('Avvio connessione ' + config.ftp.protocol.toUpperCase() + '...', 'info');
    try {
        let fullRemotePath = config.ftp.basePath + config.remotePath;
        fullRemotePath = fullRemotePath.replace(/\/+/g, '/');
        if (!fullRemotePath.startsWith('/')) fullRemotePath = '/' + fullRemotePath;
        addLog(`Percorso destinazione: ${fullRemotePath}`, 'info');
        await invoke('upload_folder', {
            config: {
                host: config.ftp.host,
                username: config.ftp.username,
                password: config.ftp.password,
                port: config.ftp.port,
                protocol: config.ftp.protocol,
                remote_path: fullRemotePath,
                local_path: config.localFolder,
                accept_invalid_certs: config.ftp.acceptInvalidCerts
            }
        });
    } catch (e) { addLog(`Errore: ${e}`, 'error'); statusEl.textContent = 'Errore durante l\'upload'; }
}

function updateProgress(current, total, filename, status) {
    const pct = Math.round((current / total) * 100);
    document.getElementById('progress-fill').style.width = `${pct}%`;
    document.getElementById('progress-percent').textContent = `${pct}%`;
    document.getElementById('progress-files').textContent = `${current} / ${total} file`;
    document.getElementById('upload-status').textContent = `Upload: ${filename}`;
    addLog(status === 'success' ? `✓ ${filename}` : `✗ ${filename}`, status);
}

function addLog(msg, type = 'info') {
    const log = document.getElementById('upload-log');
    const e = document.createElement('div');
    e.className = `log-${type}`;
    e.textContent = msg;
    log.appendChild(e);
    log.scrollTop = log.scrollHeight;
}

function showUploadComplete(data) {
    document.getElementById('upload-status').textContent = 'Upload completato!';
    document.getElementById('progress-fill').style.width = '100%';
    document.getElementById('progress-percent').textContent = '100%';
    document.getElementById('upload-complete').style.display = 'block';
    document.getElementById('complete-message').textContent = `${data.total_files} file caricati in ${data.remote_path}`;
    addLog(`Completato! ${data.total_files} file caricati.`, 'success');
    btnPrev.textContent = '← Nuova installazione';
    btnPrev.disabled = false;
    btnPrev.onclick = () => { currentStep = 1; updateNavigation(); resetUploadUI(); };
}

function showUploadError(error) {
    document.getElementById('upload-status').textContent = 'Errore durante l\'upload';
    addLog(`Errore: ${error}`, 'error');
}

function resetUploadUI() {
    document.getElementById('upload-complete').style.display = 'none';
    document.getElementById('upload-log').innerHTML = '';
    document.getElementById('progress-fill').style.width = '0%';
    document.getElementById('progress-percent').textContent = '0%';
    document.getElementById('progress-files').textContent = '0 / 0 file';
    btnPrev.textContent = '← Indietro';
    btnPrev.onclick = prevStep;
}
