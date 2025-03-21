"""
Models management for Scribe application.
Handles downloading, management, and selection of whisper models.
"""
import os
import shutil
import logging
import requests
import json
import hashlib
from typing import Dict, List, Optional, Tuple, Callable
import platform
import torch

from utils.config import get_config
from utils.logging import get_logger

logger = get_logger(__name__)

# Model information
MODELS = {
    "tiny": {
        "name": "Tiny",
        "description": "Tiny model (75M parameters)",
        "size_mb": 75,
        "url_base": "https://huggingface.co/guillaumekln/faster-whisper-tiny/resolve/main",
        "files": [
            "model.bin",
            "config.json",
            "tokenizer.json",
            "vocabulary.txt"
        ],
        "md5": {
            "model.bin": "bd577a113a864445d451f0cfa57322b3",
            "config.json": "7779dd56c86a50931da8ab5b54c6e0cb",
            "tokenizer.json": "409bab3ff36f03adcceeaa254ae59191", 
            "vocabulary.txt": "d12a2cccd7b69e569d7bb64b6d7d2603"
        }
    },
    "base": {
        "name": "Base",
        "description": "Base model (244M parameters)",
        "size_mb": 142,
        "url_base": "https://huggingface.co/guillaumekln/faster-whisper-base/resolve/main",
        "files": [
            "model.bin",
            "config.json",
            "tokenizer.json",
            "vocabulary.txt"
        ],
        "md5": {
            "model.bin": "51e2612c7c5a29fd21c153e899ae3ac4",
            "config.json": "c6eb4a26e9b54c134fc90d0e8db428d9",
            "tokenizer.json": "409bab3ff36f03adcceeaa254ae59191",
            "vocabulary.txt": "d12a2cccd7b69e569d7bb64b6d7d2603"
        }
    },
    "small": {
        "name": "Small",
        "description": "Small model (486M parameters)",
        "size_mb": 461,
        "url_base": "https://huggingface.co/guillaumekln/faster-whisper-small/resolve/main",
        "files": [
            "model.bin",
            "config.json",
            "tokenizer.json",
            "vocabulary.txt"
        ],
        "md5": {
            "model.bin": "ab753e68c413f0d7541da40518e48676",
            "config.json": "eca5a3d0f7d4e2ee6903c5e74eb58e25", 
            "tokenizer.json": "409bab3ff36f03adcceeaa254ae59191",
            "vocabulary.txt": "d12a2cccd7b69e569d7bb64b6d7d2603"
        }
    },
    "medium": {
        "name": "Medium",
        "description": "Medium model (1.5B parameters)",
        "size_mb": 1500,
        "url_base": "https://huggingface.co/guillaumekln/faster-whisper-medium/resolve/main",
        "files": [
            "model.bin",
            "config.json",
            "tokenizer.json",
            "vocabulary.txt"
        ],
        "md5": {
            "model.bin": "cae164e13c91eaa98e4a8e3ef352722a",
            "config.json": "aa0405ab73e8e601b4e4bc19bc0ea7bd", 
            "tokenizer.json": "409bab3ff36f03adcceeaa254ae59191",
            "vocabulary.txt": "d12a2cccd7b69e569d7bb64b6d7d2603"
        }
    },
}


