"""
History management for Scribe application.
Handles recording and transcript history storage and retrieval.
"""
import os
import json
import time
import shutil
from datetime import datetime
from typing import List, Dict, Any, Optional, Tuple
import uuid

from utils.config import get_config
from utils.logging import get_logger
from core.transcriber import TranscriptionSegment

logger = get_logger(__name__)


class HistoryItem:
    """Represents a recording with optional transcript in history."""
    
    def __init__(
        self,
        item_id: str,
        audio_path: str,
        created_at: float,
        transcribed: bool = False,
        transcript_path: Optional[str] = None,
        title: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ):
        """
        Initialize a history item.
        
        Args:
            item_id: Unique ID for this item
            audio_path: Path to the audio file
            created_at: Timestamp when this item was created
            transcribed: Whether this item has been transcribed
            transcript_path: Path to the transcript file if transcribed
            title: Optional title for this item
            metadata: Optional metadata for this item
        """
        self.item_id = item_id
        self.audio_path = audio_path
        self.created_at = created_at
        self.transcribed = transcribed
        self.transcript_path = transcript_path
        self.title = title or os.path.basename(audio_path)
        self.metadata = metadata or {}
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "item_id": self.item_id,
            "audio_path": self.audio_path,
            "created_at": self.created_at,
            "transcribed": self.transcribed,
            "transcript_path": self.transcript_path,
            "title": self.title,
            "metadata": self.metadata
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'HistoryItem':
        """Create from dictionary."""
        return cls(
            item_id=data.get("item_id", str(uuid.uuid4())),
            audio_path=data.get("audio_path", ""),
            created_at=data.get("created_at", time.time()),
            transcribed=data.get("transcribed", False),
            transcript_path=data.get("transcript_path"),
            title=data.get("title"),
            metadata=data.get("metadata", {})
        )
    
    @property
    def created_date_str(self) -> str:
        """Get formatted date string for UI display."""
        dt = datetime.fromtimestamp(self.created_at)
        return dt.strftime("%Y-%m-%d %H:%M:%S")
    
    @property
    def filename(self) -> str:
        """Get the audio filename."""
        return os.path.basename(self.audio_path)
    
    @property
    def audio_exists(self) -> bool:
        """Check if the audio file exists."""
        return os.path.exists(self.audio_path)
    
    @property
    def transcript_exists(self) -> bool:
        """Check if the transcript file exists."""
        return self.transcript_path is not None and os.path.exists(self.transcript_path)


