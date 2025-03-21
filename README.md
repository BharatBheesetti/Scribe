# Scribe - Audio Transcription Application

A desktop application for recording and transcribing audio using faster-whisper.

## System Requirements

- Python 3.8 or higher
- Windows 10/11
- Microphone or audio input device
- 4GB RAM minimum (8GB recommended)
- 2GB free disk space

## Python Package Requirements

- faster-whisper>=0.9.0
- torch>=2.0.0
- requests>=2.28.0
- sounddevice>=0.4.6
- numpy>=1.24.0

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/scribe.git
cd scribe
```

2. Create and activate a virtual environment:
```bash
python -m venv venv
.\venv\Scripts\activate  # Windows
source venv/bin/activate  # Linux/Mac
```

3. Install dependencies:
```bash
pip install -r requirements.txt
```

4. Run the application:
```bash
python main.py
```

## Building from Source

To build a standalone executable:

```bash
python build.py
```

This will create a standalone executable in the `dist/Scribe` directory.

## Usage

1. Launch the application
2. Click "Record" to start recording audio
3. Speak into your microphone
4. Click "Stop" to end recording
5. The audio will be automatically transcribed
6. View and edit the transcription in the text area

## Features

- Real-time audio recording
- High-quality transcription using faster-whisper
- Simple and intuitive interface
- Support for multiple languages
- Export transcriptions to text files

## License

This project is licensed under the MIT License - see the LICENSE file for details.