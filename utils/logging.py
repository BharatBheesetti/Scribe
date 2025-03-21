"""
Logging configuration for Scribe application.
Sets up logging with appropriate formatting and file handling.
"""
import logging
import os
import sys
import platform
from logging.handlers import RotatingFileHandler
from typing import Optional

from utils.config import get_config


def setup_logging(log_level: Optional[int] = None) -> None:
    """
    Configure application-wide logging.
    
    Args:
        log_level: Optional logging level (defaults to INFO if not specified)
    """
    if log_level is None:
        log_level = logging.INFO
    
    # Get config for log file location
    config = get_config()
    log_dir = os.path.join(config.config_dir, "logs")
    os.makedirs(log_dir, exist_ok=True)
    log_file = os.path.join(log_dir, "scribe.log")
    
    # Configure root logger
    root_logger = logging.getLogger()
    root_logger.setLevel(log_level)
    
    # Clear any existing handlers
    for handler in root_logger.handlers[:]:
        root_logger.removeHandler(handler)
    
    # Create console handler with a higher log level
    console_handler = logging.StreamHandler(stream=sys.stdout)
    console_handler.setLevel(log_level)
    
    # Create file handler for logging to a file
    file_handler = RotatingFileHandler(
        log_file, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8'
    )
    file_handler.setLevel(log_level)
    
    # Create formatter and add it to the handlers
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    console_handler.setFormatter(formatter)
    file_handler.setFormatter(formatter)
    
    # Add handlers to the logger
    root_logger.addHandler(console_handler)
    root_logger.addHandler(file_handler)
    
    # Log system information
    logger = logging.getLogger(__name__)
    logger.info("Logging initialized")
    logger.info(f"OS: {platform.system()} {platform.release()}")
    logger.info(f"Python: {platform.python_version()}")
    logger.info(f"CPU: {platform.processor()}")
    
    try:
        # Attempt to get GPU information on Windows
        if platform.system() == "Windows":
            try:
                import pynvml
                pynvml.nvmlInit()
                device_count = pynvml.nvmlDeviceGetCount()
                for i in range(device_count):
                    handle = pynvml.nvmlDeviceGetHandleByIndex(i)
                    name = pynvml.nvmlDeviceGetName(handle)
                    logger.info(f"GPU {i}: {name.decode('utf-8')}")
                pynvml.nvmlShutdown()
            except (ImportError, Exception) as e:
                logger.info("No NVIDIA GPU detected or pynvml not installed")
    except Exception as e:
        logger.warning(f"Failed to get system info: {e}")
    
    return logger


def get_logger(name: str) -> logging.Logger:
    """
    Get a logger with the specified name.
    
    Args:
        name: Logger name, typically __name__ from the calling module
        
    Returns:
        Logger instance
    """
    return logging.getLogger(name)