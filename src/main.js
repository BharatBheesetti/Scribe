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
const autoStartToggle = document.getElementById('auto-start-toggle');

// Hotkey capture widget elements
const hotkeyDisplay = document.getElementById('hotkey-display');
const hotkeyDisplayMode = document.getElementById('hotkey-display-mode');
const hotkeyCaptureModeEl = document.getElementById('hotkey-capture-mode');
const hotkeyCaptureBox = document.getElementById('hotkey-capture-box');
const hotkeyCapturePreview = document.getElementById('hotkey-capture-preview');
const hotkeyCaptureError = document.getElementById('hotkey-capture-error');
const hotkeyChangeBtn = document.getElementById('hotkey-change-btn');
const hotkeySaveBtn = document.getElementById('hotkey-save-btn');
const hotkeyCancelBtn = document.getElementById('hotkey-cancel-btn');
const historyHotkeyHint = document.getElementById('history-hotkey-hint');

// Tab navigation elements
const tabSettings = document.getElementById('tab-settings');
const tabHistory = document.getElementById('tab-history');
const panelSettings = document.getElementById('panel-settings');
const panelHistory = document.getElementById('panel-history');
const historyList = document.getElementById('history-list');
const historyEmpty = document.getElementById('history-empty');
const clearHistoryBtn = document.getElementById('clear-history-btn');
const historySearch = document.getElementById('history-search');
const historySearchClear = document.getElementById('history-search-clear');
const historyResultCount = document.getElementById('history-result-count');

// Module-level state for history search
let cachedHistoryEntries = [];   // Full unfiltered list from last loadHistory() call
let historyTabVisible = false;   // Track whether History tab is currently shown
let historyRefreshInterval = null;

// Module-level state for hotkey capture
let currentHotkey = 'Ctrl+Shift+Space';  // Will be loaded from backend
let capturedHotkey = null;               // The key combo captured during capture mode
let hotkeyCaptureModeActive = false;     // Whether we're in capture mode

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
    await loadCurrentHotkey();
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
        if (autoStartToggle) autoStartToggle.checked = settings.auto_start || false;
    } catch (err) {
        console.error('Failed to load settings:', err);
    }
}

async function saveCurrentSettings() {
    try {
        const settings = {
            hotkey: currentHotkey,         // from module state (backend merges anyway)
            model_size: 'base',            // will be overridden by active model
            language: languageSelect ? languageSelect.value : 'auto',
            output_mode: outputModeSelect ? outputModeSelect.value : 'clipboard_paste',
            filler_removal: fillerRemovalToggle ? fillerRemovalToggle.checked : true,
            sound_effects: soundEffectsToggle ? soundEffectsToggle.checked : true,
            auto_start: autoStartToggle ? autoStartToggle.checked : false,
        };
        await invoke('save_settings', { newSettings: settings });
    } catch (err) {
        console.error('Failed to save settings:', err);
    }
}

// ---------------------------------------------------------------------------
// Hotkey capture widget
// ---------------------------------------------------------------------------

/**
 * Convert a canonical hotkey key name (from into_string()) to a human-friendly
 * display name. Handles HIGH-2: KeyA -> A, Digit5 -> 5, etc.
 */
function keyDisplayName(name) {
    // Strip "Key" prefix for letters: KeyA -> A
    if (/^Key[A-Z]$/.test(name)) return name.slice(3);
    // Strip "Digit" prefix for numbers: Digit5 -> 5
    if (/^Digit\d$/.test(name)) return name.slice(5);
    // Arrow keys: ArrowUp -> Up
    if (name.startsWith('Arrow')) return name.slice(5);
    // Backquote -> `
    if (name === 'Backquote') return '`';
    // Backslash -> \
    if (name === 'Backslash') return '\\';
    // BracketLeft -> [, BracketRight -> ]
    if (name === 'BracketLeft') return '[';
    if (name === 'BracketRight') return ']';
    // Comma -> ,
    if (name === 'Comma') return ',';
    // Period -> .
    if (name === 'Period') return '.';
    // Quote -> '
    if (name === 'Quote') return "'";
    // Semicolon -> ;
    if (name === 'Semicolon') return ';';
    // Slash -> /
    if (name === 'Slash') return '/';
    // Minus -> -
    if (name === 'Minus') return '-';
    // Equal -> =
    if (name === 'Equal') return '=';
    // CapsLock -> Caps Lock
    if (name === 'CapsLock') return 'Caps Lock';
    // NumLock -> Num Lock
    if (name === 'NumLock') return 'Num Lock';
    // ScrollLock -> Scroll Lock
    if (name === 'ScrollLock') return 'Scroll Lock';
    // PrintScreen -> Print Screen
    if (name === 'PrintScreen') return 'Print Screen';
    // PageUp -> Page Up, PageDown -> Page Down
    if (name === 'PageUp') return 'Page Up';
    if (name === 'PageDown') return 'Page Down';
    // Numpad keys: Numpad0 -> Num 0, NumpadAdd -> Num +
    if (/^Numpad\d$/.test(name)) return 'Num ' + name.slice(6);
    if (name === 'NumpadAdd') return 'Num +';
    if (name === 'NumpadSubtract') return 'Num -';
    if (name === 'NumpadMultiply') return 'Num *';
    if (name === 'NumpadDivide') return 'Num /';
    if (name === 'NumpadDecimal') return 'Num .';
    if (name === 'NumpadEnter') return 'Num Enter';
    if (name === 'NumpadEqual') return 'Num =';
    // F-keys, Space, Enter, Tab, etc. -- use as-is
    return name;
}

