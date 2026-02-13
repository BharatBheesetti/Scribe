// ---------------------------------------------------------------------------
// Scribe Settings UI — Tauri IPC (no HTTP, no Python)
// ---------------------------------------------------------------------------

// DOM references
const setupScreen = document.getElementById('setup-screen');
const settingsScreen = document.getElementById('settings-screen');
const errorOverlay = document.getElementById('error-overlay');

const setupProgressBar = document.getElementById('setup-progress-bar');
const setupStatus = document.getElementById('setup-status');
const setupModelSize = document.getElementById('setup-model-size');

const activeModelName = document.getElementById('active-model-name');
const activeModelDesc = document.getElementById('active-model-desc');
const modelList = document.getElementById('model-list');

const languageSelect = document.getElementById('language-select');
const outputModeSelect = document.getElementById('output-mode-select');
const fillerRemovalToggle = document.getElementById('filler-removal-toggle');
const soundEffectsToggle = document.getElementById('sound-effects-toggle');

// Tab navigation elements
const tabSettings = document.getElementById('tab-settings');
const tabHistory = document.getElementById('tab-history');
const panelSettings = document.getElementById('panel-settings');
const panelHistory = document.getElementById('panel-history');
const historyList = document.getElementById('history-list');
const historyEmpty = document.getElementById('history-empty');
const clearHistoryBtn = document.getElementById('clear-history-btn');

// ---------------------------------------------------------------------------
// Tauri IPC helpers
// ---------------------------------------------------------------------------

function waitForTauri() {
    return new Promise((resolve) => {
        function check() {
            if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.event) {
                resolve(window.__TAURI__);
            } else {
                setTimeout(check, 50);
            }
        }
        check();
    });
}

let tauriApi = null;

async function invoke(cmd, args) {
    if (!tauriApi) tauriApi = await waitForTauri();
    return tauriApi.core.invoke(cmd, args);
}

async function listen(event, handler) {
    if (!tauriApi) tauriApi = await waitForTauri();
    return tauriApi.event.listen(event, handler);
}

// ---------------------------------------------------------------------------
// Screen management
// ---------------------------------------------------------------------------

function showScreen(screen) {
    setupScreen.classList.add('hidden');
    settingsScreen.classList.add('hidden');
    errorOverlay.classList.add('hidden');
    screen.classList.remove('hidden');
}

// ---------------------------------------------------------------------------
// Settings screen
// ---------------------------------------------------------------------------

async function showSettingsScreen() {
    showScreen(settingsScreen);
    await refreshModels();
    await loadSettings();
}

// ---------------------------------------------------------------------------
// Settings persistence (language, output mode)
// ---------------------------------------------------------------------------

async function loadSettings() {
    try {
        const settings = await invoke('get_settings');
        if (languageSelect) languageSelect.value = settings.language || 'auto';
        if (outputModeSelect) outputModeSelect.value = settings.output_mode || 'clipboard_paste';
        if (fillerRemovalToggle) fillerRemovalToggle.checked = settings.filler_removal !== false;
        if (soundEffectsToggle) soundEffectsToggle.checked = settings.sound_effects !== false;
    } catch (err) {
        console.error('Failed to load settings:', err);
    }
}

async function saveCurrentSettings() {
    try {
        const settings = {
            hotkey: 'Ctrl+Shift+Space',  // hardcoded for now
            model_size: 'base',           // will be overridden by active model
            language: languageSelect ? languageSelect.value : 'auto',
            output_mode: outputModeSelect ? outputModeSelect.value : 'clipboard_paste',
            filler_removal: fillerRemovalToggle ? fillerRemovalToggle.checked : true,
            sound_effects: soundEffectsToggle ? soundEffectsToggle.checked : true,
        };
        await invoke('save_settings', { newSettings: settings });
    } catch (err) {
        console.error('Failed to save settings:', err);
    }
}

async function refreshModels() {
    try {
        const info = await invoke('get_app_info');
        const models = info.models;

        // Update active model card
        const active = models.find(m => m.active);
        if (active) {
            activeModelName.textContent = active.name.charAt(0).toUpperCase() + active.name.slice(1);
            activeModelDesc.textContent = `${active.description} (${active.size_mb}MB)`;
        } else {
            activeModelName.textContent = 'No model loaded';
            activeModelDesc.textContent = 'Download a model below to get started.';
        }

        renderModelList(models);
    } catch (err) {
        console.error('Failed to refresh models:', err);
    }
}

