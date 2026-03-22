// iCONup - Frontend Logic

if (!navigator.onLine) {
    const ov = document.getElementById('offline-overlay');
    ov.style.display = 'flex';
    document.querySelector('.app').style.display = 'none';
    throw new Error('offline');
}

let invoke, open, save, listen;
try {
    invoke = window.__TAURI__.core.invoke;
    open = window.__TAURI__.dialog.open;
    save = window.__TAURI__.dialog.save;
    listen = window.__TAURI__.event.listen;
} catch(e) {
    console.warn('Tauri not available, running in browser mode');
    invoke = async () => [];
    open = async () => null;
    save = async () => null;
    listen = async () => {};
}

let currentStep = 1;
const totalSteps = 4;
let profiles = [];
let currentProfileId = null;
let profileToDelete = null;

let config = {
    ftp: { host: '', username: '', password: '', port: 21, protocol: 'ftps', basePath: '' },
    product: '',
    remotePath: '',
    localFolder: ''
};

const productPaths = { iconcms: '/_iconcms/', iconstat: '/_iconstat/', iconblog: '/_iconblog/', iconvert: '/_iconcms/iconvert/', custom: '' };

// Custom dropdown helpers
function initCustomSelect(wrapperId, selectId, onChange) {
    const wrapper = document.getElementById(wrapperId);
    const trigger = wrapper.querySelector('.custom-select-trigger');
    const optionsContainer = wrapper.querySelector('.custom-select-options');
    const hiddenSelect = document.getElementById(selectId);

    trigger.addEventListener('click', (e) => {
        e.stopPropagation();
        document.querySelectorAll('.custom-select.open').forEach(el => { if (el !== wrapper) el.classList.remove('open'); });
        wrapper.classList.toggle('open');
    });

    optionsContainer.addEventListener('click', (e) => {
        const opt = e.target.closest('.custom-select-option');
        if (!opt) return;
        const val = opt.dataset.value;
        const text = opt.textContent;
        hiddenSelect.value = val;
        trigger.querySelector('.custom-select-value').textContent = text;
        optionsContainer.querySelectorAll('.custom-select-option').forEach(o => o.classList.remove('selected'));
        opt.classList.add('selected');
        wrapper.classList.remove('open');
        if (onChange) onChange(val);
    });
}

function setCustomSelectValue(wrapperId, selectId, value) {
    const wrapper = document.getElementById(wrapperId);
    const hiddenSelect = document.getElementById(selectId);
    hiddenSelect.value = value;
    const opts = wrapper.querySelectorAll('.custom-select-option');
    opts.forEach(o => {
        o.classList.remove('selected');
        if (o.dataset.value === value) {
            o.classList.add('selected');
            wrapper.querySelector('.custom-select-value').textContent = o.textContent;
        }
    });
}

function renderCustomSelectOptions(wrapperId, options) {
    const wrapper = document.getElementById(wrapperId);
    const container = wrapper.querySelector('.custom-select-options');
    container.innerHTML = options.map(o => `<div class="custom-select-option${o.selected ? ' selected' : ''}" data-value="${o.value}">${o.text}</div>`).join('');
    const sel = options.find(o => o.selected) || options[0];
    if (sel) wrapper.querySelector('.custom-select-value').textContent = sel.text;
}

document.addEventListener('click', () => document.querySelectorAll('.custom-select.open').forEach(el => el.classList.remove('open')));

const panels = document.querySelectorAll('.panel');
const steps = document.querySelectorAll('.step');
const btnPrev = document.getElementById('btn-prev');
const btnNext = document.getElementById('btn-next');