/**
 * Parse a canonical hotkey string and return display parts in conventional order.
 * Input: "shift+control+Space" (from into_string())
 * Output: ["Ctrl", "Shift", "Space"]
 *
 * MEDIUM-3: Enforces conventional modifier order: Ctrl, Shift, Alt, Super
 */
function parseHotkeyParts(hotkeyStr) {
    const parts = hotkeyStr.split('+').map(p => p.trim());
    const mods = [];
    let key = null;

    for (const part of parts) {
        switch (part.toLowerCase()) {
            case 'control':
            case 'ctrl':
                mods.push('Ctrl');
                break;
            case 'shift':
                mods.push('Shift');
                break;
            case 'alt':
            case 'option':
                mods.push('Alt');
                break;
            case 'super':
            case 'cmd':
            case 'command':
                mods.push('Super');
                break;
            default:
                key = keyDisplayName(part);
                break;
        }
    }

    // Sort modifiers in conventional order
    const order = ['Ctrl', 'Shift', 'Alt', 'Super'];
    const sortedMods = order.filter(m => mods.includes(m));

    if (key) sortedMods.push(key);
    return sortedMods;
}

/**
 * Render the hotkey display as <kbd> elements.
 * Also updates the history tab hint text.
 */
function renderHotkeyDisplay(hotkeyStr) {
    if (!hotkeyDisplay) return;
    hotkeyDisplay.textContent = ''; // Clear

    const parts = parseHotkeyParts(hotkeyStr);
    parts.forEach((part, i) => {
        if (i > 0) {
            hotkeyDisplay.appendChild(document.createTextNode(' + '));
        }
        const kbd = document.createElement('kbd');
        kbd.textContent = part;
        hotkeyDisplay.appendChild(kbd);
    });

    // Also update the history hint
    if (historyHotkeyHint) {
        historyHotkeyHint.textContent = parts.join(' + ');
    }
}

/**
 * Load the current hotkey from the backend and render it.
 */
async function loadCurrentHotkey() {
    try {
        const hk = await invoke('get_current_hotkey');
        currentHotkey = hk;
        renderHotkeyDisplay(hk);
    } catch (err) {
        console.error('Failed to load current hotkey:', err);
        renderHotkeyDisplay('Ctrl+Shift+Space');
    }
}

/**
 * Map DOM KeyboardEvent.code to global-hotkey key names.
 */
