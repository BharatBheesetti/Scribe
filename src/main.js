// ---------------------------------------------------------------------------
// Scribe Settings UI
// ---------------------------------------------------------------------------

const API_BASE = 'http://127.0.0.1:8765';
const POLL_INTERVAL_MS = 2000;

// DOM references
const setupScreen   = document.getElementById('setup-screen');
const settingsScreen = document.getElementById('settings-screen');
const errorOverlay  = document.getElementById('error-overlay');

const setupProgressBar = document.getElementById('setup-progress-bar');
const setupStatus      = document.getElementById('setup-status');
const setupModelSize   = document.getElementById('setup-model-size');

const activeModelName = document.getElementById('active-model-name');
const activeModelDesc = document.getElementById('active-model-desc');
const modelList       = document.getElementById('model-list');

let pollTimer = null;
let downloadPollTimer = null;

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
// API helpers
// ---------------------------------------------------------------------------

async function fetchHealth() {
    const resp = await fetch(`${API_BASE}/health`);
    if (!resp.ok) throw new Error(`Health check failed: ${resp.status}`);
    return resp.json();
}

async function fetchModels() {
    const resp = await fetch(`${API_BASE}/models`);
    if (!resp.ok) throw new Error(`Models fetch failed: ${resp.status}`);
    return resp.json();
}

async function requestDownload(modelName) {
    const resp = await fetch(`${API_BASE}/models/download/${modelName}`, { method: 'POST' });
    if (!resp.ok) throw new Error(`Download request failed: ${resp.status}`);
    return resp.json();
}

async function requestSwitch(modelName) {
    const resp = await fetch(`${API_BASE}/models/switch`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: modelName }),
    });
    if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err.detail || `Switch failed: ${resp.status}`);
    }
    return resp.json();
}

// ---------------------------------------------------------------------------
// First-run setup flow
// ---------------------------------------------------------------------------

function startSetupPolling() {
    let progress = 0;
    let serviceReached = false;

    async function tick() {
        try {
            const health = await fetchHealth();
            serviceReached = true;

            if (health.model_loaded) {
                // Model is ready - setup complete
                setupProgressBar.style.width = '100%';
                setupStatus.textContent = 'Model loaded! Starting Scribe...';
                stopPolling();

                // Brief pause so the user sees 100%, then switch to settings
                setTimeout(() => {
                    showSettingsScreen();
                }, 800);
                return;
            }

            // Service is running but model still loading/downloading
            // Animate progress bar to convey activity (we don't have exact %)
            if (progress < 90) {
                // Approach 90% asymptotically while waiting
                progress += (90 - progress) * 0.08;
            }
            setupProgressBar.style.width = `${Math.round(progress)}%`;
            setupStatus.textContent = 'Downloading and loading model...';

        } catch (err) {
            // Service not reachable yet
            if (!serviceReached) {
                setupStatus.textContent = 'Waiting for Scribe service to start...';
            } else {
                setupStatus.textContent = 'Connection lost, retrying...';
            }
        }
    }

    tick();
    pollTimer = setInterval(tick, POLL_INTERVAL_MS);
}

function stopPolling() {
    if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
}

// ---------------------------------------------------------------------------
// Settings screen
// ---------------------------------------------------------------------------

async function showSettingsScreen() {
    showScreen(settingsScreen);
    await refreshModels();
}

async function refreshModels() {
    try {
        const [health, modelsData] = await Promise.all([fetchHealth(), fetchModels()]);
        const models = modelsData.models;

        // Update active model card
        const active = models.find(m => m.active);
        if (active) {
            activeModelName.textContent = active.name.charAt(0).toUpperCase() + active.name.slice(1);
            activeModelDesc.textContent = `${active.description} (${active.size_mb}MB)`;
        }

        // Render model list
        renderModelList(models);

    } catch (err) {
        console.error('Failed to refresh models:', err);
    }
}

function renderModelList(models) {
    modelList.innerHTML = '';

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
        } else if (model.downloading) {
            dotClass = 'status-downloading';
            statusLabel = 'Downloading...';
        } else if (model.downloaded) {
            dotClass = 'status-downloaded';
            statusLabel = 'Downloaded';
        }

        // Action button
        let buttonHtml = '';
        if (model.active) {
            buttonHtml = `<button class="btn btn-active" disabled>Active</button>`;
        } else if (model.downloading) {
            buttonHtml = `<button class="btn btn-downloading" disabled>Downloading...</button>`;
        } else if (model.downloaded) {
            buttonHtml = `<button class="btn btn-switch" data-model="${model.name}">Switch</button>`;
        } else {
            buttonHtml = `<button class="btn btn-download" data-model="${model.name}">Download</button>`;
        }

        card.innerHTML = `
            <div class="model-card-header">
                <span class="status-dot ${dotClass}"></span>
                <span class="model-name">${model.name}</span>
                <span class="model-size">${model.size_mb}MB</span>
            </div>
            <div class="model-card-body">
                <span class="model-description">${model.description}</span>
                <span class="model-status-label">${statusLabel}</span>
            </div>
            <div class="model-card-actions">
                ${buttonHtml}
            </div>
        `;

        modelList.appendChild(card);
    }

    // Attach event listeners
    modelList.querySelectorAll('.btn-download').forEach(btn => {
        btn.addEventListener('click', handleDownload);
    });
    modelList.querySelectorAll('.btn-switch').forEach(btn => {
        btn.addEventListener('click', handleSwitch);
    });
}

async function handleDownload(e) {
    const modelName = e.target.dataset.model;
    e.target.textContent = 'Starting...';
    e.target.disabled = true;

    try {
        await requestDownload(modelName);
        // Start polling for download completion
        startDownloadPolling();
    } catch (err) {
        console.error('Download failed:', err);
        e.target.textContent = 'Download';
        e.target.disabled = false;
    }
}

function startDownloadPolling() {
    if (downloadPollTimer) return;

    downloadPollTimer = setInterval(async () => {
        try {
            const modelsData = await fetchModels();
            const anyDownloading = modelsData.models.some(m => m.downloading);
            renderModelList(modelsData.models);

            if (!anyDownloading) {
                clearInterval(downloadPollTimer);
                downloadPollTimer = null;
            }
        } catch (err) {
            console.error('Download poll failed:', err);
        }
    }, POLL_INTERVAL_MS);
}

async function handleSwitch(e) {
    const modelName = e.target.dataset.model;
    e.target.textContent = 'Switching...';
    e.target.disabled = true;

    try {
        await requestSwitch(modelName);
        await refreshModels();
    } catch (err) {
        console.error('Switch failed:', err);
        e.target.textContent = 'Switch';
        e.target.disabled = false;
        alert(`Failed to switch model: ${err.message}`);
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

async function init() {
    try {
        const health = await fetchHealth();

        if (health.model_loaded) {
            // Model already loaded - go straight to settings
            showSettingsScreen();
        } else {
            // Model still loading - show setup screen
            showScreen(setupScreen);
            startSetupPolling();
        }
    } catch (err) {
        // Service not reachable - show setup screen (will poll until available)
        console.warn('Service not reachable, entering setup mode:', err.message);
        showScreen(setupScreen);
        startSetupPolling();
    }
}

init();
