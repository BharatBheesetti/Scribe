# Scribe - Secure Local Audio Transcription

Scribe is a privacy-focused desktop application for recording and transcribing audio directly on your computer. No cloud services, no data sharing, complete privacy.


![scribe_social_preview](https://github.com/user-attachments/assets/98616594-8dfe-441b-a58c-e123330a3a1d)

## Features

- **100% Local Processing** - All transcription happens on your machine
- **Privacy First** - Your audio never leaves your computer
- **Simple Interface** - Clean, dark-themed UI that's easy to use
- **Multiple Models** - Choose from tiny to medium-sized models based on your needs
- **Fast Transcription** - Powered by faster-whisper for efficient processing
- **Format Options** - Export transcripts as TXT, JSON, or SRT subtitles

## Quick Start

1. **Install Python**: Make sure you have Python 3.8+ installed
2. **Clone the repo**:
   ```
   git clone https://github.com/BharatBheesetti/Scribe.git
   cd Scribe
   ```
3. **Set up virtual environment**:
   ```
   python -m venv venv
   .\venv\Scripts\activate  # Windows
   source venv/bin/activate  # Linux/Mac
   ```
4. **Install dependencies**:
   ```
   pip install -r requirements.txt
   ```
5. **Run Scribe**:
   ```
   python main.py
   ```

The first time you run Scribe, it will offer to download the tiny model (~75MB). For better accuracy, you can download larger models from the Tools menu.

## System Requirements

- Windows 10/11 (macOS and Linux support coming soon)
- Python 3.8 or higher
- 4GB RAM minimum (8GB recommended)
- 2GB free disk space
- Microphone for recording

## Why Scribe?

- **Privacy**: No internet connection needed for transcription
- **Control**: You choose which model to use and where files are stored
- **Security**: Your sensitive audio never leaves your computer
- **Simplicity**: No accounts, no subscriptions, no complications

## Tech Stack

- **UI**: Tkinter with custom styling
- **Audio Processing**: sounddevice + wave
- **Transcription Engine**: faster-whisper
- **Models**: Compatible with Whisper models from tiny to large

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
