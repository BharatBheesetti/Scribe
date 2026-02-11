from fastapi import FastAPI, File, UploadFile, HTTPException
from faster_whisper import WhisperModel
import uvicorn
import os
import tempfile
import torch
import asyncio
import sys

app = FastAPI()
MODEL = None

@app.on_event("startup")
async def load_model():
    global MODEL

    # Determine device
    device = "cuda" if torch.cuda.is_available() else "cpu"
    compute_type = "float16" if device == "cuda" else "int8"

    # Model directory
    model_dir = os.path.join(os.getenv("APPDATA", "."), "Maata", "models")
    os.makedirs(model_dir, exist_ok=True)

    print(f"Loading Whisper model on {device}...", flush=True)

    # Load medium model as per SPEC.md
    MODEL = WhisperModel(
        "medium",
        device=device,
        compute_type=compute_type,
        download_root=model_dir
    )

    print("Model loaded successfully!", flush=True)

@app.post("/transcribe")
async def transcribe(audio: UploadFile = File(...)):
    if MODEL is None:
        raise HTTPException(status_code=500, detail="Model not loaded")

    # Save uploaded file to temp
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as temp_file:
        content = await audio.read()
        temp_file.write(content)
        temp_path = temp_file.name

    try:
        # Transcribe
        segments, info = MODEL.transcribe(
            temp_path,
            language=None,  # Auto-detect
            vad_filter=True,
            beam_size=5
        )

        # Extract text from segments
        text_parts = []
        for segment in segments:
            text_parts.append(segment.text)

        text = " ".join(text_parts).strip()

        return {
            "text": text,
            "language": info.language,
            "duration": info.duration
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

    finally:
        # Cleanup temp file
        if os.path.exists(temp_path):
            os.remove(temp_path)

@app.get("/health")
async def health():
    return {"status": "ok", "model_loaded": MODEL is not None}

async def stdin_listener():
    """Listen for shutdown command from stdin"""
    loop = asyncio.get_event_loop()
    while True:
        try:
            line = await loop.run_in_executor(None, sys.stdin.readline)
            if line.strip() == "SHUTDOWN":
                print("Received shutdown signal", flush=True)
                os._exit(0)
        except:
            await asyncio.sleep(1)

if __name__ == "__main__":
    # Start stdin listener in background
    loop = asyncio.get_event_loop()
    loop.create_task(stdin_listener())

    # Run FastAPI server
    uvicorn.run(
        app,
        host="127.0.0.1",
        port=8765,
        log_level="error"
    )