function mapDomCodeToHotkeyKey(code) {
    const directMap = {
        'Space': 'Space', 'Enter': 'Enter', 'Tab': 'Tab',
        'Backspace': 'Backspace', 'Delete': 'Delete',
        'Home': 'Home', 'End': 'End', 'PageUp': 'PageUp', 'PageDown': 'PageDown',
        'Insert': 'Insert', 'Escape': 'Escape',
        'ArrowUp': 'ArrowUp', 'ArrowDown': 'ArrowDown',
        'ArrowLeft': 'ArrowLeft', 'ArrowRight': 'ArrowRight',
        'PrintScreen': 'PrintScreen', 'ScrollLock': 'ScrollLock',
        'Pause': 'Pause', 'NumLock': 'NumLock',
        'Backquote': 'Backquote', 'Backslash': 'Backslash',
        'BracketLeft': 'BracketLeft', 'BracketRight': 'BracketRight',
        'Comma': 'Comma', 'Period': 'Period', 'Quote': 'Quote',
        'Semicolon': 'Semicolon', 'Slash': 'Slash',
        'Minus': 'Minus', 'Equal': 'Equal',
        'CapsLock': 'CapsLock',
    };

    if (directMap[code]) return directMap[code];

    // KeyA-KeyZ
    if (/^Key[A-Z]$/.test(code)) return code;

    // Digit0-Digit9
    if (/^Digit[0-9]$/.test(code)) return code;

    // F1-F24
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;

    // Numpad keys
    if (/^Numpad\d$/.test(code)) return code;
    if (code === 'NumpadAdd') return 'NumpadAdd';
    if (code === 'NumpadSubtract') return 'NumpadSubtract';
    if (code === 'NumpadMultiply') return 'NumpadMultiply';
    if (code === 'NumpadDivide') return 'NumpadDivide';
    if (code === 'NumpadDecimal') return 'NumpadDecimal';
    if (code === 'NumpadEnter') return 'NumpadEnter';
    if (code === 'NumpadEqual') return 'NumpadEqual';

    // Unknown key
    return null;
}

function showCaptureError(msg) {
    if (hotkeyCaptureError) {
        hotkeyCaptureError.textContent = msg;
        hotkeyCaptureError.classList.remove('hidden');
    }
}

function preventDefaultDuringCapture(e) {
    if (hotkeyCaptureModeActive) {
        e.preventDefault();
        e.stopPropagation();
    }
}

function handleHotkeyCapture(e) {
    e.preventDefault();
    e.stopPropagation();

    // Ignore pure modifier key presses (wait for the actual key)
    const modifierKeys = ['Control', 'Shift', 'Alt', 'Meta'];
    if (modifierKeys.includes(e.key)) return;

    // Escape cancels capture mode (no matter what modifiers are held)
    if (e.key === 'Escape') {
        exitCaptureMode();
        return;
    }

    // Build modifier string in conventional order (MEDIUM-3)
    const mods = [];
    if (e.ctrlKey) mods.push('Ctrl');
    if (e.shiftKey) mods.push('Shift');
    if (e.altKey) mods.push('Alt');

    // MEDIUM-4: Warn about Windows/Super key
    if (e.metaKey) {
        showCaptureError('The Windows key is not supported as a hotkey modifier. Most Win+key combos are intercepted by Windows.');
        return;
    }

    // Map e.code to the global-hotkey key name
    const keyName = mapDomCodeToHotkeyKey(e.code);

    if (!keyName) {
        showCaptureError('That key is not supported as a hotkey.');
        return;
    }

    // Validate: must have at least one modifier, UNLESS it's an F-key
    const isFKey = /^F\d+$/.test(keyName);
    if (mods.length === 0 && !isFKey) {
        showCaptureError('Hotkey must include at least one modifier (Ctrl, Alt, Shift) unless it is an F-key.');
        return;
    }

    // Build the hotkey string
    const hotkeyStr = [...mods, keyName].join('+');
    capturedHotkey = hotkeyStr;

    // Show preview
    if (hotkeyCaptureBox) hotkeyCaptureBox.classList.add('hidden');
    if (hotkeyCapturePreview) {
        hotkeyCapturePreview.classList.remove('hidden');
        hotkeyCapturePreview.textContent = '';

        const displayParts = parseHotkeyParts(hotkeyStr);
        displayParts.forEach((part, i) => {
            if (i > 0) hotkeyCapturePreview.appendChild(document.createTextNode(' + '));
            const kbd = document.createElement('kbd');
            kbd.textContent = part;
            hotkeyCapturePreview.appendChild(kbd);
        });
    }

    if (hotkeyCaptureError) hotkeyCaptureError.classList.add('hidden');
    if (hotkeySaveBtn) hotkeySaveBtn.classList.remove('hidden');
}