function renderModelList(models) {
    modelList.textContent = '';

    for (const model of models) {
        const card = document.createElement('div');
        card.className = 'model-card';
        if (model.active) card.classList.add('model-active');

        // Status dot
        let dotClass = 'status-available';
        let statusLabel = 'Not downloaded';
        if (model.active) {
            dotClass = 'status-loaded';
            statusLabel = 'Active';
        } else if (model.downloaded) {
            dotClass = 'status-downloaded';
            statusLabel = 'Downloaded';
        }

        // Header
        const header = document.createElement('div');
        header.className = 'model-card-header';

        const dot = document.createElement('span');
        dot.className = 'status-dot ' + dotClass;
        header.appendChild(dot);

        const nameSpan = document.createElement('span');
        nameSpan.className = 'model-name';
        nameSpan.textContent = model.name;
        header.appendChild(nameSpan);

        const sizeSpan = document.createElement('span');
        sizeSpan.className = 'model-size';
        sizeSpan.textContent = model.size_mb + 'MB';
        header.appendChild(sizeSpan);

        card.appendChild(header);

        // Body
        const body = document.createElement('div');
        body.className = 'model-card-body';

        const descSpan = document.createElement('span');
        descSpan.className = 'model-description';
        descSpan.textContent = model.description;
        body.appendChild(descSpan);

        const statusSpan = document.createElement('span');
        statusSpan.className = 'model-status-label';
        statusSpan.textContent = statusLabel;
        body.appendChild(statusSpan);

        card.appendChild(body);

        // Actions
        const actions = document.createElement('div');
        actions.className = 'model-card-actions';

        const btn = document.createElement('button');
        btn.className = 'btn';
        if (model.active) {
            btn.classList.add('btn-active');
            btn.disabled = true;
            btn.textContent = 'Active';
        } else if (model.downloaded) {
            btn.classList.add('btn-switch');
            btn.dataset.model = model.name;
            btn.textContent = 'Switch';
            btn.addEventListener('click', handleSwitch);
        } else {
            btn.classList.add('btn-download');
            btn.dataset.model = model.name;
            btn.dataset.size = String(model.size_mb);
            btn.textContent = 'Download (' + model.size_mb + 'MB)';
            btn.addEventListener('click', handleDownload);
        }

        actions.appendChild(btn);
        card.appendChild(actions);

        modelList.appendChild(card);
    }
}

// ---------------------------------------------------------------------------
// Tab navigation
// ---------------------------------------------------------------------------

function switchTab(tabName) {
    // Update tab buttons
    tabSettings.classList.remove('tab-active');
    tabHistory.classList.remove('tab-active');

    // Update panels
    panelSettings.classList.add('hidden');
    panelHistory.classList.add('hidden');

    if (tabName === 'settings') {
        tabSettings.classList.add('tab-active');
        panelSettings.classList.remove('hidden');
    } else if (tabName === 'history') {
        tabHistory.classList.add('tab-active');
        panelHistory.classList.remove('hidden');
        loadHistory();
    }
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

function formatTimestamp(ts) {
    // The backend stores timestamps as Unix epoch seconds (string)
    const epoch = parseInt(ts, 10);
    if (isNaN(epoch)) return ts;
    const date = new Date(epoch * 1000);
    return date.toLocaleString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
    });
}

function formatDuration(seconds) {
    if (seconds < 1) return '<1s';
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    if (mins > 0) return mins + 'm ' + secs + 's';
    return secs + 's';
}

async function loadHistory() {
    try {
        const history = await invoke('get_history');
        const entries = history.entries || [];
        renderHistory(entries);
    } catch (err) {
        console.error('Failed to load history:', err);
    }
}

function renderHistory(entries) {
    // Clear previous content
    historyList.textContent = '';

    if (entries.length === 0) {
        historyList.classList.add('hidden');
        historyEmpty.classList.remove('hidden');
        clearHistoryBtn.disabled = true;
        return;
    }

    historyList.classList.remove('hidden');
    historyEmpty.classList.add('hidden');
    clearHistoryBtn.disabled = false;

    for (const entry of entries) {
        const card = document.createElement('div');
        card.className = 'history-entry';

        // Header row: timestamp + duration
        const header = document.createElement('div');
        header.className = 'history-entry-header';

        const tsSpan = document.createElement('span');
        tsSpan.className = 'history-timestamp';
        tsSpan.textContent = formatTimestamp(entry.timestamp);
        header.appendChild(tsSpan);

        const durSpan = document.createElement('span');
        durSpan.className = 'history-duration';
        durSpan.textContent = formatDuration(entry.duration_seconds);
        header.appendChild(durSpan);

        card.appendChild(header);

        // Text preview (CSS clamps to 3 lines)
        const textP = document.createElement('p');
        textP.className = 'history-text';
        textP.textContent = entry.text;
        card.appendChild(textP);

        // Footer: model/language info + copy button
        const footer = document.createElement('div');
        footer.className = 'history-entry-footer';

        const metaSpan = document.createElement('span');
        metaSpan.className = 'history-meta';
        metaSpan.textContent = entry.model + ' / ' + entry.language;
        footer.appendChild(metaSpan);

        const copyBtn = document.createElement('button');
        copyBtn.className = 'btn btn-copy';
        copyBtn.textContent = 'Copy';
        copyBtn.addEventListener('click', () => {
            navigator.clipboard.writeText(entry.text).then(() => {
                copyBtn.textContent = 'Copied!';
                setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
            }).catch(() => {
                copyBtn.textContent = 'Failed';
                setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
            });
        });
        footer.appendChild(copyBtn);

        card.appendChild(footer);
        historyList.appendChild(card);
    }
}

