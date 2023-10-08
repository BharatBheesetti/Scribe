import os
import subprocess
import threading
import tkinter as tk
from tkinter import filedialog
from tkinter import ttk
import queue
from datetime import datetime

class TranscriberApp:
    def __init__(self, root):
        self.root = root
        self.initialize_ui()
        self.initialize_variables()
    
    
    
    def initialize_ui(self):
        self.root = root
        root.title("Scribe")
        root.state("zoomed")
        root.configure(bg="#A3A9B7")
        self.audio_device = "Microphone Array (Realtek(R) Audio)"
        self.fs = 44100
        self.duration = 7200
        self.output_directory = os.path.join(os.path.expanduser("~"), "Documents", "Scribe", "recordings")
        if not os.path.exists(self.output_directory):
            os.makedirs(self.output_directory)
        
        
        self.base_filename = "Scribe_Recording"
        self.start_time = None
        self.process = None
        self.output_path = None

        for i in range(5):
            root.grid_rowconfigure(i, weight=1)
            root.grid_columnconfigure(i, weight=1)

        title_label = tk.Label(root, text="Scribe", font=("Arial", 24), bg="#A3A9B7")
        title_label.grid(row=0, column=1, columnspan=3)
        
        subtitle_label = tk.Label(root, text="Take notes with AI", font=("Arial", 14), bg="#A3A9B7")
        subtitle_label.grid(row=1, column=1, columnspan=3, sticky="n")
        
        
        self.status_var = tk.StringVar(value=f"Your recordings will be at: {self.output_directory}")
        self.record_btn_text = tk.StringVar(value="Start new recording")
        self.upload_btn_text = tk.StringVar(value="Upload an mp3 file")      
        self.transcribe_btn_text = tk.StringVar(value="Transcribe Audio")

        
        self.record_btn = tk.Button(root, textvariable=self.record_btn_text, command=self.toggle_recording, width=20)
        self.record_btn.grid(row=2, column=0, ipadx=20, ipady=5)
        
        self.upload_btn = tk.Button(root, textvariable=self.upload_btn_text, command=self.upload_audio, width=20)
        self.upload_btn.grid(row=2, column=4, ipadx=20, ipady=5)

        self.transcribe_btn = tk.Button(root, textvariable=self.transcribe_btn_text, command=lambda: self.transcribe_audio(self.transcription_path), width=20)
        self.transcribe_btn.grid(row=2, column=2, ipadx=20, ipady=5)
        self.transcribe_btn.config(state=tk.DISABLED)

        self.status_label = tk.Label(root, textvariable=self.status_var)
        self.status_label.config(bg="#A3A9B7")
        self.status_label.grid(row=3, column=2, sticky="n")
      
        self.progress = ttk.Progressbar(root, orient='horizontal', length=300, mode='determinate')
        self.progress.grid(row=3, column=2, sticky='s')

        self.transcript_box = tk.Text(root, width=80, height=25)
        self.transcript_box.grid(row=4, column=0, columnspan=5, pady=10)
        self.transcript_box.config(state=tk.DISABLED)

    def initialize_variables(self):
        self.isRecording = False
        self.isTranscribing = False
        self.transcription_path = None

    def upload_audio(self):
        try:
            audio_path = filedialog.askopenfilename(title="Select Audio file", filetypes=[("Audio files", "*.mp3;*.wav")])
            if audio_path:
                self.transcription_path = audio_path
                self.output_path = audio_path
                self.status_var.set("Audio file successfully uploaded.")
                self.transcribe_btn.config(state=tk.NORMAL)  # Enable the transcribe button here
            else:
                self.status_var.set("No audio file selected.")
        except Exception as e:
            self.status_var.set(f"Error: {e}")


    def transcribe_audio(self, audio_path):
        if not self.isTranscribing and audio_path:
            self.isTranscribing = True
            self.q = queue.Queue()
            self.transcribing_thread = threading.Thread(target=self._transcribe_audio, args=(audio_path,), daemon=True)
            self.transcribing_thread.start()
            self.root.after(100, self.check_transcription_output)
        else:
            self.status_var.set("Previous transcription still in progress or no audio file selected.")

    
    def _transcribe_audio(self, audio_path):
        try:
            if audio_path:
                self.status_var.set("Transcription in progress...")
                self.process = subprocess.Popen(
                    ["python", "transcribe.py", audio_path],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True
                )
                for line in iter(self.process.stdout.readline, ''):
                    self.q.put(line)
                self.process.stdout.close()
                self.process.wait()
                self.update_gui("Transcription task ended.")
                self.reset_ui_after_transcription()  # Reset UI here
            else:
                self.status_var.set("No valid audio file selected.")
        except Exception as e:
            self.update_gui(f"Transcription failed: {e}")
            self.reset_ui_after_transcription()  # Reset UI here if exception occurs
            self.isTranscribing = False



    def check_transcription_output(self):
        try:
            line = self.q.get_nowait()
            print(f"Captured line: {line}")  # Add this line
            self.transcript_box.config(state=tk.NORMAL)
            if "PROGRESS:" in line:
                progress_value = float(line.split(":")[1])
                self.progress['value'] = progress_value
            else:
                self.transcript_box.insert(tk.END, line)
                self.transcript_box.see(tk.END)
            self.transcript_box.config(state=tk.DISABLED)
        except queue.Empty:
            pass
        
        finally:
            if self.transcribing_thread.is_alive():
                self.root.after(100, self.check_transcription_output)
            else:
                self.progress['value'] = 100
                self.reset_ui_after_transcription()  # Call the reset function here



    def start_recording(self):
        formatted_datetime = datetime.now().strftime("%d%m%Y_%H%M%S")
        output_filename = f"{self.base_filename}{formatted_datetime}.wav"
        self.output_path = os.path.join(self.output_directory, output_filename).replace("\\", "/")
        print(f"Attempting to save to {self.output_path}")

        self.start_time = datetime.now()

        self.process = subprocess.Popen([
            'ffmpeg', 
            '-f', 'dshow', 
            '-i', f'audio={self.audio_device}',
            self.output_path], 
            stdin=subprocess.PIPE, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.PIPE, text=True)
        self.ffmpeg_process = self.process


    def stop_recording(self):
        try:
            if self.process:
                self.process.stdin.write('q')
                self.process.stdin.flush()
                self.process.wait()
                self.process = None
                self.start_time = None
                self.transcribe_audio(self.output_path)
        except Exception as e:
            self.update_gui(f"Recording failed: {e}")
        self.transcription_path = self.output_path
            