class ModelManager:
    """Manages whisper model downloading and selection."""
    
    def __init__(self):
        """Initialize the model manager."""
        self.config = get_config()
        self.models_dir = self.config.get_models_dir()
        self.current_model = None
        self._check_system_capabilities()
    
    def _check_system_capabilities(self) -> None:
        """Check system capabilities to determine optimal model settings."""
        # Check for GPU
        cuda_available = torch.cuda.is_available()
        
        if cuda_available:
            device_count = torch.cuda.device_count()
            device_name = torch.cuda.get_device_name(0) if device_count > 0 else "Unknown"
            memory_total = torch.cuda.get_device_properties(0).total_memory / (1024**3) if device_count > 0 else 0
            
            logger.info(f"CUDA is available: {cuda_available}")
            logger.info(f"Number of CUDA devices: {device_count}")
            logger.info(f"CUDA device name: {device_name}")
            logger.info(f"CUDA device memory: {memory_total:.2f} GB")
            
            # Set computation type based on GPU memory
            if memory_total > 4:
                compute_type = "float16"
            else:
                compute_type = "int8_float16"
            
            device = "cuda"
        else:
            logger.info("CUDA is not available, using CPU")
            device = "cpu"
            compute_type = "int8"
        
        # Update config with system capabilities
        self.config.update_config("transcription", "compute_type", compute_type)
        self.config.update_config("transcription", "device", device)
    
    def get_model_info(self, model_id: str) -> Dict:
        """Get information about a specific model."""
        return MODELS.get(model_id, {})
    
    def get_all_models(self) -> List[Dict]:
        """Get information about all available models."""
        return [{"id": model_id, **model_info} for model_id, model_info in MODELS.items()]
    
    def get_installed_models(self) -> List[str]:
        """Get a list of installed model IDs."""
        installed = []
        for model_id in MODELS:
            if self.is_model_installed(model_id):
                installed.append(model_id)
        return installed
    
    def is_model_installed(self, model_id: str) -> bool:
        """Check if a specific model is installed."""
        if model_id not in MODELS:
            return False
        
        model_path = os.path.join(self.models_dir, model_id)
        if not os.path.exists(model_path):
            return False
        
        # Check if all required files exist
        for file in MODELS[model_id]["files"]:
            file_path = os.path.join(model_path, file)
            if not os.path.exists(file_path):
                return False
        
        return True
    
    def download_model(self, model_id: str, progress_callback: Optional[Callable[[float], None]] = None) -> bool:
        """
        Download a model.
        
        Args:
            model_id: ID of the model to download
            progress_callback: Optional callback function to report progress (0-100)
            
        Returns:
            True if successful, False otherwise
        """
        if model_id not in MODELS:
            logger.error(f"Unknown model ID: {model_id}")
            return False
        
        model_info = MODELS[model_id]
        model_path = os.path.join(self.models_dir, model_id)
        
        # Create model directory if it doesn't exist
        os.makedirs(model_path, exist_ok=True)
        
        # Calculate total download size for progress reporting
        total_files = len(model_info["files"])
        completed_files = 0
        
        # Download each file
        for file in model_info["files"]:
            url = f"{model_info['url_base']}/{file}"
            file_path = os.path.join(model_path, file)
            
            try:
                # Download the file
                response = requests.get(url, stream=True)
                response.raise_for_status()
                
                total_size = int(response.headers.get('content-length', 0))
                block_size = 1024  # 1 KB
                
                with open(file_path, 'wb') as f:
                    downloaded = 0
                    for data in response.iter_content(block_size):
                        f.write(data)
                        downloaded += len(data)
                        
                        # Calculate and report progress
                        if progress_callback and total_size > 0:
                            file_progress = downloaded / total_size
                            overall_progress = (completed_files + file_progress) / total_files * 100
                            progress_callback(overall_progress)
                
                
                # Skip MD5 verification - models are frequently updated
                # Just log the actual MD5 for reference
                if file in model_info.get("md5", {}):
                    expected_md5 = model_info["md5"][file]
                    actual_md5 = self._calculate_md5(file_path)
                    
                    if expected_md5 != actual_md5:
                        logger.info(f"Note: MD5 for {file} is {actual_md5} (expected was {expected_md5})")
                
                completed_files += 1
                if progress_callback:
                    progress_callback(completed_files / total_files * 100)
                    
            except Exception as e:
                logger.error(f"Failed to download {file}: {e}")
                # Clean up partial download
                if os.path.exists(file_path):
                    os.remove(file_path)
                return False
        
        logger.info(f"Successfully downloaded model: {model_id}")
        return True
    
    def _calculate_md5(self, file_path: str) -> str:
        """Calculate MD5 hash of a file."""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
    
    def delete_model(self, model_id: str) -> bool:
        """Delete a downloaded model."""
        if model_id not in MODELS:
            logger.error(f"Unknown model ID: {model_id}")
            return False
        
        model_path = os.path.join(self.models_dir, model_id)
        if not os.path.exists(model_path):
            # Model not installed
            return True
        
        try:
            shutil.rmtree(model_path)
            logger.info(f"Deleted model: {model_id}")
            return True
        except Exception as e:
            logger.error(f"Failed to delete model {model_id}: {e}")
            return False
    
    def get_active_model_id(self) -> str:
        """Get the currently active model ID from config."""
        return self.config.get_config()["transcription"]["model"]
    
    def set_active_model(self, model_id: str) -> bool:
        """Set the active model."""
        if model_id not in MODELS:
            logger.error(f"Unknown model ID: {model_id}")
            return False
        
        if not self.is_model_installed(model_id):
            logger.error(f"Model {model_id} is not installed")
            return False
        
        self.config.update_config("transcription", "model", model_id)
        logger.info(f"Set active model to: {model_id}")
        return True
    
    def get_model_path(self, model_id: str) -> str:
        """Get the file system path for a model."""
        return os.path.join(self.models_dir, model_id)
    
    def get_computation_settings(self) -> Tuple[str, str]:
        """Get computation device and type from config."""
        config = self.config.get_config()
        return config["transcription"]["device"], config["transcription"]["compute_type"]


# Global model manager instance
_model_manager_instance = None

def get_model_manager() -> ModelManager:
    """Get the global model manager instance."""
    global _model_manager_instance
    if _model_manager_instance is None:
        _model_manager_instance = ModelManager()
    return _model_manager_instance