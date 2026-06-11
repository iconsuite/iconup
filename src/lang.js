// iCONup - Localization
const LANGS = {
    it: {
        offlineTitle: 'Nessuna connessione',
        offlineDesc: 'iCONup richiede una connessione internet per funzionare.',
        offlineRetry: 'Riprova',
        stepServer: 'Server',
        stepProduct: 'Prodotto',
        stepFolder: 'Cartella',
        stepUpload: 'Upload',
        serverTitle: 'Configurazione Server',
        serverDesc: 'Seleziona un profilo salvato o inserisci nuovi dati',
        profileLabel: 'Profilo',
        profileNew: '— Nuovo —',
        profileNameLabel: 'Nome profilo',
        profileNamePlaceholder: 'Es: iCONSuite',
        profileNameHint: 'Un nome personalizzato per identificare questo sito',
        hostLabel: 'Host',
        hostPlaceholder: 'ftp.tuosito.it',
        usernameLabel: 'Username',
        usernamePlaceholder: 'username',
        passwordLabel: 'Password',
        portLabel: 'Porta',
        protocolLabel: 'Protocollo',
        basePathLabel: 'Percorso base',
        basePathPlaceholder: '/',
        basePathHint: 'La cartella principale del sito sul server',
        productTitle: 'Seleziona Prodotto',
        productDesc: 'Quale software della iCON Suite vuoi installare?',
        productCustom: 'Personalizzato',
        productCustomDesc: 'Percorso custom',
        customPathLabel: 'Percorso personalizzato',
        customPathPlaceholder: '/nome-cartella/',
        folderTitle: 'Seleziona Cartella',
        folderDesc: 'Scegli la cartella contenente i file da caricare',
        folderBrowse: 'Sfoglia ...',
        folderNone: 'Nessuna cartella selezionata',
        folderContents: 'Contenuto della cartella:',
        folderMore: '... e altri {n} file',
        folderTotal: 'Totale: {n} file',
        folderDialogTitle: 'Seleziona la cartella da caricare',
        uploadTitle: 'Upload in corso',
        uploadPreparing: 'Preparazione upload...',
        uploadVerifying: 'Verifica cartella remota...',
        uploadConnecting: 'Connessione al server...',
        uploadInProgress: 'Upload in corso...',
        uploadComplete: 'Upload completato!',
        uploadCompleteTitle: 'Installazione completata!',
        uploadCompleteErrors: 'Upload completato con errori',
        uploadCompleteErrorsTitle: 'Completato con errori',
        uploadFilesIn: '{n} file caricati in {path}',
        uploadFilesErrors: '{ok} file caricati, {errors} errori',
        uploadLogComplete: 'Completato! {n} file caricati.',
        uploadLogErrors: 'Completato con {errors} errori su {total} file.',
        retryUpload: 'Ripeti upload',
        uploadErrorConnection: 'Errore durante la connessione',
        uploadErrorUpload: "Errore durante l'upload",
        uploadCancelled: 'Upload annullato.',
        uploadStarting: 'Avvio connessione {protocol}...',
        uploadDestination: 'Percorso destinazione: {path}',
        uploadFile: 'Upload: {file}',
        btnBack: 'Indietro',
        btnNext: 'Avanti',
        btnStartUpload: 'Inizia Upload',
        btnNewInstall: 'Nuova installazione',
        manageProfilesTitle: 'Gestione Profili',
        profilesEmpty: 'Nessun profilo salvato',
        profilesEmptyHint: 'I profili salvati appariranno qui',
        btnImport: 'Importa',
        btnExport: 'Esporta',
        btnClose: 'Chiudi',
        confirmDeleteTitle: 'Conferma eliminazione',
        confirmDeleteMsg: 'Sei sicuro di voler eliminare il profilo',
        btnCancel: 'Annulla',
        btnDelete: 'Elimina',
        folderExistsTitle: 'Cartella esistente',
        folderExistsMsg: 'La cartella {path} esiste già sul server.',
        folderExistsWarn: 'Continuando, i file esistenti verranno sovrascritti.',
        btnUpdateApp: "Aggiorna l'app",
        alertProfileName: 'Inserisci un nome per il profilo',
        alertHostUser: 'Inserisci almeno host e username',
        alertProfileSaved: 'Profilo salvato!',
        alertNoProfiles: 'Nessun profilo da esportare',
        alertExported: 'Profili esportati!',
        alertImportedN: 'Importati {n} profili!',
        alertNoProfilesFound: 'Nessun profilo trovato',
        alertHost: "Inserisci l'indirizzo del server",
        alertUser: 'Inserisci il nome utente',
        alertPass: 'Inserisci la password',
        alertProduct: 'Seleziona un prodotto',
        alertCustomPath: 'Inserisci il percorso personalizzato',
        alertFolder: 'Seleziona la cartella da caricare',
        tipManageProfiles: 'Gestione profili',
        tipSaveProfile: 'Salva profilo',
        tipDuplicate: 'Duplica',
        tipDelete: 'Elimina',
        profileCopySuffix: ' (copia)',
    },
    en: {
        offlineTitle: 'No connection',
        offlineDesc: 'iCONup requires an internet connection to work.',
        offlineRetry: 'Retry',
        stepServer: 'Server',
        stepProduct: 'Product',
        stepFolder: 'Folder',
        stepUpload: 'Upload',
        serverTitle: 'Server Configuration',
        serverDesc: 'Select a saved profile or enter new data',
        profileLabel: 'Profile',
        profileNew: '— New —',
        profileNameLabel: 'Profile name',
        profileNamePlaceholder: 'E.g.: iCONSuite',
        profileNameHint: 'A custom name to identify this site',
        hostLabel: 'Host',
        hostPlaceholder: 'ftp.yoursite.com',
        usernameLabel: 'Username',
        usernamePlaceholder: 'username',
        passwordLabel: 'Password',
        portLabel: 'Port',
        protocolLabel: 'Protocol',
        basePathLabel: 'Base path',
        basePathPlaceholder: '/',
        basePathHint: 'The main folder of the site on the server',
        productTitle: 'Select Product',
        productDesc: 'Which iCON Suite software do you want to install?',
        productCustom: 'Custom',
        productCustomDesc: 'Custom path',
        customPathLabel: 'Custom path',
        customPathPlaceholder: '/folder-name/',
        folderTitle: 'Select Folder',
        folderDesc: 'Choose the folder containing the files to upload',
        folderBrowse: 'Browse ...',
        folderNone: 'No folder selected',
        folderContents: 'Folder contents:',
        folderMore: '... and {n} more files',
        folderTotal: 'Total: {n} files',
        folderDialogTitle: 'Select the folder to upload',
        uploadTitle: 'Upload in progress',
        uploadPreparing: 'Preparing upload...',
        uploadVerifying: 'Verifying remote folder...',
        uploadConnecting: 'Connecting to server...',
        uploadInProgress: 'Uploading...',
        uploadComplete: 'Upload complete!',
        uploadCompleteTitle: 'Installation complete!',
        uploadCompleteErrors: 'Upload complete with errors',
        uploadCompleteErrorsTitle: 'Completed with errors',
        uploadFilesIn: '{n} files uploaded to {path}',
        uploadFilesErrors: '{ok} files uploaded, {errors} errors',
        uploadLogComplete: 'Complete! {n} files uploaded.',
        uploadLogErrors: 'Completed with {errors} errors on {total} files.',
        retryUpload: 'Retry upload',
        uploadErrorConnection: 'Connection error',
        uploadErrorUpload: 'Upload error',
        uploadCancelled: 'Upload cancelled.',
        uploadStarting: 'Starting {protocol} connection...',
        uploadDestination: 'Destination path: {path}',
        uploadFile: 'Upload: {file}',
        btnBack: 'Back',
        btnNext: 'Next',
        btnStartUpload: 'Start Upload',
        btnNewInstall: 'New installation',
        manageProfilesTitle: 'Profile Management',
        profilesEmpty: 'No saved profiles',
        profilesEmptyHint: 'Saved profiles will appear here',
        btnImport: 'Import',
        btnExport: 'Export',
        btnClose: 'Close',
        confirmDeleteTitle: 'Confirm deletion',
        confirmDeleteMsg: 'Are you sure you want to delete the profile',
        btnCancel: 'Cancel',
        btnDelete: 'Delete',
        folderExistsTitle: 'Folder exists',
        folderExistsMsg: 'The folder {path} already exists on the server.',
        folderExistsWarn: 'Existing files will be overwritten.',
        btnUpdateApp: 'Update app',
        alertProfileName: 'Enter a profile name',
        alertHostUser: 'Enter at least host and username',
        alertProfileSaved: 'Profile saved!',
        alertNoProfiles: 'No profiles to export',
        alertExported: 'Profiles exported!',
        alertImportedN: 'Imported {n} profiles!',
        alertNoProfilesFound: 'No profiles found',
        alertHost: 'Enter the server address',
        alertUser: 'Enter the username',
        alertPass: 'Enter the password',
        alertProduct: 'Select a product',
        alertCustomPath: 'Enter the custom path',
        alertFolder: 'Select the folder to upload',
        tipManageProfiles: 'Manage profiles',
        tipSaveProfile: 'Save profile',
        tipDuplicate: 'Duplicate',
        tipDelete: 'Delete',
        profileCopySuffix: ' (copy)',
    }
};

let currentLang = 'it';

function t(key, params) {
    let str = LANGS[currentLang][key] || LANGS['it'][key] || key;
    if (params) {
        Object.keys(params).forEach(k => {
            str = str.replace('{' + k + '}', params[k]);
        });
    }
    return str;
}

function applyLang() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        el.textContent = t(el.dataset.i18n);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        el.placeholder = t(el.dataset.i18nPlaceholder);
    });
    document.querySelectorAll('[data-i18n-title]').forEach(el => {
        el.title = t(el.dataset.i18nTitle);
    });
    const btn = document.getElementById('btn-lang');
    if (btn) btn.textContent = currentLang === 'it' ? 'EN' : 'IT';
}

function setLang(lang) {
    currentLang = lang;
    applyLang();
    try { localStorage.setItem('iconup-lang', lang); } catch(e) {}
}

function toggleLang() {
    setLang(currentLang === 'it' ? 'en' : 'it');
}

function initLang() {
    try {
        const saved = localStorage.getItem('iconup-lang');
        if (saved && LANGS[saved]) currentLang = saved;
    } catch(e) {}
    applyLang();
}
