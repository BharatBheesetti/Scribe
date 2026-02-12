from fastapi import FastAPI, File, UploadFile, HTTPException
from fastapi.responses import JSONResponse
from contextlib import asynccontextmanager
from pydantic import BaseModel
from faster_whisper import WhisperModel
import uvicorn
import os
import json
import tempfile
import torch
import asyncio
import sys
import threading
import time

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

AVAILABLE_MODELS = {
    "tiny":     {"size_mb": 75,   "description": "Fastest, least accurate"},
    "base":     {"size_mb": 142,  "description": "Fast, good accuracy"},
    "small":    {"size_mb": 466,  "description": "Balanced speed/accuracy"},
    "medium":   {"size_mb": 1500, "description": "Slower, high accuracy"},
    "large-v3": {"size_mb": 3100, "description": "Slowest, best accuracy"},
}

DEFAULT_MODEL = "base"

# ---------------------------------------------------------------------------
# App-level directories & settings
# ---------------------------------------------------------------------------

APP_DIR = os.path.join(os.getenv("APPDATA", "."), "Scribe")
MODELS_DIR = os.path.join(APP_DIR, "models")
SETTINGS_PATH = os.path.join(APP_DIR, "settings.json")


def _ensure_dirs():
    os.makedirs(APP_DIR, exist_ok=True)
    os.makedirs(MODELS_DIR, exist_ok=True)


def load_settings() -> dict:
    """Load settings from disk, returning defaults if missing or corrupt."""
    _ensure_dirs()
    if os.path.exists(SETTINGS_PATH):
        try:
            with open(SETTINGS_PATH, "r", encoding="utf-8") as f:
                return json.load(f)
        except (json.JSONDecodeError, OSError):
            pass
    return {"model_size": DEFAULT_MODEL}


def save_settings(settings: dict):
    """Persist settings to disk."""
    _ensure_dirs()
    with open(SETTINGS_PATH, "w", encoding="utf-8") as f:
        json.dump(settings, f, indent=2)


# ---------------------------------------------------------------------------
# Global state
# ---------------------------------------------------------------------------

MODEL = None
ACTIVE_MODEL_SIZE: str = DEFAULT_MODEL
DOWNLOAD_IN_PROGRESS: dict = {}  # model_size -> True while downloading


# ---------------------------------------------------------------------------
# Model helpers
# ---------------------------------------------------------------------------

def _model_is_downloaded(size: str) -> bool:
    """Check whether a model's files already exist locally."""
    # faster-whisper stores CTranslate2 models under <download_root>/models--Systran--faster-whisper-<size>
    # A simpler heuristic: try listing the expected directory pattern.
    for entry in os.listdir(MODELS_DIR) if os.path.isdir(MODELS_DIR) else []:
        if size in entry and os.path.isdir(os.path.join(MODELS_DIR, entry)):
            subdir = os.path.join(MODELS_DIR, entry)
            # Check for snapshot content (at least the model.bin or model file)
            snapshots = os.path.join(subdir, "snapshots")
            if os.path.isdir(snapshots):
                return True
            # Also check for blobs (means download in progress or complete)
            blobs = os.path.join(subdir, "blobs")
            if os.path.isdir(blobs) and len(os.listdir(blobs)) > 0:
                return True
    return False


def _load_model(size: str):
    """Load a Whisper model of the given size and return it."""
    device = "cuda" if torch.cuda.is_available() else "cpu"
    compute_type = "float16" if device == "cuda" else "int8"

    print(f"Loading Whisper model '{size}' on {device}...", flush=True)
    model = WhisperModel(
        size,
        device=device,
        compute_type=compute_type,
        download_root=MODELS_DIR,
    )
    print(f"Model '{size}' loaded successfully!", flush=True)
    return model


# ---------------------------------------------------------------------------
# Lifespan (replaces deprecated @app.on_event)
# ---------------------------------------------------------------------------

@asynccontextmanager
async def lifespan(app: FastAPI):
    global MODEL, ACTIVE_MODEL_SIZE
    _ensure_dirs()

    settings = load_settings()
    desired = settings.get("model_size", DEFAULT_MODEL)
    if desired not in AVAILABLE_MODELS:
        desired = DEFAULT_MODEL

    try:
        MODEL = _load_model(desired)
        ACTIVE_MODEL_SIZE = desired
    except Exception as exc:
        print(f"Failed to load model '{desired}': {exc}", flush=True)
        # Fall back to base if the preferred model is unavailable
        if desired != DEFAULT_MODEL:
            print(f"Falling back to '{DEFAULT_MODEL}'...", flush=True)
            try:
                MODEL = _load_model(DEFAULT_MODEL)
                ACTIVE_MODEL_SIZE = DEFAULT_MODEL
            except Exception as exc2:
                print(f"Fallback also failed: {exc2}", flush=True)

    # Persist whatever we actually loaded
    settings["model_size"] = ACTIVE_MODEL_SIZE
    save_settings(settings)

    yield  # app is running

    # Shutdown
    print("Whisper service shutting down.", flush=True)


app = FastAPI(lifespan=lifespan)


# ---------------------------------------------------------------------------
# Pydantic models
# ---------------------------------------------------------------------------

class SwitchModelRequest(BaseModel):
    model: str


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------

