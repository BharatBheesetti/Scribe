"""
Main UI application for Scribe.
Provides GUI interface for recording, transcribing, and managing audio files.
"""
import os
import tkinter as tk
from tkinter import ttk, filedialog, messagebox
import threading
import asyncio
import time
from typing import List, Dict, Any, Optional, Callable
from enum import Enum
from datetime import datetime, timedelta

from utils.config import get_config
from utils.logging import get_logger
from core.audio import get_audio_recorder
from core.transcriber import get_transcriber, TranscriptionSegment
from core.models import get_model_manager
from core.history import get_history_manager, HistoryItem
from ui.colors import *  # Import colors from ui directory

logger = get_logger(__name__)


class AppState(Enum):
    """Application states."""
    IDLE = "idle"
    RECORDING = "recording"
    TRANSCRIBING = "transcribing"
    UPLOADING = "uploading"
    DOWNLOADING_MODEL = "downloading_model"


class ScribeApp:
    """Main application class for Scribe."""
    
    def __init__(self, root):
        """
        Initialize the application.
        
        Args:
            root: Tkinter root window
        """
        self.root = root
        self.root.title("Scribe")
        self.root.minsize(800, 600)
        self.root.geometry("1024x768")
        
        # Set background color directly
        self.root.configure(bg=BACKGROUND)
        
        # Store colors for easy access
        self.COLORS = {
            "background": BACKGROUND,
            "content_bg": CONTENT_BG,
            "card_bg": CARD_BG,
            "text_primary": TEXT_PRIMARY,
            "text_secondary": TEXT_SECONDARY,
            "accent": ACCENT,
            "border": BORDER,
            "btn_primary": BTN_PRIMARY,
            "btn_secondary": BTN_SECONDARY,
            "btn_danger": BTN_DANGER,
            "success": SUCCESS,
            "warning": WARNING,
            "error": ERROR,
            "info": INFO,
            "recording": RECORDING
        }
        
        self.config = get_config()
        self.audio_recorder = get_audio_recorder()
        self.transcriber = get_transcriber()
        self.model_manager = get_model_manager()
        self.history_manager = get_history_manager()
        
        self.current_item = None
        self.current_segments = []
        self.state = AppState.IDLE
        
        self._setup_ui()
        self._update_history_list()
        self._update_ui_state()
        self._check_models()
    
    def _setup_ui(self):
        """Set up the user interface with custom colors."""
        # Create main frame with custom colors
        self.main_frame = tk.Frame(
            self.root, 
            bg=BACKGROUND,
            bd=0
        )
        self.main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Configure rows and columns for main frame
        self.main_frame.columnconfigure(0, weight=3)  # History panel
        self.main_frame.columnconfigure(1, weight=7)  # Content panel
        self.main_frame.rowconfigure(0, weight=1)
        
        # Create history panel with custom colors
        self.history_frame = tk.LabelFrame(
            self.main_frame, 
            text="History",
            bg=CONTENT_BG,
            fg=TEXT_PRIMARY,
            bd=1,
            relief=tk.GROOVE,
            highlightbackground=BORDER
        )
        self.history_frame.grid(row=0, column=0, sticky="nsew", padx=5, pady=5)

        # Configure history frame
        self.history_frame.columnconfigure(0, weight=1)
        self.history_frame.rowconfigure(0, weight=0)  # Search bar
        self.history_frame.rowconfigure(1, weight=1)  # History list
        
        # Create search bar with custom colors
        self.search_var = tk.StringVar()
        self.search_entry = tk.Entry(
            self.history_frame, 
            textvariable=self.search_var,
            bg=CARD_BG,
            fg=TEXT_PRIMARY,
            insertbackground=TEXT_PRIMARY,  # cursor color
            relief=tk.FLAT,
            highlightbackground=BORDER,
            highlightthickness=1
        )
        self.search_entry.grid(row=0, column=0, sticky="ew", padx=5, pady=5)
        self.search_var.trace_add("write", lambda *args: self._filter_history())
        
        # Create history list with custom colors
        self.history_list = ttk.Treeview(
            self.history_frame,
            columns=("title", "date", "status"),
            show="headings"
        )
        
        # Configure the Treeview style for dark mode
        style = ttk.Style()
        style.theme_use('default')  # Use default theme as base
        
        # Configure the Treeview with our custom colors
        style.configure(
            "Treeview",
            background=CARD_BG,
            foreground=TEXT_PRIMARY,
            fieldbackground=CARD_BG,
            borderwidth=0,
            font=('Segue UI', 9)
        )
        
        # Configure the Treeview headers
        style.configure(
            "Treeview.Heading",
            background=CONTENT_BG,
            foreground=TEXT_PRIMARY,
            relief=tk.FLAT,
            font=('Segue UI', 9, 'bold')
        )
        
        # Configure selection colors
        style.map(
            "Treeview",
            background=[('selected', ACCENT)],
            foreground=[('selected', TEXT_PRIMARY)]
        )
        
        # Configure column headings
        self.history_list.heading("title", text="Title")
        self.history_list.heading("date", text="Date")
        self.history_list.heading("status", text="Status")
        
        self.history_list.column("title", width=150)
        self.history_list.column("date", width=100)
        self.history_list.column("status", width=80)
        
        self.history_list.grid(row=1, column=0, sticky="nsew")
        self.history_list.bind("<<TreeviewSelect>>", self._on_history_select)
        
        # Add scrollbar to history list
        history_scrollbar = tk.Scrollbar(
            self.history_frame,
            orient=tk.VERTICAL,
            command=self.history_list.yview,
            bg=CONTENT_BG,
            troughcolor=CARD_BG,
            activebackground=ACCENT
        )
        self.history_list.configure(yscrollcommand=history_scrollbar.set)
        history_scrollbar.grid(row=1, column=1, sticky="ns")
        
        # Create history controls with custom colors
        self.history_controls = tk.Frame(
            self.history_frame,
            bg=CONTENT_BG
        )
        self.history_controls.grid(row=2, column=0, sticky="ew", padx=5, pady=5)
        
        # Create custom looking buttons
        def create_button(parent, text, command, color):
            """Create a button with custom styling"""
            btn = tk.Button(
                parent, 
                text=text, 
                command=command,
                bg=color,
                fg=TEXT_PRIMARY,
                activebackground=self._adjust_color(color, 1.1),
                activeforeground=TEXT_PRIMARY,
                relief=tk.FLAT,
                bd=1,
                padx=10,
                pady=2,
                font=("Segue UI", 9)
            )
            # Add hover effect
            btn.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(color, 1.1)))
            btn.bind("<Leave>", lambda e: e.widget.config(bg=color))
            return btn
        
        # Create buttons with custom colors
        delete_btn = create_button(
            self.history_controls, "Delete", self._delete_selected, BTN_DANGER
        )
        delete_btn.pack(side=tk.LEFT, padx=2)
        
        rename_btn = create_button(
            self.history_controls, "Rename", self._rename_selected, BTN_SECONDARY
        )
        rename_btn.pack(side=tk.LEFT, padx=2)
        
        export_btn = create_button(
            self.history_controls, "Export", self._export_selected, BTN_PRIMARY
        )
        export_btn.pack(side=tk.LEFT, padx=2)
        
        # Create content panel with custom colors
        self.content_frame = tk.Frame(
            self.main_frame,
            bg=CONTENT_BG,
            bd=1,
            relief=tk.GROOVE,
            highlightbackground=BORDER
        )
        self.content_frame.grid(row=0, column=1, sticky="nsew", padx=5, pady=5)
        
        # Configure content frame
        self.content_frame.columnconfigure(0, weight=1)
        self.content_frame.rowconfigure(0, weight=0)  # Controls
        self.content_frame.rowconfigure(1, weight=0)  # Progress
        self.content_frame.rowconfigure(2, weight=1)  # Transcript
        
        # Create controls frame with custom colors
        self.controls_frame = tk.Frame(
            self.content_frame,
            bg=CONTENT_BG
        )
        self.controls_frame.grid(row=0, column=0, sticky="ew", padx=5, pady=5)
        
        # Create recording controls
        self.record_button = create_button(
            self.controls_frame, "Record", self._toggle_recording, BTN_DANGER
        )
        self.record_button.config(width=10)
        self.record_button.pack(side=tk.LEFT, padx=5)
        
        self.upload_button = create_button(
            self.controls_frame, "Upload Audio", self._upload_audio, BTN_SECONDARY
        )
        self.upload_button.config(width=12)
        self.upload_button.pack(side=tk.LEFT, padx=5)
        
        self.transcribe_button = create_button(
            self.controls_frame, "Transcribe", self._transcribe_selected, BTN_PRIMARY
        )
        self.transcribe_button.config(width=10)
        self.transcribe_button.pack(side=tk.LEFT, padx=5)
        
        self.copy_button = create_button(
            self.controls_frame, "Copy Text", self._copy_transcript, BTN_SECONDARY
        )
        self.copy_button.config(width=10)
        self.copy_button.pack(side=tk.LEFT, padx=5)
        
        # Model selector
        self.model_var = tk.StringVar()
        self.model_label = tk.Label(
            self.controls_frame, 
            text="Model:",
            font=("Segue UI", 10),
            bg=CONTENT_BG,
            fg=TEXT_PRIMARY
        )
        self.model_label.pack(side=tk.LEFT, padx=(20, 5))
        
        # Create a custom styled combobox
        style.configure(
            "TCombobox",
            fieldbackground=CARD_BG,
            background=CARD_BG,
            foreground=TEXT_PRIMARY,
            arrowcolor=TEXT_PRIMARY,
            selectbackground=ACCENT,
            selectforeground=TEXT_PRIMARY
        )
        
        self.model_selector = ttk.Combobox(
            self.controls_frame,
            textvariable=self.model_var,
            state="readonly",
            width=15
        )
        self.model_selector.pack(side=tk.LEFT)
        self.model_selector.bind("<<ComboboxSelected>>", self._on_model_select)
        self._update_model_selector()
        
        # Create status panel
        self.status_frame = tk.Frame(
            self.content_frame,
            bg=CONTENT_BG
        )
        self.status_frame.grid(row=1, column=0, sticky="ew", padx=5, pady=5)
        
        # Status label
        self.status_var = tk.StringVar(value="Ready")
        self.status_label = tk.Label(
            self.status_frame,
            textvariable=self.status_var,
            bg=CONTENT_BG,
            fg=TEXT_SECONDARY,
            font=("Segue UI", 10)
        )
        self.status_label.pack(side=tk.LEFT)
        
        # Progress bar
        self.progress_var = tk.DoubleVar(value=0)
        self.progress_frame = tk.Frame(
            self.status_frame,
            bg=CARD_BG,
            height=20,
            width=300,
            bd=0
        )
        
        # Create a custom progress bar
        self.progress_bar_bg = tk.Frame(
            self.progress_frame,
            bg=CARD_BG,
            height=20,
            width=300
        )
        self.progress_bar_bg.place(x=0, y=0, relwidth=1, relheight=1)
        
        self.progress_bar = tk.Frame(
            self.progress_bar_bg,
            bg=ACCENT,
            height=20,
            width=0
        )
        self.progress_bar.place(x=0, y=0, relheight=1, relwidth=0)
        
        self.progress_frame.pack(side=tk.RIGHT, padx=5)
        
        # Initialize recording timer with custom colors
        self.timer_var = tk.StringVar(value="00:00:00")
        self.timer_label = tk.Label(
            self.status_frame,
            textvariable=self.timer_var,
            font=("Segue UI", 12, "bold"),
            bg=CONTENT_BG,
            fg=RECORDING
        )
        
        # Create transcript text area with custom colors
        self.transcript_frame = tk.LabelFrame(
            self.content_frame,
            text="Transcript",
            bg=CONTENT_BG,
            fg=TEXT_PRIMARY,
            bd=1,
            relief=tk.GROOVE,
            highlightbackground=BORDER
        )
        self.transcript_frame.grid(row=2, column=0, sticky="nsew", padx=5, pady=5)
        
        self.transcript_text = tk.Text(
            self.transcript_frame,
            wrap=tk.WORD,
            font=("Segue UI", 11),
            bg=CARD_BG,
            fg=TEXT_PRIMARY,
            insertbackground=TEXT_PRIMARY,
            selectbackground=ACCENT,
            selectforeground=TEXT_PRIMARY,
            relief=tk.FLAT,
            bd=0,
            padx=10,
            pady=10,
            highlightthickness=0
        )
        self.transcript_text.pack(fill=tk.BOTH, expand=True)
        
        # Add a scrollbar to the transcript text area
        transcript_scrollbar = tk.Scrollbar(
            self.transcript_text,
            bg=CONTENT_BG,
            troughcolor=CARD_BG,
            activebackground=ACCENT
        )
        transcript_scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        self.transcript_text.config(yscrollcommand=transcript_scrollbar.set)
        transcript_scrollbar.config(command=self.transcript_text.yview)
        
        # Create status bar with custom colors
        self.statusbar = tk.Label(
            self.root,
            text="Ready",
            bd=1,
            relief=tk.SUNKEN,
            anchor=tk.W,
            bg=CONTENT_BG,
            fg=TEXT_SECONDARY,
            padx=5,
            pady=2
        )
        self.statusbar.pack(side=tk.BOTTOM, fill=tk.X)
        
        # Set up menu
        self._setup_menu()
    
    def _setup_menu(self):
        """Set up application menu with custom colors."""
        # Configure menu colors
        self.root.option_add('*Menu.background', CONTENT_BG)
        self.root.option_add('*Menu.foreground', TEXT_PRIMARY)
        self.root.option_add('*Menu.activeBackground', ACCENT)
        self.root.option_add('*Menu.activeForeground', TEXT_PRIMARY)
        
        menubar = tk.Menu(self.root)
        
        # File menu
        file_menu = tk.Menu(menubar, tearoff=0)
        file_menu.add_command(label="New Recording", command=self._start_recording)
        file_menu.add_command(label="Upload Audio", command=self._upload_audio)
        file_menu.add_separator()
        file_menu.add_command(label="Exit", command=self.root.quit)
        menubar.add_cascade(label="File", menu=file_menu)
        
        # Edit menu
        edit_menu = tk.Menu(menubar, tearoff=0)
        edit_menu.add_command(label="Copy Transcript", command=self._copy_transcript)
        edit_menu.add_command(label="Export Transcript", command=self._export_selected)
        menubar.add_cascade(label="Edit", menu=edit_menu)
        
        # Tools menu
        tools_menu = tk.Menu(menubar, tearoff=0)
        tools_menu.add_command(label="Manage Models", command=self._show_model_manager)
        tools_menu.add_command(label="Settings", command=self._show_settings)
        menubar.add_cascade(label="Tools", menu=tools_menu)
        
        # Help menu
        help_menu = tk.Menu(menubar, tearoff=0)
        help_menu.add_command(label="About", command=self._show_about)
        menubar.add_cascade(label="Help", menu=help_menu)
        
        self.root.config(menu=menubar)
    
    def _update_ui_state(self):
        """Update UI elements based on current state."""
        # Get current state
        state = self.state
        
        # Update status text
        if state == AppState.IDLE:
            self.status_var.set("Ready")
        elif state == AppState.RECORDING:
            self.status_var.set("Recording...")
        elif state == AppState.TRANSCRIBING:
            self.status_var.set("Transcribing...")
        elif state == AppState.UPLOADING:
            self.status_var.set("Uploading...")
        elif state == AppState.DOWNLOADING_MODEL:
            self.status_var.set("Downloading model...")
        
        # Update button states
        recording_active = (state == AppState.RECORDING)
        processing_active = (state in [AppState.TRANSCRIBING, AppState.DOWNLOADING_MODEL])
        
        # Recording button text and state
        if recording_active:
            self.record_button.configure(text="Stop Recording")
        else:
            self.record_button.configure(text="Record")
        
        # Enable/disable buttons based on state
        self.upload_button.configure(state="disabled" if recording_active or processing_active else "normal")
        
        # Transcribe button state depends on having a selected item
        can_transcribe = self.current_item is not None and not processing_active and not recording_active
        self.transcribe_button.configure(state="normal" if can_transcribe else "disabled")
        
        # Copy button state depends on having a transcript
        has_transcript = self.current_item is not None and self.current_item.transcribed
        self.copy_button.configure(state="normal" if has_transcript else "disabled")
        
        # Update history list interactivity (treeview doesn't have state property)
        if processing_active:
            # Disable interaction by unbinding events
            self.history_list.unbind("<<TreeviewSelect>>")
        else:
            # Re-enable interaction
            self.history_list.bind("<<TreeviewSelect>>", self._on_history_select)
        
        # Show/hide progress bar
        if state in [AppState.TRANSCRIBING, AppState.DOWNLOADING_MODEL]:
            self.progress_frame.pack(side=tk.RIGHT, padx=5)
        else:
            self.progress_var.set(0)
            self.progress_bar.place(relwidth=0)
            self.progress_frame.pack_forget()
        
        # Show/hide timer
        if state == AppState.RECORDING:
            self.timer_label.pack(side=tk.LEFT, padx=20)
        else:
            self.timer_label.pack_forget()
            self.timer_var.set("00:00:00")
    
    def _update_history_list(self):
        """Update the history list with items from history manager."""
        # Clear current items
        for item in self.history_list.get_children():
            self.history_list.delete(item)
        
        # Get history items
        items = self.history_manager.get_all_items()
        
        # Apply search filter if search box has text
        search_text = self.search_var.get().lower()
        if search_text:
            items = [item for item in items if search_text in item.title.lower()]
        
        # Add items to the list
        for item in items:
            status = "Transcribed" if item.transcribed else "Audio Only"
            self.history_list.insert(
                "", 
                "end", 
                iid=item.item_id,
                values=(item.title, item.created_date_str, status)
            )
    
    def _filter_history(self):
        """Filter history list based on search box."""
        self._update_history_list()
    
    def _on_history_select(self, event):
        """Handle history item selection."""
        selected_items = self.history_list.selection()
        if not selected_items:
            # No selection
            self.current_item = None
            self.current_segments = []
            self.transcript_text.delete("1.0", tk.END)
            self._update_ui_state()
            return
        
        # Get selected item
        item_id = selected_items[0]
        item = self.history_manager.get_item_by_id(item_id)
        
        if item is None:
            # Item not found (should not happen)
            return
        
        # Update current item
        self.current_item = item
        
        # Load transcript if available
        if item.transcribed and item.transcript_exists:
            segments, metadata = self.history_manager.load_transcript(item.item_id)
            self.current_segments = segments
            self._display_transcript(segments)
        else:
            self.transcript_text.delete("1.0", tk.END)
            self.transcript_text.insert(tk.END, "[No transcript available]")
            self.current_segments = []
        
        # Update UI state
        self._update_ui_state()
    
    def _display_transcript(self, segments: List[TranscriptionSegment]):
        """Display transcript segments in the text area."""
        self.transcript_text.delete("1.0", tk.END)
        
        for segment in segments:
            # Format timestamp
            start_time = self._format_time(segment.start)
            end_time = self._format_time(segment.end)
            timestamp = f"[{start_time} → {end_time}] "
            
            # Insert timestamp with tag
            self.transcript_text.insert(tk.END, timestamp, "timestamp")
            
            # Insert segment text
            self.transcript_text.insert(tk.END, segment.text + "\n\n")
        
        # Configure timestamp tag
        self.transcript_text.tag_configure("timestamp", foreground=TEXT_SECONDARY)
    
    def _format_time(self, seconds: float) -> str:
        """Format time in seconds to MM:SS.ms format."""
        minutes, seconds = divmod(seconds, 60)
        return f"{int(minutes):02d}:{seconds:.2f}"
    
    def _toggle_recording(self):
        """Toggle recording state."""
        if self.state == AppState.RECORDING:
            self._stop_recording()
        else:
            self._start_recording()
    
    def _start_recording(self):
        """Start recording audio."""
        if self.state != AppState.IDLE:
            return
        
        # Start recording
        success, path_or_error = self.audio_recorder.start_recording()
        
        if success:
            # Update state
            self.state = AppState.RECORDING
            self._update_ui_state()
            
            # Start timer
            self._start_timer()
            
            # Log
            logger.info("Recording started")
        else:
            # Show error
            self._show_error("Recording Error", f"Failed to start recording: {path_or_error}")
    
    def _stop_recording(self):
        """Stop recording audio."""
        if self.state != AppState.RECORDING:
            return
        
        # Stop recording
        success, path_or_error = self.audio_recorder.stop_recording()
        
        # Stop timer
        self._stop_timer()
        
        # Update state
        self.state = AppState.IDLE
        self._update_ui_state()
        
        if success:
            # Add to history
            item = self.history_manager.add_recording(path_or_error)
            
            # Update history list
            self._update_history_list()
            
            # Select the new item
            self.history_list.selection_set(item.item_id)
            self.history_list.see(item.item_id)
            self._on_history_select(None)
            
            # Ask if user wants to transcribe
            if self._show_confirm("Transcribe", "Do you want to transcribe this recording?"):
                self._transcribe_selected()
            
            # Log
            logger.info(f"Recording stopped and saved: {path_or_error}")
        else:
            # Show error
            self._show_error("Recording Error", f"Failed to stop recording: {path_or_error}")
    
    def _start_timer(self):
        """Start the recording timer."""
        self.start_time = time.time()
        self._update_timer()
    
    def _stop_timer(self):
        """Stop the recording timer."""
        self.start_time = None
        self.timer_var.set("00:00:00")
    
    def _update_timer(self):
        """Update the recording timer."""
        if self.state != AppState.RECORDING or self.start_time is None:
            return
        
        # Calculate elapsed time
        elapsed = time.time() - self.start_time
        
        # Format time
        hours, remainder = divmod(elapsed, 3600)
        minutes, seconds = divmod(remainder, 60)
        time_str = f"{int(hours):02d}:{int(minutes):02d}:{int(seconds):02d}"
        
        # Update timer label
        self.timer_var.set(time_str)
        
        # Schedule next update
        self.root.after(1000, self._update_timer)
    
    def _upload_audio(self):
        """Upload an audio file."""
        if self.state != AppState.IDLE:
            return
        
        # Show file dialog
        file_path = filedialog.askopenfilename(
            title="Select Audio File",
            filetypes=[
                ("Audio Files", "*.mp3 *.wav *.m4a *.ogg"),
                ("All Files", "*.*")
            ]
        )
        
        if not file_path:
            # User cancelled
            return
        
        # Update state
        self.state = AppState.UPLOADING
        self._update_ui_state()
        
        try:
            # Save the file to history
            saved_path = self.audio_recorder.save_audio_file(file_path)
            
            if saved_path:
                # Add to history
                item = self.history_manager.add_recording(saved_path)
                
                # Update history list
                self._update_history_list()
                
                # Select the new item
                self.history_list.selection_set(item.item_id)
                self.history_list.see(item.item_id)
                self._on_history_select(None)
                
                # Ask if user wants to transcribe
                if self._show_confirm("Transcribe", "Do you want to transcribe this audio file?"):
                    self._transcribe_selected()
                
                # Log
                logger.info(f"Audio file uploaded: {saved_path}")
            else:
                # Show error
                self._show_error("Upload Error", "Failed to save audio file")
        finally:
            # Update state
            self.state = AppState.IDLE
            self._update_ui_state()
    
    def _transcribe_selected(self):
        """Transcribe the selected audio file."""
        if self.state != AppState.IDLE or self.current_item is None:
            return
        
        # Start transcription in a separate thread
        threading.Thread(target=self._run_transcription, daemon=True).start()
    
    def _run_transcription(self):
        """Run transcription in a background thread."""
        # Update state
        self.state = AppState.TRANSCRIBING
        
        # Update UI on main thread
        self.root.after(0, self._update_ui_state)
        
        try:
            # Get active model ID
            model_id = self.model_manager.get_active_model_id()
            
            # Clear transcript text
            self.root.after(0, lambda: self.transcript_text.delete("1.0", tk.END))
            
            # Start transcription
            segments, info = self.transcriber.transcribe(
                self.current_item.audio_path,
                model_id=model_id,
                progress_callback=self._update_transcription_progress
            )
            
            if "error" in info:
                # Show error
                self.root.after(0, lambda: self._show_error("Transcription Error", info["error"]))
                return
            
            # Save transcript
            transcript_path = self.history_manager.save_transcript(
                self.current_item.item_id,
                segments,
                metadata=info
            )
            
            if transcript_path:
                # Update current segments
                self.current_segments = segments
                
                # Display transcript
                self.root.after(0, lambda: self._display_transcript(segments))
                
                # Update history list
                self.root.after(0, self._update_history_list)
                
                # Log
                logger.info(f"Transcription completed: {transcript_path}")
            else:
                # Show error
                self.root.after(0, lambda: self._show_error(
                    "Transcription Error", 
                    "Failed to save transcript"
                ))
        finally:
            # Update state
            self.state = AppState.IDLE
            
            # Update UI on main thread
            self.root.after(0, self._update_ui_state)
    
    def _update_transcription_progress(self, progress: float):
        """Update transcription progress."""
        if self.state != AppState.TRANSCRIBING:
            return
        
        # Update progress bar on main thread
        self.root.after(0, lambda: self.progress_bar.place(relwidth=progress/100))
    
    def _update_model_selector(self):
        """Update the model selector with available models."""
        # Get installed models
        installed_models = self.model_manager.get_installed_models()
        
        # Get all models info
        all_models = {model["id"]: model["name"] for model in self.model_manager.get_all_models()}
        
        # Create the values list (installed models first)
        values = []
        for model_id in installed_models:
            values.append(f"{all_models[model_id]} (installed)")
        
        for model_id, model_name in all_models.items():
            if model_id not in installed_models:
                values.append(f"{model_name} (download)")
        
        # Update combobox
        self.model_selector["values"] = values
        
        # Set current model
        active_model_id = self.model_manager.get_active_model_id()
        for i, model_id in enumerate(all_models.keys()):
            if model_id == active_model_id and model_id in installed_models:
                self.model_selector.current(values.index(f"{all_models[model_id]} (installed)"))
                break
    
    def _on_model_select(self, event):
        """Handle model selection."""
        # Get selected item
        selected = self.model_selector.get()
        
        # Parse model name
        model_name = selected.split(" (")[0]
        
        # Get model ID from name
        model_id = None
        for model in self.model_manager.get_all_models():
            if model["name"] == model_name:
                model_id = model["id"]
                break
        
        if model_id is None:
            # Model not found (should not happen)
            return
        
        # Check if model is installed
        is_installed = self.model_manager.is_model_installed(model_id)
        
        if is_installed:
            # Set as active model
            self.model_manager.set_active_model(model_id)
            logger.info(f"Active model set to: {model_id}")
        else:
            # Ask if user wants to download
            if self._show_confirm("Download Model", 
                               f"The {model_name} model is not installed. Do you want to download it?"):
                self._download_model(model_id)
            else:
                # Revert selection
                active_model_id = self.model_manager.get_active_model_id()
                for model in self.model_manager.get_all_models():
                    if model["id"] == active_model_id:
                        self.model_var.set(f"{model['name']} (installed)")
                        break
    
    def _download_model(self, model_id: str):
        """Download a model."""
        # Start download in a separate thread
        threading.Thread(target=self._run_model_download, args=(model_id,), daemon=True).start()
    
    def _run_model_download(self, model_id: str):
        """Run model download in a background thread."""
        # Update state
        self.state = AppState.DOWNLOADING_MODEL
        
        # Update UI on main thread
        self.root.after(0, self._update_ui_state)
        
        try:
            # Get model info
            model_info = self.model_manager.get_model_info(model_id)
            
            # Show download info
            self.root.after(0, lambda: self.status_var.set(
                f"Downloading {model_info['name']} model ({model_info['size_mb']} MB)..."
            ))
            
            # Start download
            success = self.model_manager.download_model(
                model_id,
                progress_callback=self._update_download_progress
            )
            
            if success:
                # Set as active model
                self.model_manager.set_active_model(model_id)
                
                # Update model selector
                self.root.after(0, self._update_model_selector)
                
                # Show success message
                self.root.after(0, lambda: self._show_info(
                    "Download Complete",
                    f"The {model_info['name']} model has been downloaded and set as active."
                ))
                
                # Log
                logger.info(f"Model downloaded: {model_id}")
            else:
                # Show error
                self.root.after(0, lambda: self._show_error(
                    "Download Error",
                    f"Failed to download the {model_info['name']} model."
                ))
        finally:
            # Update state
            self.state = AppState.IDLE
            
            # Update UI on main thread
            self.root.after(0, self._update_ui_state)
    
    def _update_download_progress(self, progress: float):
        """Update download progress."""
        if self.state != AppState.DOWNLOADING_MODEL:
            return
        
        # Update progress bar on main thread
        self.root.after(0, lambda: self.progress_bar.place(relwidth=progress/100))
    
    def _copy_transcript(self):
        """Copy transcript text to clipboard."""
        if not self.current_segments:
            return
        
        # Get transcript text
        text = ""
        for segment in self.current_segments:
            text += segment.text + " "
        
        # Copy to clipboard
        self.root.clipboard_clear()
        self.root.clipboard_append(text.strip())
        
        # Show confirmation
        self.statusbar.configure(text="Transcript copied to clipboard")
        self.root.after(3000, lambda: self.statusbar.configure(text="Ready"))
    
    def _delete_selected(self):
        """Delete the selected history item."""
        if self.current_item is None:
            return
        
        # Ask for confirmation
        if not self._show_confirm("Delete Item", 
                               f"Are you sure you want to delete '{self.current_item.title}'?"):
            return
        
        # Delete item
        success = self.history_manager.delete_item(self.current_item.item_id)
        
        if success:
            # Clear current item
            self.current_item = None
            self.current_segments = []
            
            # Clear transcript
            self.transcript_text.delete("1.0", tk.END)
            
            # Update history list
            self._update_history_list()
            
            # Update UI state
            self._update_ui_state()
            
            # Log
            logger.info(f"Item deleted")
        else:
            # Show error
            self._show_error("Delete Error", "Failed to delete item")
    
    def _rename_selected(self):
        """Rename the selected history item."""
        if self.current_item is None:
            return
        
        # Show dialog
        new_title = self._askstring(
            "Rename Item", 
            "Enter new title:",
            initialvalue=self.current_item.title
        )
        
        if not new_title:
            # User cancelled
            return
        
        # Rename item
        success = self.history_manager.rename_item(self.current_item.item_id, new_title)
        
        if success:
            # Update current item
            self.current_item.title = new_title
            
            # Update history list
            self._update_history_list()
            
            # Reselect the item
            self.history_list.selection_set(self.current_item.item_id)
            
            # Log
            logger.info(f"Item renamed: {self.current_item.item_id} -> {new_title}")
        else:
            # Show error
            self._show_error("Rename Error", "Failed to rename item")
    
    def _export_selected(self):
        """Export the transcript of the selected item."""
        if self.current_item is None or not self.current_item.transcribed:
            return
        
        # Show file dialog
        file_path = filedialog.asksaveasfilename(
            title="Export Transcript",
            defaultextension=".txt",
            filetypes=[
                ("Text Files", "*.txt"),
                ("JSON Files", "*.json"),
                ("SRT Subtitles", "*.srt")
            ],
            initialfile=f"{self.current_item.title}_transcript"
        )
        
        if not file_path:
            # User cancelled
            return
        
        # Determine format
        format_type = "txt"
        if file_path.endswith(".json"):
            format_type = "json"
        elif file_path.endswith(".srt"):
            format_type = "srt"
        
        # Export transcript
        success = self.history_manager.export_transcript(
            self.current_item.item_id,
            file_path,
            format_type
        )
        
        if success:
            # Show confirmation
            self._show_info("Export Complete", f"Transcript exported to {file_path}")
            
            # Log
            logger.info(f"Transcript exported: {file_path}")
        else:
            # Show error
            self._show_error("Export Error", "Failed to export transcript")
    
    def _check_models(self):
        """Check if any models are installed, download tiny model if needed."""
        # Get installed models
        installed_models = self.model_manager.get_installed_models()
        
        if not installed_models:
            # No models installed, ask if user wants to download tiny model
            if self._show_confirm("Download Model", 
                               "No transcription models are installed. Would you like to download the Tiny model (75MB)?"):
                self._download_model("tiny")
    
    def _show_model_manager(self):
        """Show model manager dialog."""
        if self.state != AppState.IDLE:
            return
        
        # Create dialog
        dialog = tk.Toplevel(self.root)
        dialog.title("Model Manager")
        dialog.geometry("500x400")
        dialog.transient(self.root)
        dialog.grab_set()
        dialog.configure(bg=BACKGROUND)
        
        # Make dialog modal
        dialog.resizable(False, False)
        dialog.protocol("WM_DELETE_WINDOW", dialog.destroy)
        
        # Create frame
        frame = tk.Frame(
            dialog, 
            bg=BACKGROUND,
            bd=0
        )
        frame.pack(fill=tk.BOTH, expand=True, padx=15, pady=15)
        
        # Create header
        header_label = tk.Label(
            frame, 
            text="Manage Transcription Models",
            font=("Segue UI", 14, "bold"),
            bg=BACKGROUND,
            fg=TEXT_PRIMARY
        )
        header_label.pack(pady=(0, 15))
        
        # Create model list
        model_frame = tk.Frame(
            frame,
            bg=CONTENT_BG,
            bd=1,
            relief=tk.GROOVE,
            highlightbackground=BORDER
        )
        model_frame.pack(fill=tk.BOTH, expand=True, pady=10)
        
        # Configure columns
        columns = ("name", "size", "status")
        model_list = ttk.Treeview(
            model_frame,
            columns=columns,
            show="headings",
            height=10
        )
        
        model_list.heading("name", text="Model")
        model_list.heading("size", text="Size")
        model_list.heading("status", text="Status")
        
        model_list.column("name", width=150)
        model_list.column("size", width=100)
        model_list.column("status", width=150)
        
        model_list.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Add scrollbar
        scrollbar = tk.Scrollbar(
            model_list,
            orient=tk.VERTICAL,
            command=model_list.yview,
            bg=CONTENT_BG,
            troughcolor=CARD_BG,
            activebackground=ACCENT
        )
        model_list.configure(yscrollcommand=scrollbar.set)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        # Populate model list
        all_models = self.model_manager.get_all_models()
        installed_models = self.model_manager.get_installed_models()
        active_model_id = self.model_manager.get_active_model_id()
        
        for model in all_models:
            model_id = model["id"]
            model_name = model["name"]
            model_size = f"{model['size_mb']} MB"
            
            if model_id in installed_models:
                if model_id == active_model_id:
                    status = "Active"
                else:
                    status = "Installed"
            else:
                status = "Not Installed"
            
            model_list.insert("", "end", iid=model_id, values=(model_name, model_size, status))
        
        # Create button frame
        button_frame = tk.Frame(
            frame,
            bg=BACKGROUND
        )
        button_frame.pack(fill=tk.X, pady=10)
        
        def on_download():
            # Get selected model
            selected = model_list.selection()
            if not selected:
                return
            
            model_id = selected[0]
            model_info = next((m for m in all_models if m["id"] == model_id), None)
            
            if model_info and model_id not in installed_models:
                # Close dialog
                dialog.destroy()
                
                # Download model
                self._download_model(model_id)
        
        def on_set_active():
            # Get selected model
            selected = model_list.selection()
            if not selected:
                return
            
            model_id = selected[0]
            
            if model_id in installed_models:
                # Set as active
                self.model_manager.set_active_model(model_id)
                
                # Update model selector
                self._update_model_selector()
                
                # Close dialog
                dialog.destroy()
        
        def on_delete():
            # Get selected model
            selected = model_list.selection()
            if not selected:
                return
            
            model_id = selected[0]
            model_info = next((m for m in all_models if m["id"] == model_id), None)
            
            if model_info and model_id in installed_models:
                # Ask for confirmation
                if not self._show_confirm("Delete Model", 
                                       f"Are you sure you want to delete the {model_info['name']} model?"):
                    return
                
                # Delete model
                success = self.model_manager.delete_model(model_id)
                
                if success:
                    # Update model selector
                    self._update_model_selector()
                    
                    # Close dialog
                    dialog.destroy()
                else:
                    # Show error
                    self._show_error("Delete Error", f"Failed to delete the {model_info['name']} model")
        
        # Create buttons with custom colors
        def create_dialog_button(parent, text, command, color, width=12):
            """Create a button with custom styling"""
            btn = tk.Button(
                parent, 
                text=text, 
                command=command,
                bg=color,
                fg=TEXT_PRIMARY,
                activebackground=self._adjust_color(color, 1.1),
                activeforeground=TEXT_PRIMARY,
                relief=tk.FLAT,
                bd=1,
                padx=10,
                pady=2,
                width=width,
                font=("Segue UI", 9)
            )
            # Add hover effect
            btn.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(color, 1.1)))
            btn.bind("<Leave>", lambda e: e.widget.config(bg=color))
            return btn
        
        download_btn = create_dialog_button(button_frame, "Download", on_download, BTN_PRIMARY)
        download_btn.pack(side=tk.LEFT, padx=5)
        
        set_active_btn = create_dialog_button(button_frame, "Set Active", on_set_active, ACCENT)
        set_active_btn.pack(side=tk.LEFT, padx=5)
        
        delete_btn = create_dialog_button(button_frame, "Delete", on_delete, BTN_DANGER)
        delete_btn.pack(side=tk.LEFT, padx=5)
        
        close_btn = create_dialog_button(button_frame, "Close", dialog.destroy, BTN_SECONDARY)
        close_btn.pack(side=tk.RIGHT, padx=5)
    
    def _show_settings(self):
        """Show settings dialog."""
        if self.state != AppState.IDLE:
            return
        
        # Create dialog
        dialog = tk.Toplevel(self.root)
        dialog.title("Settings")
        dialog.geometry("500x400")
        dialog.transient(self.root)
        dialog.grab_set()
        dialog.configure(bg=BACKGROUND)
        
        # Create notebook tabs manually
        main_frame = tk.Frame(dialog, bg=BACKGROUND)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Create tab headers
        tab_frame = tk.Frame(main_frame, bg=BACKGROUND)
        tab_frame.pack(fill=tk.X)
        
        # Tab state
        active_tab = tk.StringVar(value="general")
        
        # Tab content frame
        content_frame = tk.Frame(
            main_frame, 
            bg=CONTENT_BG,
            bd=1,
            relief=tk.GROOVE,
            highlightbackground=BORDER
        )
        content_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        
        # Create tab content frames
        general_frame = tk.Frame(content_frame, bg=CONTENT_BG, padx=15, pady=15)
        audio_frame = tk.Frame(content_frame, bg=CONTENT_BG, padx=15, pady=15)
        transcription_frame = tk.Frame(content_frame, bg=CONTENT_BG, padx=15, pady=15)
        
        # Create tab buttons
        def change_tab(tab_name):
            """Change active tab"""
            active_tab.set(tab_name)
            general_frame.pack_forget()
            audio_frame.pack_forget()
            transcription_frame.pack_forget()
            
            if tab_name == "general":
                general_frame.pack(fill=tk.BOTH, expand=True)
            elif tab_name == "audio":
                audio_frame.pack(fill=tk.BOTH, expand=True)
            elif tab_name == "transcription":
                transcription_frame.pack(fill=tk.BOTH, expand=True)
        
        def create_tab_button(text, tab_name):
            """Create tab button with highlighting based on active state"""
            def update_style(*args):
                if active_tab.get() == tab_name:
                    btn.configure(bg=CONTENT_BG, relief=tk.SOLID)
                else:
                    btn.configure(bg=CARD_BG, relief=tk.FLAT)
            
            btn = tk.Button(
                tab_frame,
                text=text,
                bg=CARD_BG if active_tab.get() != tab_name else CONTENT_BG,
                fg=TEXT_PRIMARY,
                relief=tk.FLAT if active_tab.get() != tab_name else tk.SOLID,
                bd=1,
                padx=10,
                pady=5,
                font=("Segue UI", 9),
                command=lambda: change_tab(tab_name)
            )
            btn.pack(side=tk.LEFT, padx=(0, 1))
            
            # Update button style when active tab changes
            active_tab.trace_add("write", update_style)
            return btn
        
        # Create tab buttons
        general_tab = create_tab_button("General", "general")
        audio_tab = create_tab_button("Audio", "audio")
        transcription_tab = create_tab_button("Transcription", "transcription")
        
        # Get current config
        config = self.config.get_config()
        
        # Helper function to create setting rows
        def create_setting_row(parent, label_text, widget, row):
            """Create a consistent setting row with label and widget"""
            label = tk.Label(
                parent,
                text=label_text,
                font=("Segue UI", 10),
                bg=CONTENT_BG,
                fg=TEXT_PRIMARY,
                anchor=tk.W
            )
            label.grid(row=row, column=0, sticky=tk.W, pady=10)
            widget.grid(row=row, column=1, sticky=tk.W, pady=10)
        
        # General settings
        font_size_var = tk.IntVar(value=config["ui"]["font_size"])
        font_size_spinner = tk.Spinbox(
            general_frame,
            from_=8,
            to=18,
            textvariable=font_size_var,
            width=5,
            bg=CARD_BG,
            fg=TEXT_PRIMARY,
            buttonbackground=BTN_SECONDARY,
            relief=tk.FLAT,
            bd=1,
            highlightbackground=BORDER,
            highlightthickness=1,
            font=("Segue UI", 9)
        )
        create_setting_row(general_frame, "Font Size:", font_size_spinner, 0)
        
        # Audio settings
        # Get available devices
        devices = self.audio_recorder.get_available_devices()
        device_names = [device["name"] for device in devices]
        current_device = self.audio_recorder.get_current_device()
        
        device_var = tk.StringVar(value=current_device)
        device_dropdown = ttk.Combobox(
            audio_frame,
            textvariable=device_var,
            values=device_names,
            state="readonly",
            width=25
        )
        create_setting_row(audio_frame, "Audio Device:", device_dropdown, 0)
        
        sample_rate_var = tk.IntVar(value=config["audio"]["sample_rate"])
        sample_rates = [8000, 16000, 22050, 44100, 48000]
        sample_rate_dropdown = ttk.Combobox(
            audio_frame,
            textvariable=sample_rate_var,
            values=sample_rates,
            state="readonly",
            width=10
        )
        create_setting_row(audio_frame, "Sample Rate:", sample_rate_dropdown, 1)
        
        bitrate_var = tk.IntVar(value=config["audio"]["mp3_bitrate"])
        bitrates = [64, 96, 128, 192, 256, 320]
        bitrate_dropdown = ttk.Combobox(
            audio_frame,
            textvariable=bitrate_var,
            values=bitrates,
            state="readonly",
            width=10
        )
        create_setting_row(audio_frame, "MP3 Bitrate:", bitrate_dropdown, 2)
        
        # Transcription settings
        language_var = tk.StringVar(value=config["transcription"]["language"])
        languages = ["auto", "en", "fr", "de", "es", "it", "ja", "zh", "ru", "pt"]
        language_dropdown = ttk.Combobox(
            transcription_frame,
            textvariable=language_var,
            values=languages,
            state="readonly",
            width=10
        )
        create_setting_row(transcription_frame, "Language:", language_dropdown, 0)
        
        def create_checkbox(parent, var):
            """Create a custom styled checkbox"""
            frame = tk.Frame(parent, bg=CONTENT_BG)
            
            checkbox = tk.Checkbutton(
                frame,
                variable=var,
                bg=CONTENT_BG,
                activebackground=CONTENT_BG,
                selectcolor=CARD_BG,
                fg=ACCENT,
                activeforeground=ACCENT
            )
            checkbox.pack(side=tk.LEFT)
            return frame
        
        vad_var = tk.BooleanVar(value=config["transcription"]["vad_filter"])
        vad_check = create_checkbox(transcription_frame, vad_var)
        create_setting_row(transcription_frame, "VAD Filter:", vad_check, 1)
        
        word_timestamps_var = tk.BooleanVar(value=config["transcription"]["word_timestamps"])
        word_timestamps_check = create_checkbox(transcription_frame, word_timestamps_var)
        create_setting_row(transcription_frame, "Word Timestamps:", word_timestamps_check, 2)
        
        # Button frame
        button_frame = tk.Frame(dialog, bg=BACKGROUND)
        button_frame.pack(fill=tk.X, pady=10, padx=10)
        
        def on_save():
            # Update config
            self.config.update_config("ui", "font_size", font_size_var.get())
            
            # Update audio settings
            selected_device = device_var.get()
            for device in devices:
                if device["name"] == selected_device:
                    self.audio_recorder.set_audio_device(device["id"])
                    break
            
            self.config.update_config("audio", "sample_rate", sample_rate_var.get())
            self.config.update_config("audio", "mp3_bitrate", bitrate_var.get())
            
            # Update transcription settings
            self.config.update_config("transcription", "language", language_var.get())
            self.config.update_config("transcription", "vad_filter", vad_var.get())
            self.config.update_config("transcription", "word_timestamps", word_timestamps_var.get())
            
            # Update UI
            self.transcript_text.configure(font=("Segue UI", font_size_var.get()))
            
            # Close dialog
            dialog.destroy()
        
        def create_dialog_button(parent, text, command, color, width=10):
            """Create a button with custom styling"""
            btn = tk.Button(
                parent, 
                text=text, 
                command=command,
                bg=color,
                fg=TEXT_PRIMARY,
                activebackground=self._adjust_color(color, 1.1),
                activeforeground=TEXT_PRIMARY,
                relief=tk.FLAT,
                bd=1,
                padx=10,
                pady=2,
                width=width,
                font=("Segue UI", 9)
            )
            # Add hover effect
            btn.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(color, 1.1)))
            btn.bind("<Leave>", lambda e: e.widget.config(bg=color))
            return btn
        
        save_btn = create_dialog_button(button_frame, "Save", on_save, BTN_PRIMARY)
        save_btn.pack(side=tk.RIGHT, padx=5)
        
        cancel_btn = create_dialog_button(button_frame, "Cancel", dialog.destroy, BTN_SECONDARY)
        cancel_btn.pack(side=tk.RIGHT, padx=5)
        
        # Show the general tab by default
        change_tab("general")
    
    def _show_about(self):
        """Show about dialog with custom colors."""
        # Create dialog
        dialog = tk.Toplevel(self.root)
        dialog.title("About Scribe")
        dialog.geometry("400x300")
        dialog.transient(self.root)
        dialog.grab_set()
        dialog.resizable(False, False)
        dialog.configure(bg=BACKGROUND)
        
        # Create content
        frame = tk.Frame(
            dialog, 
            bg=BACKGROUND,
            bd=0
        )
        frame.pack(fill=tk.BOTH, expand=True, padx=20, pady=20)
        
        # Create title
        title_label = tk.Label(
            frame, 
            text="Scribe", 
            font=("Segue UI", 22, "bold"),
            bg=BACKGROUND,
            fg=ACCENT
        )
        title_label.pack(pady=(5, 0))
        
        # Create subtitle
        subtitle_label = tk.Label(
            frame, 
            text="Audio Transcription Tool",
            font=("Segue UI", 12),
            bg=BACKGROUND,
            fg=TEXT_PRIMARY
        )
        subtitle_label.pack(pady=(0, 10))
        
        # Create version
        version_label = tk.Label(
            frame, 
            text="Version 1.0.0",
            font=("Segue UI", 10),
            bg=BACKGROUND,
            fg=TEXT_SECONDARY
        )
        version_label.pack(pady=5)
        
        # Create separator
        separator = tk.Frame(
            frame,
            height=1,
            bg=BORDER
        )
        separator.pack(fill=tk.X, pady=15)
        
        # Create credits
        credits_label = tk.Label(
            frame, 
            text="Powered by Faster-Whisper",
            font=("Segue UI", 10),
            bg=BACKGROUND,
            fg=TEXT_SECONDARY
        )
        credits_label.pack(pady=5)
        
        copyright_label = tk.Label(
            frame, 
            text="© 2023 Bharat Bheesetti",
            font=("Segue UI", 10),
            bg=BACKGROUND,
            fg=TEXT_SECONDARY
        )
        copyright_label.pack(pady=5)
        
        # Create close button
        close_button = tk.Button(
            frame, 
            text="Close", 
            command=dialog.destroy,
            bg=BTN_SECONDARY,
            fg=TEXT_PRIMARY,
            activebackground=self._adjust_color(BTN_SECONDARY, 1.1),
            activeforeground=TEXT_PRIMARY,
            relief=tk.FLAT,
            bd=1,
            padx=10,
            pady=2,
            width=10,
            font=("Segue UI", 9)
        )
        close_button.pack(pady=15)
        
        # Add hover effect
        close_button.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(BTN_SECONDARY, 1.1)))
        close_button.bind("<Leave>", lambda e: e.widget.config(bg=BTN_SECONDARY))
    
    def _askstring(self, title, prompt, initialvalue=None):
        """Simple dialog to ask for a string with custom colors."""
        dialog = tk.Toplevel(self.root)
        dialog.title(title)
        dialog.transient(self.root)
        dialog.geometry("400x150")
        dialog.grab_set()
        dialog.resizable(False, False)
        dialog.configure(bg=BACKGROUND)
        
        # Create content frame
        frame = tk.Frame(
            dialog, 
            bg=BACKGROUND,
            bd=0
        )
        frame.pack(fill=tk.BOTH, expand=True, padx=20, pady=20)
        
        # Create prompt label
        label = tk.Label(
            frame, 
            text=prompt,
            font=("Segue UI", 10),
            bg=BACKGROUND,
            fg=TEXT_PRIMARY
        )
        label.pack(pady=(0, 10))
        
        # Create entry with custom colors
        string_var = tk.StringVar()
        if initialvalue:
            string_var.set(initialvalue)
        
        entry = tk.Entry(
            frame, 
            textvariable=string_var, 
            width=40,
            bg=CARD_BG,
            fg=TEXT_PRIMARY,
            insertbackground=TEXT_PRIMARY,
            selectbackground=ACCENT,
            selectforeground=TEXT_PRIMARY,
            relief=tk.FLAT,
            bd=1,
            highlightbackground=BORDER,
            highlightthickness=1
        )
        entry.pack(pady=10, fill=tk.X)
        entry.select_range(0, tk.END)
        entry.focus_set()
        
        result = [None]  # Use list to store result (mutable)
        
        def on_ok():
            result[0] = string_var.get()
            dialog.destroy()
        
        def on_cancel():
            dialog.destroy()
        
        # Create button frame
        button_frame = tk.Frame(
            frame,
            bg=BACKGROUND
        )
        button_frame.pack(fill=tk.X, pady=(10, 0))
        
        # Create OK button
        ok_button = tk.Button(
            button_frame, 
            text="OK", 
            command=on_ok,
            bg=BTN_PRIMARY,
            fg=TEXT_PRIMARY,
            activebackground=self._adjust_color(BTN_PRIMARY, 1.1),
            activeforeground=TEXT_PRIMARY,
            relief=tk.FLAT,
            bd=1,
            padx=10,
            pady=2,
            width=10,
            font=("Segue UI", 9)
        )
        ok_button.pack(side=tk.RIGHT, padx=5)
        
        # Add hover effect
        ok_button.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(BTN_PRIMARY, 1.1)))
        ok_button.bind("<Leave>", lambda e: e.widget.config(bg=BTN_PRIMARY))
        
        # Create Cancel button
        cancel_button = tk.Button(
            button_frame, 
            text="Cancel", 
            command=on_cancel,
            bg=BTN_SECONDARY,
            fg=TEXT_PRIMARY,
            activebackground=self._adjust_color(BTN_SECONDARY, 1.1),
            activeforeground=TEXT_PRIMARY,
            relief=tk.FLAT,
            bd=1,
            padx=10,
            pady=2,
            width=10,
            font=("Segue UI", 9)
        )
        cancel_button.pack(side=tk.RIGHT, padx=5)
        
        # Add hover effect
        cancel_button.bind("<Enter>", lambda e: e.widget.config(bg=self._adjust_color(BTN_SECONDARY, 1.1)))
        cancel_button.bind("<Leave>", lambda e: e.widget.config(bg=BTN_SECONDARY))
        
        # Handle Enter key
        dialog.bind("<Return>", lambda event: on_ok())
        dialog.bind("<Escape>", lambda event: on_cancel())
        
        # Center the dialog
        dialog.update_idletasks()
        width = dialog.winfo_width()
        height = dialog.winfo_height()
        x = (dialog.winfo_screenwidth() // 2) - (width // 2)
        y = (dialog.winfo_screenheight() // 2) - (height // 2)
        dialog.geometry(f"{width}x{height}+{x}+{y}")
        
        # Wait for dialog to close
        dialog.wait_window()
        
        return result[0]
    
    def _show_error(self, title, message):
        """Show an error message with custom styling."""
        return messagebox.showerror(title, message)
    
    def _show_info(self, title, message):
        """Show an info message with custom styling."""
        return messagebox.showinfo(title, message)
    
    def _show_confirm(self, title, message):
        """Show a confirmation dialog with custom styling."""
        return messagebox.askyesno(title, message)
    
    def _adjust_color(self, hex_color, factor):
        """Adjust a hex color by a factor (>1 = lighter, <1 = darker)"""
        # Extract RGB components
        r = int(hex_color[1:3], 16)
        g = int(hex_color[3:5], 16)
        b = int(hex_color[5:7], 16)
        
        # Adjust each component
        r = min(255, int(r * factor))
        g = min(255, int(g * factor))
        b = min(255, int(b * factor))
        
        # Convert back to hex
        return f"#{r:02x}{g:02x}{b:02x}"