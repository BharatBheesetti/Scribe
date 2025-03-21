"""
Configuration management for Scribe application.
Handles loading, saving, and providing default configuration values.
"""
import json
import os
import logging
from typing import Dict, Any, Optional

# Setup constants
APP_NAME = "Scribe"
DEFAULT_CONFIG = {
    "audio": {
        "input_device": "default",
        "sample_rate": 44100,
        "bit_depth": 16,
        "mp3_bitrate": 192,
    },
    "transcription": {
        "model": "tiny",
        "language": "auto",
        "compute_type": "auto",  # Will be set based on system capabilities
        "device": "cpu",         # Will be set based on system capabilities
        "vad_filter": True,
        "word_timestamps": True,
    },
    "ui": {
        "font_size": 10,
        "theme": "default",
        "max_history_items": 100,
    },
    "storage": {
        "recordings_dir": "",  # Will be set during initialization
        "models_dir": "",      # Will be set during initialization
        "max_recording_size_mb": 1000,  # 1GB total storage limit
    }
}


class ConfigManager:
    """Manages application configuration."""
    
    def __init__(self):
        self.logger = logging.getLogger(__name__)
        self.config_dir = self._get_config_dir()
        self.config_path = os.path.join(self.config_dir, "config.json")
        self.config = self._load_config()
        
    def _get_config_dir(self) -> str:
        """Get the configuration directory based on the OS."""
        config_dir = os.path.join(os.environ.get("APPDATA", os.path.expanduser("~")), APP_NAME)
        os.makedirs(config_dir, exist_ok=True)
        return config_dir
        
    def _load_config(self) -> Dict[str, Any]:
        """Load configuration from disk or create default if not exists."""
        if os.path.exists(self.config_path):
            try:
                with open(self.config_path, 'r') as f:
                    config = json.load(f)
                    # Merge with default config to ensure all keys exist
                    self._merge_configs(DEFAULT_CONFIG, config)
                    return config
            except Exception as e:
                self.logger.error(f"Failed to load config: {e}")
                return DEFAULT_CONFIG.copy()
        else:
            # Create default config
            config = DEFAULT_CONFIG.copy()
            
            # Set default paths
            recordings_dir = os.path.join(self.config_dir, "recordings")
            models_dir = os.path.join(self.config_dir, "models")
            
            os.makedirs(recordings_dir, exist_ok=True)
            os.makedirs(models_dir, exist_ok=True)
            
            config["storage"]["recordings_dir"] = recordings_dir
            config["storage"]["models_dir"] = models_dir
            
            self.save_config(config)
            return config
    
    def _merge_configs(self, default: Dict[str, Any], current: Dict[str, Any]) -> None:
        """Recursively merge default config with current config."""
        for key, value in default.items():
            if key not in current:
                current[key] = value
            elif isinstance(value, dict) and isinstance(current[key], dict):
                self._merge_configs(value, current[key])
    
    def save_config(self, config: Optional[Dict[str, Any]] = None) -> None:
        """Save configuration to disk."""
        if config:
            self.config = config
        
        try:
            with open(self.config_path, 'w') as f:
                json.dump(self.config, f, indent=4)
        except Exception as e:
            self.logger.error(f"Failed to save config: {e}")
    
    def get_config(self) -> Dict[str, Any]:
        """Get the current configuration."""
        return self.config
    
    def update_config(self, section: str, key: str, value: Any) -> None:
        """Update a specific configuration value."""
        if section in self.config and key in self.config[section]:
            self.config[section][key] = value
            self.save_config()
        else:
            self.logger.error(f"Invalid config section or key: {section}.{key}")
    
    def get_recordings_dir(self) -> str:
        """Get the recordings directory path."""
        return self.config["storage"]["recordings_dir"]
    
    def get_models_dir(self) -> str:
        """Get the models directory path."""
        return self.config["storage"]["models_dir"]


# Global config instance
_config_instance = None

def get_config() -> ConfigManager:
    """Get the global configuration manager instance."""
    global _config_instance
    if _config_instance is None:
        _config_instance = ConfigManager()
    return _config_instance