@app.post("/transcribe")
async def transcribe(audio: UploadFile = File(...)):
    if MODEL is None:
        raise HTTPException(status_code=503, detail="Model not loaded yet")

    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as temp_file:
        content = await audio.read()
        temp_file.write(content)
        temp_path = temp_file.name

    try:
        segments, info = MODEL.transcribe(
            temp_path,
            language=None,
            vad_filter=True,
            beam_size=5,
        )

        text_parts = [segment.text for segment in segments]
        text = " ".join(text_parts).strip()

        return {
            "text": text,
            "language": info.language,
            "duration": info.duration,
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        if os.path.exists(temp_path):
            os.remove(temp_path)


@app.get("/health")
async def health():
    return {
        "status": "ok",
        "model_loaded": MODEL is not None,
        "active_model": ACTIVE_MODEL_SIZE,
    }


# ---------------------------------------------------------------------------
# Model management endpoints
# ---------------------------------------------------------------------------

@app.get("/models")
async def list_models():
    """Return all available models with download status and active flag."""
    models = []
    for size, meta in AVAILABLE_MODELS.items():
        models.append({
            "name": size,
            "size_mb": meta["size_mb"],
            "description": meta["description"],
            "downloaded": _model_is_downloaded(size),
            "active": size == ACTIVE_MODEL_SIZE,
            "downloading": DOWNLOAD_IN_PROGRESS.get(size, False),
        })
    return {"models": models}


@app.post("/models/download/{size}")
async def download_model(size: str):
    """Trigger background download of a model. Progress is printed to stdout."""
    if size not in AVAILABLE_MODELS:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown model size '{size}'. Available: {list(AVAILABLE_MODELS.keys())}",
        )

    if DOWNLOAD_IN_PROGRESS.get(size):
        return {"status": "already_downloading", "model": size}

    if _model_is_downloaded(size):
        return {"status": "already_downloaded", "model": size}

    # Run the download in a background thread so the endpoint returns immediately
    def _background_download():
        DOWNLOAD_IN_PROGRESS[size] = True
        try:
            print(f"DOWNLOAD_START:{size}", flush=True)
            device = "cuda" if torch.cuda.is_available() else "cpu"
            compute_type = "float16" if device == "cuda" else "int8"

            # Emit synthetic progress updates while the download runs.
            # faster-whisper does not expose a progress callback, so we poll
            # the blobs directory to estimate completion.
            download_done = threading.Event()
            expected_bytes = AVAILABLE_MODELS[size]["size_mb"] * 1024 * 1024

            def _progress_reporter():
                blob_dir = None
                last_pct = -1
                while not download_done.is_set():
                    # Try to find the blob directory for this model
                    if blob_dir is None:
                        for entry in os.listdir(MODELS_DIR) if os.path.isdir(MODELS_DIR) else []:
                            if size in entry:
                                candidate = os.path.join(MODELS_DIR, entry, "blobs")
                                if os.path.isdir(candidate):
                                    blob_dir = candidate
                                    break

                    if blob_dir and os.path.isdir(blob_dir):
                        total = 0
                        for f in os.listdir(blob_dir):
                            fp = os.path.join(blob_dir, f)
                            if os.path.isfile(fp):
                                total += os.path.getsize(fp)
                        pct = min(int((total / expected_bytes) * 100), 99) if expected_bytes > 0 else 0
                        if pct != last_pct:
                            print(f"DOWNLOAD_PROGRESS:{size}:{pct}", flush=True)
                            last_pct = pct

                    download_done.wait(timeout=0.5)

            progress_thread = threading.Thread(target=_progress_reporter, daemon=True)
            progress_thread.start()

            # Actually trigger the download by loading the model
            WhisperModel(
                size,
                device=device,
                compute_type=compute_type,
                download_root=MODELS_DIR,
            )

            download_done.set()
            progress_thread.join(timeout=5)
            print(f"DOWNLOAD_PROGRESS:{size}:100", flush=True)
            print(f"DOWNLOAD_COMPLETE:{size}", flush=True)
        except Exception as exc:
            print(f"DOWNLOAD_ERROR:{size}:{exc}", flush=True)
        finally:
            DOWNLOAD_IN_PROGRESS[size] = False

    thread = threading.Thread(target=_background_download, daemon=True)
    thread.start()

    return {"status": "downloading", "model": size}


@app.post("/models/switch")
async def switch_model(request: SwitchModelRequest):
    """Switch the active model. The model must already be downloaded."""
    global MODEL, ACTIVE_MODEL_SIZE

    size = request.model
    if size not in AVAILABLE_MODELS:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown model size '{size}'. Available: {list(AVAILABLE_MODELS.keys())}",
        )

    if size == ACTIVE_MODEL_SIZE and MODEL is not None:
        return {"status": "already_active", "model": size}

    if not _model_is_downloaded(size):
        raise HTTPException(
            status_code=400,
            detail=f"Model '{size}' is not downloaded. Call POST /models/download/{size} first.",
        )

    try:
        new_model = _load_model(size)
        MODEL = new_model
        ACTIVE_MODEL_SIZE = size

        # Persist preference
        settings = load_settings()
        settings["model_size"] = size
        save_settings(settings)

        return {"status": "switched", "model": size}
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Failed to load model '{size}': {exc}")


# ---------------------------------------------------------------------------
# Stdin listener (graceful shutdown from Rust host)
# ---------------------------------------------------------------------------

async def stdin_listener():
    """Listen for shutdown command from stdin."""
    loop = asyncio.get_event_loop()
    while True:
        try:
            line = await loop.run_in_executor(None, sys.stdin.readline)
            if line.strip() == "SHUTDOWN":
                print("Received shutdown signal", flush=True)
                os._exit(0)
        except Exception:
            await asyncio.sleep(1)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    loop = asyncio.get_event_loop()
    loop.create_task(stdin_listener())

    uvicorn.run(
        app,
        host="127.0.0.1",
        port=8765,
        log_level="error",
    )
