import argparse
import sys
import os
import logging
from faster_whisper import WhisperModel

try:
    from faster_whisper import WhisperModel
except ImportError:
    logging.info("Installing faster-whisper...")
    os.system('pip install faster-whisper')
    from faster_whisper import WhisperModel



def initialize_model(args):
    try:
        return WhisperModel(args.model, device=args.device, compute_type=args.compute_type)
    except Exception as e:
        logging.error(f"Failed to initialize the model: {e}")
        sys.exit(1)

def transcribe_audio(model, audio_file, beam_size):
    try:
        return model.transcribe(audio_file, beam_size=beam_size)
    except Exception as e:
        logging.error(f"Failed to transcribe audio: {e}")
        sys.exit(1)

def print_transcription(segments, chunk_size):
    total_segments = len(segments)
    chunk = []
    for i, segment in enumerate(segments):
        transcript_text = f"[{segment.start:.2f}s -> {segment.end:.2f}s] {segment.text}\n"
        chunk.append(transcript_text)
        if (i + 1) % chunk_size == 0:
            print(''.join(chunk), end='', flush=True)
            progress = (i + 1) / total_segments * 100
            print(f"PROGRESS:{progress}", flush=True)
            chunk = []

    if chunk:
        print(''.join(chunk), end='')
        print("PROGRESS:100")

def main():
    logging.basicConfig(level=logging.INFO)
    parser = argparse.ArgumentParser(description="Transcribe audio files.")
    parser.add_argument("audio_file", help="Path to the audio file to transcribe.")
    parser.add_argument("--model", default="medium", help="Whisper model to use.")
    parser.add_argument("--device", default="cuda", choices=["cuda", "cpu"], help="Computation device.")
    parser.add_argument("--compute_type", default="float16", choices=["float16", "int8_float16", "int8"], help="Compute type.")
    parser.add_argument("--chunk_size", type=int, default=5, help="Number of segments per chunk")
    parser.add_argument("--beam_size", type=int, default=5, help="Beam size for transcription.")
    args = parser.parse_args()

    if not os.path.exists(args.audio_file):
        logging.error(f"Audio file {args.audio_file} not found.")
        sys.exit(1)

    model = initialize_model(args)
    segments_list = list(transcribe_audio(model, args.audio_file, args.beam_size)[0])
    total_segments = len(segments_list)
    print_transcription(segments_list, args.chunk_size)

if __name__ == "__main__":
    main()