async function enterCaptureMode() {
    hotkeyCaptureModeActive = true;
    capturedHotkey = null;

    if (hotkeyDisplayMode) hotkeyDisplayMode.classList.add('hidden');
    if (hotkeyCaptureModeEl) hotkeyCaptureModeEl.classList.remove('hidden');

    // Reset to listening state
    if (hotkeyCaptureBox) hotkeyCaptureBox.classList.remove('hidden');
    if (hotkeyCapturePreview) hotkeyCapturePreview.classList.add('hidden');
    if (hotkeyCaptureError) hotkeyCaptureError.classList.add('hidden');
    if (hotkeySaveBtn) hotkeySaveBtn.classList.add('hidden');

    // MEDIUM-2: Temporarily unregister recording hotkey during capture
    // to prevent it from firing while user presses keys
    try {
        await invoke('pause_hotkey');
    } catch (err) {
        console.error('Failed to pause hotkey during capture:', err);
    }

    // Add keydown listener
    document.addEventListener('keydown', handleHotkeyCapture);
    document.addEventListener('keydown', preventDefaultDuringCapture, true);
}

async function exitCaptureMode() {
    hotkeyCaptureModeActive = false;
    capturedHotkey = null;

    if (hotkeyDisplayMode) hotkeyDisplayMode.classList.remove('hidden');
    if (hotkeyCaptureModeEl) hotkeyCaptureModeEl.classList.add('hidden');

    document.removeEventListener('keydown', handleHotkeyCapture);
    document.removeEventListener('keydown', preventDefaultDuringCapture, true);

    // MEDIUM-2: Re-register recording hotkey after capture mode ends
    try {
        await invoke('resume_hotkey');
    } catch (err) {
        console.error('Failed to resume hotkey after capture:', err);
    }
}

async function saveHotkey() {
    if (!capturedHotkey) return;

    if (hotkeySaveBtn) {
        hotkeySaveBtn.disabled = true;
        hotkeySaveBtn.textContent = 'Saving...';
    }

    try {
        // The backend validates, registers new, unregisters old, and persists
        const normalized = await invoke('set_hotkey', { hotkey: capturedHotkey });
        currentHotkey = normalized;
        renderHotkeyDisplay(normalized);

        // Exit capture mode but do NOT re-register via resume_hotkey
        // because set_hotkey already registered the new one
        hotkeyCaptureModeActive = false;
        capturedHotkey = null;

        if (hotkeyDisplayMode) hotkeyDisplayMode.classList.remove('hidden');
        if (hotkeyCaptureModeEl) hotkeyCaptureModeEl.classList.add('hidden');

        document.removeEventListener('keydown', handleHotkeyCapture);
        document.removeEventListener('keydown', preventDefaultDuringCapture, true);
    } catch (err) {
        showCaptureError(typeof err === 'string' ? err : 'Failed to set hotkey. Try a different combination.');
    } finally {
        if (hotkeySaveBtn) {
            hotkeySaveBtn.disabled = false;
            hotkeySaveBtn.textContent = 'Save';
        }
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
        historyTabVisible = false;
        stopHistoryRefresh();
        if (historySearch) historySearch.value = '';
        if (historySearchClear) historySearchClear.classList.add('hidden');
        if (historyResultCount) historyResultCount.classList.add('hidden');
        // Clean up any no-results element
        const noRes = panelHistory.querySelector('.history-no-results');
        if (noRes) noRes.remove();
    } else if (tabName === 'history') {
        tabHistory.classList.add('tab-active');
        panelHistory.classList.remove('hidden');
        loadHistory();
        historyTabVisible = true;
        startHistoryRefresh();
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function debounce(fn, ms) {
    let timer;
    return function(...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), ms);
    };
}

function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
              .replace(/"/g, '&quot;');
}

/**
 * Highlights matching substrings in text by wrapping them in <mark> tags.
 * Uses split-map-join on RAW text, then escapes each fragment.
 * This ensures HTML-special characters in text are handled correctly.
 */