document.addEventListener('DOMContentLoaded', async () => {
    initLang();
    await loadProfilesFromBackend();
    initEventListeners();
    initTauriListeners();
    updateNavigation();
    renderProfileSelect();
});

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
    select.innerHTML = `<option value="">${t('profileNew')}</option>`;
    const options = [{ value: '', text: t('profileNew'), selected: true }];
    profiles.forEach(p => {
        const opt = document.createElement('option');
        opt.value = p.id;
        opt.textContent = p.name;
        select.appendChild(opt);
        options.push({ value: p.id, text: p.name, selected: false });
    });
    renderCustomSelectOptions('profile-select-wrapper', options);
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
                <button type="button" class="btn-duplicate" title="${t('tipDuplicate')}"><img class="profiles-img" src="assets/duplicate.svg" alt="gear" width="24" height="24"></button>
                <button type="button" class="btn-delete" title="${t('tipDelete')}"><img class="profiles-img" src="assets/delete.svg" alt="gear" width="24" height="24"></button>
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
    setCustomSelectValue('ftp-protocol-wrapper', 'ftp-protocol', p.protocol);
    document.getElementById('ftp-basepath').value = p.basePath;
}

function clearForm() {
    currentProfileId = null;
    document.getElementById('profile-name').value = '';
    document.getElementById('ftp-host').value = '';
    document.getElementById('ftp-user').value = '';
    document.getElementById('ftp-pass').value = '';
    document.getElementById('ftp-port').value = '22';
    setCustomSelectValue('ftp-protocol-wrapper', 'ftp-protocol', 'ftps');
    document.getElementById('ftp-basepath').value = '';
}

async function saveCurrentProfile() {
    const name = document.getElementById('profile-name').value.trim();
    const host = document.getElementById('ftp-host').value.trim();
    const username = document.getElementById('ftp-user').value.trim();
    const password = document.getElementById('ftp-pass').value;
    const port = parseInt(document.getElementById('ftp-port').value) || 22;
    const protocol = document.getElementById('ftp-protocol').value;
    const basePath = document.getElementById('ftp-basepath').value.trim();
    if (!name) { alert(t('alertProfileName')); return; }
    if (!host || !username) { alert(t('alertHostUser')); return; }
    const data = { name, host, username, password, port, protocol, basePath };
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
    setCustomSelectValue('profile-select-wrapper', 'profile-select', currentProfileId);
    alert(t('alertProfileSaved'));
}

function duplicateProfile(id) {
    const orig = profiles.find(x => x.id === id);
    if (!orig) return;
    profiles.push({ ...orig, id: generateId(), name: orig.name + t('profileCopySuffix') });
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
    if (currentProfileId === profileToDelete) { clearForm(); setCustomSelectValue('profile-select-wrapper', 'profile-select', ''); }
    profileToDelete = null;
    document.getElementById('modal-confirm').style.display = 'none';
    renderProfileSelect();
    renderProfileList();
}

async function exportProfiles() {
    if (profiles.length === 0) { alert(t('alertNoProfiles')); return; }
    try {
        const filePath = await save({ defaultPath: 'iconup-profiles.json', filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (filePath) { await invoke('export_profiles', { profiles, filePath }); alert(t('alertExported')); }
    } catch (e) { alert('Error: ' + e); }
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
                alert(t('alertImportedN', { n: imported.length }));
            } else alert(t('alertNoProfilesFound'));
        }
    } catch (e) { alert('Error: ' + e); }
}

