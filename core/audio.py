"""
Audio recording functionality using sounddevice - simpler and more reliable than FFmpeg.
"""
import sounddevice as sd
import wave
import numpy as np
from datetime import datetime
import uuid
import os
from typing import Optional, Tuple

from utils.config import get_config
from utils.logging import get_logger

logger = get_logger(__name__)

class AudioRecorder:
    def __init__(self):
        self.config = get_config()
        self.recording = False
        self.recording_path = None
        self.stream = None
        self.frames = []
        
    def start_recording(self) -> Tuple[bool, str]:
        if self.recording:
            return False, "Already recording"
            
        try:
            # Create unique filename
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            unique_id = str(uuid.uuid4())[:8]
            filename = f"recording_{timestamp}_{unique_id}.wav"
            recordings_dir = self.config.get_recordings_dir()
            os.makedirs(recordings_dir, exist_ok=True)
            self.recording_path = os.path.join(recordings_dir, filename)
            
            # Start recording
            self.frames = []
            self.recording = True
            
            def callback(indata, frames, time, status):
                if self.recording:
                    self.frames.append(indata.copy())
            
            # Open stream with default device
            self.stream = sd.InputStream(
                channels=1,
                samplerate=44100,
                callback=callback
            )
            self.stream.start()
            
            return True, self.recording_path
            
        except Exception as e:
            return False, f"Failed to start recording: {e}"
    
    def stop_recording(self) -> Tuple[bool, Optional[str]]:
        if not self.recording:
            return False, "Not recording"
            
        try:
            self.recording = False
            self.stream.stop()
            self.stream.close()
            
            # Save to WAV file
            with wave.open(self.recording_path, 'wb') as wf:
                wf.setnchannels(1)
                wf.setsampwidth(2)
                wf.setframerate(44100)
                wf.writeframes((np.concatenate(self.frames) * 32767).astype(np.int16))
            
            return True, self.recording_path
            
        except Exception as e:
            return False, f"Failed to stop recording: {e}"


# Global audio recorder instance
_audio_recorder_instance = None

def get_audio_recorder() -> AudioRecorder:
    """Get the global audio recorder instance."""
    global _audio_recorder_instance
    if _audio_recorder_instance is None:
        _audio_recorder_instance = AudioRecorder()
    return _audio_recorder_instance