# Commenting this method out. Assumption that faster-whisper can handle both mp3 and wav audio files

#    def convert_to_mp3(self, output_path):
#        if output_path:
#            output_directory, output_filename = os.path.split(output_path)
#            base_filename, _ = os.path.splitext(output_filename)
#            mp3_output_filename = f"{base_filename}.mp3"
#            mp3_output_path = os.path.join(output_directory, mp3_output_filename)
#            
#            subprocess.run(["ffmpeg","-i", output_path, "-codec:a", "libmp3lame", "-q:a", "2", mp3_output_path])
#            result = subprocess.run(["C:/Python311/python.exe", "transcribe.py", mp3_output_path], text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
#            print("MP3 conversion completed.")
#            transcript = result.stdout
#            self.display_transcript(transcript)
#            self.update_gui("Transcription completed.")

    def toggle_recording(self):
        if not self.isRecording:
            self.isRecording = True
            self.record_btn_text.set("Stop recording")
            self.status_var.set("Recording in progress...")
            self.recording_thread = threading.Thread(target=self.start_recording, daemon=True)
            self.recording_thread.start()
            self.update_timer()
            
        elif self.isRecording and not self.isTranscribing:
            self.isRecording = False
            self.status_var.set("Stopping recording...")
            self.recording_thread = threading.Thread(target=self.stop_recording, daemon=True)
            self.recording_thread.start()

#    def display_transcript(self, transcript):
#        self.transcript_box.delete("1.0", tk.END)
#        self.transcript_box.insert(tk.END, transcript)
#        self.status_var.set("Transcription completed.")
#        self.record_btn_text.set("Start new recording")
        
#    def copy_to_clipboard(self):
#        self.root.clipboard_clear()
#        self.root.clipboard_append(self.transcript_box.get("1.0", tk.END))

    def update_timer(self):
        if self.start_time:
            elapsed_time = datetime.now() - self.start_time
            self.status_var.set(f"Recording in progress... {elapsed_time}")
            self.root.after(1000, self.update_timer)

    def update_gui(self, message):
        self.status_var.set(message)
        self.record_btn.config(state=tk.NORMAL)
        self.upload_btn.config(state=tk.NORMAL)
        self.transcribe_btn.config(state=tk.NORMAL if self.transcription_path else tk.DISABLED)

    # Add this new function to reset the UI elements after transcription


    def reset_ui_after_transcription(self):
        self.isTranscribing = False
        self.record_btn_text.set("Start new recording")
        self.transcribe_btn.config(state=tk.DISABLED)  # Enable the transcribe button again
        self.upload_btn.config(state=tk.NORMAL)
        self.record_btn.config(state=tk.NORMAL if not self.isRecording else tk.DISABLED)
        self.transcription_path = None
        self.status_var.set("Transcription complete. Start a new recording or upload an audio file.")
        self.progress['value'] = 0

        
if __name__ == "__main__":
    root = tk.Tk()
    app = TranscriberApp(root)
    root.mainloop()