function initEventListeners() {
    btnPrev.addEventListener('click', prevStep);
    btnNext.addEventListener('click', nextStep);
    document.getElementById('profile-select').addEventListener('change', e => e.target.value ? loadProfileIntoForm(e.target.value) : clearForm());
    initCustomSelect('profile-select-wrapper', 'profile-select', val => val ? loadProfileIntoForm(val) : clearForm());
    document.getElementById('btn-save-profile').addEventListener('click', saveCurrentProfile);
    document.querySelectorAll('input[name="product"]').forEach(r => r.addEventListener('change', e => {
        config.product = e.target.value;
        config.remotePath = productPaths[e.target.value];
        document.querySelector('.custom-path-group').style.display = e.target.value === 'custom' ? 'block' : 'none';
    }));
    document.getElementById('custom-path').addEventListener('input', e => config.remotePath = e.target.value);
    document.getElementById('select-folder').addEventListener('click', selectFolder);
    initCustomSelect('ftp-protocol-wrapper', 'ftp-protocol', val => {
        document.getElementById('ftp-port').value = val === 'sftp' ? 22 : 21;
    });
    document.getElementById('btn-manage-profiles').addEventListener('click', () => { renderProfileList(); document.getElementById('modal-profiles').style.display = 'flex'; });
    document.getElementById('modal-close').addEventListener('click', () => document.getElementById('modal-profiles').style.display = 'none');
    document.getElementById('btn-close-modal').addEventListener('click', () => document.getElementById('modal-profiles').style.display = 'none');
    document.getElementById('btn-import').addEventListener('click', importProfiles);
    document.getElementById('btn-export').addEventListener('click', exportProfiles);
    document.getElementById('btn-cancel-delete').addEventListener('click', () => { profileToDelete = null; document.getElementById('modal-confirm').style.display = 'none'; });
    document.getElementById('btn-confirm-delete').addEventListener('click', deleteProfile);
    document.querySelectorAll('.modal-overlay').forEach(o => o.addEventListener('click', e => { if (e.target === o) { o.style.display = 'none'; profileToDelete = null; } }));
    document.getElementById('btn-lang').addEventListener('click', () => {
        toggleLang();
        renderProfileSelect();
        updateNavigation();
    });
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
    else {
        btnNext.style.display = 'inline-flex';
        if (currentStep === 3) btnNext.innerHTML = `${t('btnStartUpload')} <span style="margin-left:2px">❯</span>`;
        else btnNext.innerHTML = `<span data-i18n="btnNext">${t('btnNext')}</span> <span style="margin-left:2px">❯</span>`;
    }
    const btnLang = document.getElementById('btn-lang');
    if (currentStep === 1) { btnLang.style.opacity = '1'; btnLang.style.pointerEvents = 'auto'; }
    else { btnLang.style.opacity = '0.4'; btnLang.style.pointerEvents = 'none'; }
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
    if (!host) { alert(t('alertHost')); return false; }
    if (!user) { alert(t('alertUser')); return false; }
    if (!pass) { alert(t('alertPass')); return false; }
    config.ftp = { host, username: user, password: pass, port: parseInt(port) || 22, protocol, basePath };
    return true;
}

function validateProductSelection() {
    if (!config.product) { alert(t('alertProduct')); return false; }
    if (config.product === 'custom' && !config.remotePath) { alert(t('alertCustomPath')); return false; }
    return true;
}

function validateFolderSelection() {
    if (!config.localFolder) { alert(t('alertFolder')); return false; }
    return true;
}