class HistoryManager:
    """Manages recording and transcript history."""
    
    def __init__(self):
        """Initialize the history manager."""
        self.config = get_config()
        self.history_file = os.path.join(self.config.config_dir, "history.json")
        self.items = self._load_history()
    
    def _load_history(self) -> List[HistoryItem]:
        """
        Load history from disk.
        
        Returns:
            List of history items
        """
        if os.path.exists(self.history_file):
            try:
                with open(self.history_file, 'r') as f:
                    data = json.load(f)
                    return [HistoryItem.from_dict(item) for item in data]
            except Exception as e:
                logger.error(f"Failed to load history: {e}")
                return []
        return []
    
    def _save_history(self) -> None:
        """Save history to disk."""
        try:
            with open(self.history_file, 'w') as f:
                json.dump([item.to_dict() for item in self.items], f, indent=4)
        except Exception as e:
            logger.error(f"Failed to save history: {e}")
    
    def add_recording(self, audio_path: str, title: Optional[str] = None) -> HistoryItem:
        """
        Add a recording to history.
        
        Args:
            audio_path: Path to the audio file
            title: Optional title for this recording
            
        Returns:
            The created history item
        """
        # Generate a unique ID
        item_id = str(uuid.uuid4())
        
        # Create history item
        item = HistoryItem(
            item_id=item_id,
            audio_path=audio_path,
            created_at=time.time(),
            title=title or os.path.basename(audio_path)
        )
        
        # Add to history
        self.items.append(item)
        self._save_history()
        
        logger.info(f"Added recording to history: {audio_path}")
        return item
    
    def save_transcript(
        self,
        item_id: str,
        segments: List[TranscriptionSegment],
        metadata: Optional[Dict[str, Any]] = None
    ) -> Optional[str]:
        """
        Save transcript for a recording.
        
        Args:
            item_id: ID of the history item
            segments: List of transcription segments
            metadata: Optional metadata about the transcription
            
        Returns:
            Path to the transcript file if successful, None otherwise
        """
        # Find the history item
        item = self.get_item_by_id(item_id)
        if item is None:
            logger.error(f"History item not found: {item_id}")
            return None
        
        try:
            # Create transcripts directory
            transcripts_dir = os.path.join(self.config.get_recordings_dir(), "transcripts")
            os.makedirs(transcripts_dir, exist_ok=True)
            
            # Create transcript filename
            base_name = os.path.splitext(os.path.basename(item.audio_path))[0]
            transcript_file = f"{base_name}_transcript.json"
            transcript_path = os.path.join(transcripts_dir, transcript_file)
            
            # Save transcript
            transcript_data = {
                "item_id": item_id,
                "audio_path": item.audio_path,
                "created_at": time.time(),
                "segments": [segment.to_dict() for segment in segments],
                "metadata": metadata or {}
            }
            
            with open(transcript_path, 'w') as f:
                json.dump(transcript_data, f, indent=4)
            
            # Update history item
            item.transcribed = True
            item.transcript_path = transcript_path
            if metadata:
                item.metadata.update(metadata)
            
            self._save_history()
            
            logger.info(f"Saved transcript for {item_id}: {transcript_path}")
            return transcript_path
            
        except Exception as e:
            logger.error(f"Failed to save transcript: {e}")
            return None
    
    def load_transcript(self, item_id: str) -> Tuple[List[TranscriptionSegment], Dict[str, Any]]:
        """
        Load transcript for a recording.
        
        Args:
            item_id: ID of the history item
            
        Returns:
            Tuple of (list of segments, metadata)
        """
        # Find the history item
        item = self.get_item_by_id(item_id)
        if item is None or not item.transcript_exists:
            logger.error(f"Transcript not found for item: {item_id}")
            return [], {}
        
        try:
            # Load transcript
            with open(item.transcript_path, 'r') as f:
                data = json.load(f)
            
            # Parse segments
            segments = [TranscriptionSegment.from_dict(segment) for segment in data.get("segments", [])]
            
            # Get metadata
            metadata = data.get("metadata", {})
            
            return segments, metadata
            
        except Exception as e:
            logger.error(f"Failed to load transcript: {e}")
            return [], {}
    
    def get_transcript_text(self, item_id: str) -> str:
        """
        Get the full transcript text for a recording.
        
        Args:
            item_id: ID of the history item
            
        Returns:
            Full transcript text
        """
        segments, _ = self.load_transcript(item_id)
        return " ".join([segment.text for segment in segments])
    
    def get_all_items(self) -> List[HistoryItem]:
        """
        Get all history items sorted by creation date (newest first).
        
        Returns:
            List of history items
        """
        # Check if files exist and remove invalid entries
        valid_items = []
        for item in self.items:
            if item.audio_exists:
                valid_items.append(item)
            else:
                logger.warning(f"Removing history item with missing audio: {item.item_id}")
        
        # Update items list if invalid entries were removed
        if len(valid_items) != len(self.items):
            self.items = valid_items
            self._save_history()
        
        # Sort by creation date (newest first)
        return sorted(self.items, key=lambda x: x.created_at, reverse=True)
    
    def get_item_by_id(self, item_id: str) -> Optional[HistoryItem]:
        """
        Find a history item by ID.
        
        Args:
            item_id: ID of the history item
            
        Returns:
            History item or None if not found
        """
        for item in self.items:
            if item.item_id == item_id:
                return item
        return None
    
    def delete_item(self, item_id: str, delete_files: bool = True) -> bool:
        """
        Delete a history item.
        
        Args:
            item_id: ID of the history item
            delete_files: Whether to delete the associated files
            
        Returns:
            True if successful, False otherwise
        """
        # Find the history item
        item = self.get_item_by_id(item_id)
        if item is None:
            logger.error(f"History item not found: {item_id}")
            return False
        
        try:
            # Delete files if requested
            if delete_files:
                # Delete audio file
                if item.audio_exists:
                    os.remove(item.audio_path)
                
                # Delete transcript file
                if item.transcript_exists:
                    os.remove(item.transcript_path)
            
            # Remove from history
            self.items = [i for i in self.items if i.item_id != item_id]
            self._save_history()
            
            logger.info(f"Deleted history item: {item_id}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to delete history item: {e}")
            return False
    
    def rename_item(self, item_id: str, new_title: str) -> bool:
        """
        Rename a history item.
        
        Args:
            item_id: ID of the history item
            new_title: New title for the item
            
        Returns:
            True if successful, False otherwise
        """
        # Find the history item
        item = self.get_item_by_id(item_id)
        if item is None:
            logger.error(f"History item not found: {item_id}")
            return False
        
        try:
            # Update title
            item.title = new_title
            self._save_history()
            
            logger.info(f"Renamed history item {item_id} to: {new_title}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to rename history item: {e}")
            return False
    
    def export_transcript(self, item_id: str, export_path: str, format_type: str = "txt") -> bool:
        """
        Export transcript to a file.
        
        Args:
            item_id: ID of the history item
            export_path: Path to save the exported file
            format_type: Export format (txt, json, srt)
            
        Returns:
            True if successful, False otherwise
        """
        # Find the history item
        item = self.get_item_by_id(item_id)
        if item is None or not item.transcript_exists:
            logger.error(f"Transcript not found for item: {item_id}")
            return False
        
        try:
            segments, metadata = self.load_transcript(item_id)
            
            if format_type == "txt":
                # Export as plain text
                with open(export_path, 'w', encoding='utf-8') as f:
                    for segment in segments:
                        f.write(f"[{self._format_time(segment.start)} -> {self._format_time(segment.end)}] {segment.text}\n\n")
            
            elif format_type == "json":
                # Export as JSON
                with open(export_path, 'w', encoding='utf-8') as f:
                    json.dump({
                        "metadata": metadata,
                        "segments": [segment.to_dict() for segment in segments]
                    }, f, indent=4)
            
            elif format_type == "srt":
                # Export as SRT subtitle format
                with open(export_path, 'w', encoding='utf-8') as f:
                    for i, segment in enumerate(segments):
                        f.write(f"{i+1}\n")
                        f.write(f"{self._format_srt_time(segment.start)} --> {self._format_srt_time(segment.end)}\n")
                        f.write(f"{segment.text}\n\n")
            
            else:
                logger.error(f"Unsupported export format: {format_type}")
                return False
            
            logger.info(f"Exported transcript for {item_id} to: {export_path}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to export transcript: {e}")
            return False
    
    def _format_time(self, seconds: float) -> str:
        """Format time in seconds to MM:SS.ms format."""
        minutes, seconds = divmod(seconds, 60)
        return f"{int(minutes):02d}:{seconds:.2f}"
    
    def _format_srt_time(self, seconds: float) -> str:
        """Format time in seconds to SRT format (HH:MM:SS,ms)."""
        hours, remainder = divmod(seconds, 3600)
        minutes, seconds = divmod(remainder, 60)
        milliseconds = int((seconds - int(seconds)) * 1000)
        return f"{int(hours):02d}:{int(minutes):02d}:{int(seconds):02d},{milliseconds:03d}"
    
    def clean_old_recordings(self, max_age_days: int = 30) -> int:
        """
        Clean up old recordings.
        
        Args:
            max_age_days: Maximum age in days
            
        Returns:
            Number of items removed
        """
        # Get current time
        now = time.time()
        max_age_seconds = max_age_days * 24 * 60 * 60
        
        # Find old items
        old_items = [item for item in self.items 
                    if (now - item.created_at) > max_age_seconds]
        
        # Delete old items
        count = 0
        for item in old_items:
            if self.delete_item(item.item_id, delete_files=True):
                count += 1
        
        if count > 0:
            logger.info(f"Cleaned up {count} old recordings")
        
        return count
    
    def get_storage_usage(self) -> Dict[str, float]:
        """
        Get storage usage information.
        
        Returns:
            Dict with storage info (total_mb, audio_mb, transcript_mb)
        """
        audio_size = 0
        transcript_size = 0
        
        for item in self.items:
            # Add audio file size
            if item.audio_exists:
                audio_size += os.path.getsize(item.audio_path)
            
            # Add transcript file size
            if item.transcript_exists:
                transcript_size += os.path.getsize(item.transcript_path)
        
        # Convert to MB
        audio_mb = audio_size / (1024 * 1024)
        transcript_mb = transcript_size / (1024 * 1024)
        total_mb = audio_mb + transcript_mb
        
        return {
            "total_mb": total_mb,
            "audio_mb": audio_mb,
            "transcript_mb": transcript_mb
        }


# Global history manager instance
_history_manager_instance = None

def get_history_manager() -> HistoryManager:
    """Get the global history manager instance."""
    global _history_manager_instance
    if _history_manager_instance is None:
        _history_manager_instance = HistoryManager()
    return _history_manager_instance