async function handleClearHistory() {
    try {
        await invoke('clear_history');
        renderHistory([]);
    } catch (err) {
        console.error('Failed to clear history:', err);
    }
}

// ---------------------------------------------------------------------------
// Download flow
// ---------------------------------------------------------------------------

async function handleDownload(e) {
    const modelName = e.target.dataset.model;
    const sizeMb = e.target.dataset.size;

    // Switch to setup screen for download progress
    showScreen(setupScreen);
    setupModelSize.textContent = sizeMb;
    setupProgressBar.style.width = '0%';
    setupStatus.textContent = 'Starting download...';

    try {
        // This call will emit progress events and return when complete
        await invoke('download_model_cmd', { name: modelName });

        // Download + load complete
        setupProgressBar.style.width = '100%';
        setupStatus.textContent = 'Model loaded!';

        setTimeout(() => {
            showSettingsScreen();
        }, 800);
    } catch (err) {
        console.error('Download failed:', err);
        setupStatus.textContent = `Download failed: ${err}`;

        // Go back to settings after a moment
        setTimeout(() => {
            showSettingsScreen();
        }, 3000);
    }
}

async function handleSwitch(e) {
    const modelName = e.target.dataset.model;
    e.target.textContent = 'Loading...';
    e.target.disabled = true;

    try {
        await invoke('switch_model_cmd', { name: modelName });
        await refreshModels();
    } catch (err) {
        console.error('Switch failed:', err);
        e.target.textContent = 'Switch';
        e.target.disabled = false;
        console.error(`Failed to switch model: ${err}`);
    }
}

// ---------------------------------------------------------------------------
// Event listeners for download progress
// ---------------------------------------------------------------------------

async function setupEventListeners() {
    // Tab navigation
    if (tabSettings) {
        tabSettings.addEventListener('click', () => switchTab('settings'));
    }
    if (tabHistory) {
        tabHistory.addEventListener('click', () => switchTab('history'));
    }

    // Clear history button
    if (clearHistoryBtn) {
        clearHistoryBtn.addEventListener('click', handleClearHistory);
    }

    // Settings dropdowns — save on change
    if (languageSelect) {
        languageSelect.addEventListener('change', saveCurrentSettings);
    }
    if (outputModeSelect) {
        outputModeSelect.addEventListener('change', saveCurrentSettings);
    }
    if (fillerRemovalToggle) {
        fillerRemovalToggle.addEventListener('change', saveCurrentSettings);
    }
    if (soundEffectsToggle) {
        soundEffectsToggle.addEventListener('change', saveCurrentSettings);
    }

    // Download progress from Rust backend
    await listen('model-download-progress', (event) => {
        const { progress, downloaded_mb, total_mb } = event.payload;
        setupProgressBar.style.width = `${progress}%`;
        setupStatus.textContent = `Downloading... ${downloaded_mb}MB / ${total_mb}MB`;
    });

    // Model ready (loaded into memory)
    await listen('model-ready', () => {
        // If we're on the setup screen, transition to settings
        if (!setupScreen.classList.contains('hidden')) {
            setupProgressBar.style.width = '100%';
            setupStatus.textContent = 'Model loaded!';
            setTimeout(() => {
                showSettingsScreen();
            }, 800);
        }
    });

    // Show history tab when tray "History" menu item is clicked
    await listen('show-history', () => {
        showScreen(settingsScreen);
        switchTab('history');
    });
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

async function init() {
    await setupEventListeners();

    try {
        const info = await invoke('get_app_info');

        if (info.model_loaded) {
            // Model already loaded — show settings
            showSettingsScreen();
        } else {
            // No model yet — check if any model needs downloading
            const hasDownloaded = info.models.some(m => m.downloaded);
            if (hasDownloaded) {
                // Model exists on disk but not loaded yet (loading in progress)
                showScreen(setupScreen);
                setupStatus.textContent = 'Loading model...';
                setupProgressBar.style.width = '50%';
            } else {
                // First run — show settings so user can trigger download
                showSettingsScreen();
            }
        }
    } catch (err) {
        console.warn('Init error (backend may still be starting):', err);
        // Show settings and let user trigger actions
        showSettingsScreen();
    }
}

init();