function highlightMatches(text, query) {
    if (!query) return escapeHtml(text);

    // Escape regex special characters in the RAW query
    const regexSafe = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${regexSafe})`, 'gi');

    // Split RAW text by matches. split() with a capture group
    // includes the matched text in the result array.
    const parts = text.split(regex);

    // HTML-escape each part, wrap matches in <mark>
    return parts.map(part => {
        if (regex.test(part)) {
            regex.lastIndex = 0;
            return '<mark>' + escapeHtml(part) + '</mark>';
        }
        regex.lastIndex = 0;
        return escapeHtml(part);
    }).join('');
}

function filterHistory() {
    const query = historySearch ? historySearch.value.trim() : '';

    // Show/hide clear button based on whether input has content
    if (historySearchClear) {
        historySearchClear.classList.toggle('hidden', query.length === 0);
    }

    if (!query) {
        // Hide result count when not searching
        if (historyResultCount) historyResultCount.classList.add('hidden');
        renderHistory(cachedHistoryEntries);
        return;
    }

    const lowerQuery = query.toLowerCase();
    const filtered = cachedHistoryEntries.filter(entry =>
        entry.text.toLowerCase().includes(lowerQuery)
    );

    // Show result count: "X of Y"
    if (historyResultCount) {
        historyResultCount.textContent = filtered.length + ' of ' + cachedHistoryEntries.length;
        historyResultCount.classList.remove('hidden');
    }

    renderHistory(filtered, query);
}

function startHistoryRefresh() {
    if (historyRefreshInterval) return;
    historyRefreshInterval = setInterval(() => {
        if (historyTabVisible) {
            loadHistory();
        }
    }, 5000);
}

function stopHistoryRefresh() {
    if (historyRefreshInterval) {
        clearInterval(historyRefreshInterval);
        historyRefreshInterval = null;
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
        cachedHistoryEntries = entries;
        // Apply current search filter (or render all if no query)
        filterHistory();
    } catch (err) {
        console.error('Failed to load history:', err);
    }
}

function renderHistory(entries, query = '') {
    // Clear previous content
    historyList.textContent = '';

    if (entries.length === 0) {
        historyList.classList.add('hidden');

        if (query) {
            // Active search with no results -- show "no results" message
            // SECURITY: Use createElement + textContent, NEVER innerHTML with user input
            historyEmpty.classList.add('hidden');
            const noResults = document.createElement('div');
            noResults.className = 'history-no-results';

            const msg = document.createElement('p');
            msg.appendChild(document.createTextNode('No results found for \u201c'));
            const querySpan = document.createElement('span');
            querySpan.className = 'search-query';
            querySpan.textContent = query;  // textContent = XSS-safe
            msg.appendChild(querySpan);
            msg.appendChild(document.createTextNode('\u201d'));
            noResults.appendChild(msg);

            // Remove any previous no-results element
            const prev = historyList.parentNode.querySelector('.history-no-results');
            if (prev) prev.remove();

            historyList.parentNode.insertBefore(noResults, historyList.nextSibling);
            clearHistoryBtn.disabled = true;
        } else {
            // No search, genuinely empty history
            // Remove any lingering no-results element
            const prev = historyList.parentNode.querySelector('.history-no-results');
            if (prev) prev.remove();
            historyEmpty.classList.remove('hidden');
            clearHistoryBtn.disabled = true;
        }
        return;
    }

    historyList.classList.remove('hidden');
    historyEmpty.classList.add('hidden');
    clearHistoryBtn.disabled = false;

    // Clean up any lingering no-results element
    const prevNoResults = historyList.parentNode.querySelector('.history-no-results');
    if (prevNoResults) prevNoResults.remove();

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
        if (query) {
            textP.innerHTML = highlightMatches(entry.text, query);
            // innerHTML is safe here: highlightMatches escapes all text fragments
            // before wrapping matches in <mark> tags
        } else {
            textP.textContent = entry.text;
        }
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
        cachedHistoryEntries = [];
        if (historySearch) historySearch.value = '';
        if (historySearchClear) historySearchClear.classList.add('hidden');
        if (historyResultCount) historyResultCount.classList.add('hidden');
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

    // History search -- debounced input filtering
    if (historySearch) {
        historySearch.addEventListener('input', debounce(filterHistory, 200));
    }

    // History search clear button
    if (historySearchClear) {
        historySearchClear.addEventListener('click', () => {
            if (historySearch) {
                historySearch.value = '';
                filterHistory();
                historySearch.focus();
            }
        });
    }

    // Hotkey capture widget
    if (hotkeyChangeBtn) {
        hotkeyChangeBtn.addEventListener('click', enterCaptureMode);
    }
    if (hotkeySaveBtn) {
        hotkeySaveBtn.addEventListener('click', saveHotkey);
    }
    if (hotkeyCancelBtn) {
        hotkeyCancelBtn.addEventListener('click', exitCaptureMode);
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

    // Auto-start toggle — uses dedicated command, not saveCurrentSettings
    if (autoStartToggle) {
        autoStartToggle.addEventListener('change', async () => {
            const enabled = autoStartToggle.checked;
            try {
                await invoke('set_auto_start', { enabled });
            } catch (err) {
                console.error('Failed to set auto-start:', err);
                // Revert toggle on failure so UI stays in sync
                autoStartToggle.checked = !enabled;
            }
        });
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