async function selectFolder() {
    try {
        const selected = await open({ directory: true, multiple: false, title: t('folderDialogTitle') });
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
    if (files.length > 20) fl.innerHTML += `<div style="color:var(--text-muted)">${t('folderMore', { n: files.length - 20 })}</div>`;
    document.getElementById('file-count').textContent = t('folderTotal', { n: files.length });
}

let pendingUploadConfig = null;

function getFullRemotePath() {
    let fullRemotePath = config.ftp.basePath + config.remotePath;
    fullRemotePath = fullRemotePath.replace(/\/+/g, '/');
    if (!fullRemotePath.startsWith('/')) fullRemotePath = '/' + fullRemotePath;
    return fullRemotePath;
}

function getUploadConfig(fullRemotePath) {
    return {
        host: config.ftp.host,
        username: config.ftp.username,
        password: config.ftp.password,
        port: config.ftp.port,
        protocol: config.ftp.protocol,
        remote_path: fullRemotePath,
        local_path: config.localFolder
    };
}

async function startUpload() {
    const statusEl = document.getElementById('upload-status');
    const logEl = document.getElementById('upload-log');
    statusEl.textContent = t('uploadVerifying');
    logEl.innerHTML = '';
    addLog(t('uploadStarting', { protocol: config.ftp.protocol.toUpperCase() }), 'info');

    const fullRemotePath = getFullRemotePath();
    const uploadConfig = getUploadConfig(fullRemotePath);

    addLog(t('uploadDestination', { path: fullRemotePath }), 'info');

    try {
        const exists = await invoke('check_remote_dir', { config: uploadConfig });
        if (exists) {
            pendingUploadConfig = uploadConfig;
            document.getElementById('folder-exists-msg').innerHTML = t('folderExistsMsg', { path: '<strong>' + fullRemotePath + '</strong>' });            
            document.getElementById('modal-folder-exists').style.display = 'flex';
            return;
        }
        await doUpload(uploadConfig);
    } catch (e) {
        addLog(`Error: ${e}`, 'error');
        statusEl.textContent = t('uploadErrorConnection');
    }
}

async function doUpload(uploadConfig) {
    const statusEl = document.getElementById('upload-status');
    statusEl.textContent = t('uploadConnecting');
    addLog(t('uploadInProgress'), 'info');
    try {
        await invoke('upload_folder', { config: uploadConfig });
    } catch (e) {
        addLog(`Error: ${e}`, 'error');
        statusEl.textContent = t('uploadErrorUpload');
    }
}

document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('btn-confirm-upload').addEventListener('click', async () => {
        document.getElementById('modal-folder-exists').style.display = 'none';
        if (pendingUploadConfig) {
            await doUpload(pendingUploadConfig);
            pendingUploadConfig = null;
        }
    });
    document.getElementById('btn-cancel-upload').addEventListener('click', () => {
        document.getElementById('modal-folder-exists').style.display = 'none';
        pendingUploadConfig = null;
        currentStep = 3;
        updateNavigation();
        addLog(t('uploadCancelled'), 'info');
    });
});

function updateProgress(current, total, filename, status) {
    const pct = Math.round((current / total) * 100);
    document.getElementById('progress-fill').style.width = `${pct}%`;
    document.getElementById('progress-percent').textContent = `${pct}%`;
    document.getElementById('progress-files').textContent = `${current} / ${total} file`;
    document.getElementById('upload-status').textContent = t('uploadFile', { file: filename });
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
    btnNext.style.setProperty('display', 'none', 'important');
    document.getElementById('upload-status').textContent = t('uploadComplete');
    document.querySelector('.step[data-step="4"]').classList.remove('active');
    document.querySelector('.step[data-step="4"]').classList.add('completed');
    document.getElementById('progress-fill').style.width = '100%';
    document.getElementById('progress-percent').textContent = '100%';
    document.getElementById('upload-complete').style.display = 'block';
    document.getElementById('complete-message').textContent = t('uploadFilesIn', { n: data.total_files, path: data.remote_path });
    addLog(t('uploadLogComplete', { n: data.total_files }), 'success');
    btnPrev.innerHTML = `<span style="margin-right:2px">❮</span> ${t('btnNewInstall')}`;
    btnPrev.disabled = false;
    btnPrev.onclick = () => { currentStep = 1; updateNavigation(); resetUploadUI(); };
}

function showUploadError(error) {
    document.getElementById('upload-status').textContent = t('uploadErrorUpload');
    addLog(`Error: ${error}`, 'error');
}

function resetUploadUI() {
    document.getElementById('upload-complete').style.display = 'none';
    document.getElementById('upload-log').innerHTML = '';
    document.getElementById('progress-fill').style.width = '0%';
    document.getElementById('progress-percent').textContent = '0%';
    document.getElementById('progress-files').textContent = '0 / 0 file';
    btnPrev.innerHTML = `<span style="margin-right:2px">❮</span> <span data-i18n="btnBack">${t('btnBack')}</span>`;
    btnPrev.onclick = prevStep;
}

const btnDragClose = document.getElementById('btn-drag-close');
if (btnDragClose) {
    btnDragClose.addEventListener('click', async () => {
        try { await window.__TAURI__.process.exit(0); } 
        catch(e) { window.close(); }
    });
}
