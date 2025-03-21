"""
Transcription functionality for Scribe application.
Handles audio transcription using faster-whisper.
"""
import os
import time
import asyncio
import threading
from typing import List, Dict, Any, Optional, Callable, Generator, AsyncGenerator, Tuple
import logging

from utils.config import get_config
from utils.logging import get_logger
from core.models import get_model_manager

# Lazy import faster-whisper to avoid loading it until needed
faster_whisper = None

logger = get_logger(__name__)


class TranscriptionSegment:
    """Represents a segment of transcribed text."""
    
    def __init__(self, segment_data: Dict[str, Any]):
        """
        Initialize from segment data returned by faster-whisper.
        
        Args:
            segment_data: Segment data from faster-whisper
        """
        self.id = segment_data.get("id", 0)
        self.text = segment_data.get("text", "").strip()
        self.start = segment_data.get("start", 0.0)
        self.end = segment_data.get("end", 0.0)
        self.words = segment_data.get("words", [])
    
    @property
    def duration(self) -> float:
        """Get segment duration in seconds."""
        return self.end - self.start
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert segment to dictionary for serialization."""
        return {
            "id": self.id,
            "text": self.text,
            "start": self.start,
            "end": self.end,
            "words": self.words
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'TranscriptionSegment':
        """Create segment from dictionary."""
        return cls(data)


class Transcriber:
    """Handles audio transcription using faster-whisper."""
    
    def __init__(self):
        """Initialize the transcriber."""
        self.config = get_config()
        self.model_manager = get_model_manager()
        self.model = None
        self.model_id = None
        self.transcribing = False
        self.cancel_requested = False
    
    def _load_model(self, model_id: Optional[str] = None) -> bool:
        """
        Load the whisper model.
        
        Args:
            model_id: Optional model ID, uses active model from config if None
            
        Returns:
            True if successful, False otherwise
        """
        if model_id is None:
            model_id = self.model_manager.get_active_model_id()
        
        # Skip if model is already loaded
        if self.model is not None and self.model_id == model_id:
            return True
        
        # Check if model is installed
        if not self.model_manager.is_model_installed(model_id):
            logger.error(f"Model {model_id} is not installed")
            return False
        
        # Import faster-whisper if not already imported
        global faster_whisper
        if faster_whisper is None:
            try:
                import faster_whisper as fw
                faster_whisper = fw
            except ImportError:
                logger.error("Failed to import faster-whisper. Make sure it's installed.")
                return False
        
        try:
            # Get model path
            model_path = self.model_manager.get_model_path(model_id)
            
            # Get computation settings
            device, compute_type = self.model_manager.get_computation_settings()
            
            # Log model loading
            logger.info(f"Loading model {model_id} (device={device}, compute_type={compute_type})")
            
            # Load the model
            self.model = faster_whisper.WhisperModel(
                model_path,
                device=device,
                compute_type=compute_type
            )
            
            self.model_id = model_id
            logger.info(f"Model {model_id} loaded successfully")
            return True
            
        except Exception as e:
            logger.error(f"Failed to load model: {e}")
            self.model = None
            self.model_id = None
            return False
    
    def transcribe(
        self, 
        audio_path: str,
        model_id: Optional[str] = None,
        progress_callback: Optional[Callable[[float], None]] = None
    ) -> Tuple[List[TranscriptionSegment], Dict[str, Any]]:
        """
        Transcribe an audio file.
        
        Args:
            audio_path: Path to the audio file
            model_id: Optional model ID, uses active model from config if None
            progress_callback: Optional callback for progress updates
            
        Returns:
            Tuple of (list of segments, transcription info)
        """
        if not os.path.exists(audio_path):
            logger.error(f"Audio file not found: {audio_path}")
            return [], {"error": "Audio file not found"}
        
        # Load the model
        if not self._load_model(model_id):
            return [], {"error": "Failed to load model"}
        
        try:
            # Get transcription settings
            config = self.config.get_config()["transcription"]
            language = config["language"]
            language = None if language == "auto" else language
            vad_filter = config["vad_filter"]
            word_timestamps = config["word_timestamps"]
            
            # Set transcribing flag
            self.transcribing = True
            self.cancel_requested = False
            
            # Start transcription
            logger.info(f"Starting transcription of {audio_path}")
            segments_generator, info = self.model.transcribe(
                audio_path,
                language=language,
                vad_filter=vad_filter,
                word_timestamps=word_timestamps
            )
            
            # Process segments
            segments = []
            total_duration = self._get_audio_duration(audio_path)
            
            for segment in segments_generator:
                if self.cancel_requested:
                    logger.info("Transcription cancelled")
                    break
                
                # Create segment object
                segment_obj = TranscriptionSegment({
                    "id": len(segments),
                    "text": segment.text,
                    "start": segment.start,
                    "end": segment.end,
                    "words": [{"word": w.word, "start": w.start, "end": w.end} 
                              for w in segment.words] if segment.words else []
                })
                
                segments.append(segment_obj)
                
                # Update progress if callback provided
                if progress_callback and total_duration > 0:
                    progress = min(100, (segment.end / total_duration) * 100)
                    progress_callback(progress)
            
            # Ensure 100% progress is reported
            if progress_callback:
                progress_callback(100)
            
            logger.info(f"Transcription completed: {len(segments)} segments")
            
            # Return segments and info
            return segments, {
                "language": info.language,
                "language_probability": info.language_probability,
                "duration": total_duration,
                "model_id": self.model_id
            }
            
        except Exception as e:
            logger.error(f"Transcription failed: {e}")
            return [], {"error": str(e)}
        finally:
            self.transcribing = False
            self.cancel_requested = False
    
    async def transcribe_async(
        self, 
        audio_path: str,
        model_id: Optional[str] = None,
        progress_callback: Optional[Callable[[float], None]] = None
    ) -> Tuple[List[TranscriptionSegment], Dict[str, Any]]:
        """
        Transcribe an audio file asynchronously.
        
        Args:
            audio_path: Path to the audio file
            model_id: Optional model ID, uses active model from config if None
            progress_callback: Optional callback for progress updates
            
        Returns:
            Tuple of (list of segments, transcription info)
        """
        def _run_transcription():
            return self.transcribe(audio_path, model_id, progress_callback)
        
        # Run transcription in a thread to avoid blocking
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _run_transcription)
    
    async def transcribe_with_progress(
        self, 
        audio_path: str,
        model_id: Optional[str] = None
    ) -> AsyncGenerator[TranscriptionSegment, None]:
        """
        Transcribe an audio file and yield segments as they're transcribed.
        
        Args:
            audio_path: Path to the audio file
            model_id: Optional model ID, uses active model from config if None
            
        Yields:
            Segments as they're transcribed
        """
        if not os.path.exists(audio_path):
            logger.error(f"Audio file not found: {audio_path}")
            return
        
        # Load the model
        if not self._load_model(model_id):
            return
        
        try:
            # Get transcription settings
            config = self.config.get_config()["transcription"]
            language = config["language"]
            language = None if language == "auto" else language
            vad_filter = config["vad_filter"]
            word_timestamps = config["word_timestamps"]
            
            # Set transcribing flag
            self.transcribing = True
            self.cancel_requested = False
            
            # Start transcription
            logger.info(f"Starting transcription of {audio_path}")
            segments_generator, info = self.model.transcribe(
                audio_path,
                language=language,
                vad_filter=vad_filter,
                word_timestamps=word_timestamps
            )
            
            # Process segments
            for segment in segments_generator:
                if self.cancel_requested:
                    logger.info("Transcription cancelled")
                    break
                
                # Create segment object
                segment_obj = TranscriptionSegment({
                    "id": segment.id,
                    "text": segment.text,
                    "start": segment.start,
                    "end": segment.end,
                    "words": [{"word": w.word, "start": w.start, "end": w.end} 
                              for w in segment.words] if segment.words else []
                })
                
                yield segment_obj
                
                # Add a small delay to prevent overloading the event loop
                await asyncio.sleep(0.01)
            
        except Exception as e:
            logger.error(f"Transcription failed: {e}")
        finally:
            self.transcribing = False
            self.cancel_requested = False
    
    def cancel_transcription(self) -> bool:
        """
        Cancel an ongoing transcription.
        
        Returns:
            True if cancellation was requested, False if not transcribing
        """
        if self.transcribing:
            self.cancel_requested = True
            logger.info("Cancellation requested")
            return True
        return False
    
    def is_transcribing(self) -> bool:
        """Check if transcription is in progress."""
        return self.transcribing
    
    def _get_audio_duration(self, audio_path: str) -> float:
        """
        Get the duration of an audio file in seconds.
        
        Args:
            audio_path: Path to the audio file
            
        Returns:
            Duration in seconds or 0 if failed
        """
        try:
            import subprocess
            
            cmd = [
                "ffprobe", 
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                audio_path
            ]
            
            process = subprocess.Popen(
                cmd, 
                stdout=subprocess.PIPE, 
                stderr=subprocess.PIPE,
                text=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            
            output, _ = process.communicate()
            duration = float(output.strip())
            return duration
            
        except Exception as e:
            logger.error(f"Failed to get audio duration: {e}")
            return 0.0


# Global transcriber instance
_transcriber_instance = None

def get_transcriber() -> Transcriber:
    """Get the global transcriber instance."""
    global _transcriber_instance
    if _transcriber_instance is None:
        _transcriber_instance = Transcriber()
    return _transcriber_instance