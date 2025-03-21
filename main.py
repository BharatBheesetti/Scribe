"""
Scribe - Audio Transcription Application
Main entry point for the application.
"""
import os
import sys
import tkinter as tk
import time
import platform
import torch
import pynvml

# Add parent directory to path to handle imports when run as script
if __name__ == "__main__":
    sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))
    
from utils.logging import setup_logging
from utils.config import get_config
from ui.app import ScribeApp
from ui.colors import *  # Import our color definitions

def check_dependencies():
    """Check if required dependencies are installed."""
    try:
        # Try to import required packages
        import faster_whisper
        import torch
        import requests
        import sounddevice
       
        # Test sounddevice
        try:
            devices = sounddevice.query_devices()
            default_device = sounddevice.query_devices(kind='input')
            print(f"Found default input device: {default_device['name']}")
        except Exception as e:
            print(f"WARNING: No audio input devices found: {e}")
            return False
           
        return True
    except ImportError as e:
        print(f"ERROR: Missing dependency: {e}")
        print("Please install required packages: pip install faster-whisper torch requests sounddevice")
        return False

def create_splash_screen():
    """Create a simple splash screen while loading."""
    splash = tk.Tk()
    splash.withdraw()
    splash.overrideredirect(True)
    splash.configure(bg=BACKGROUND)
    splash.geometry("300x100")
   
    # Center the splash screen
    screen_width = splash.winfo_screenwidth()
    screen_height = splash.winfo_screenheight()
    x = (screen_width - 300) // 2
    y = (screen_height - 100) // 2
    splash.geometry(f"300x100+{x}+{y}")
   
    # Create the frame with our custom colors
    splash.deiconify()
    frame = tk.Frame(
        splash, 
        bg=BACKGROUND, 
        bd=1, 
        relief=tk.GROOVE, 
        highlightbackground=BORDER, 
        highlightthickness=1
    )
    frame.pack(fill=tk.BOTH, expand=True, padx=2, pady=2)
   
    # Create title label with custom colors
    title_label = tk.Label(
        frame,
        text="Scribe",
        font=("Segoe UI", 18, "bold"),
        bg=BACKGROUND,
        fg=ACCENT
    )
    title_label.pack(pady=(10, 5))
   
    # Create loading label with custom colors
    loading_label = tk.Label(
        frame,
        text="Loading...",
        bg=BACKGROUND,
        fg=TEXT_SECONDARY
    )
    loading_label.pack()
   
    # Update to show the splash screen
    splash.update()
    return splash

def main():
    """Main entry point for the application."""
    # Create splash screen
    splash = create_splash_screen()
   
    try:
        # Setup logging
        setup_logging()
       
        # Check dependencies
        if not check_dependencies():
            splash.destroy()
            return
       
        # Initialize config
        config = get_config()
       
        # Delay to show splash screen
        time.sleep(1)
       
        # Create main window with custom colors
        root = tk.Tk()
        root.withdraw()  # Hide window while loading
        
        # Configure the root window background
        root.configure(bg=BACKGROUND)
        
        # Create app and pass our color scheme
        app = ScribeApp(root)
       
        # Close splash screen
        splash.destroy()
       
        # Show the main window
        root.deiconify()
       
        # Start main loop
        root.mainloop()
    except Exception as e:
        # Make sure to destroy splash screen if anything goes wrong
        if splash:
            splash.destroy()
        raise e

if __name__ == "__main__":
    main()