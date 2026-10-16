import sys
import os
import sounddevice as sd
import scipy.signal
import scipy.io.wavfile as wav
import numpy as np
from faster_whisper import WhisperModel

print("CARGANDO FASTER-WHISPER (INT8) EN RAM...", file=sys.stderr)
sys.stderr.flush()

model = WhisperModel("tiny", device="cpu", compute_type="int8")

print("LISTO", file=sys.stderr)
sys.stderr.flush()

duration = 4
native_fs = 44100
target_fs = 16000
wav_path = "aivi_mic_temp.wav"

while True:
    line = sys.stdin.readline()
    if not line:
        break
    
    command = line.strip()
    if command == "RECORD":
        try:
            audio = sd.rec(int(duration * native_fs), samplerate=native_fs, channels=1, dtype='float32')
            audio = np.squeeze(audio)
            sd.wait()
            
            max_amp = np.max(np.abs(audio))
            # 🔥 UMBRAL REDUCIDO (0.005): Ahora es 4 veces más sensible.
            # Captará tu voz aunque hables más bajo o estés un poco lejos.
            if max_amp < 0.005:
                print("") 
                sys.stdout.flush()
                continue
            
            number_of_samples = round(len(audio) * target_fs / native_fs)
            resampled_audio = scipy.signal.resample(audio, number_of_samples)
            audio_int16 = (resampled_audio * 32767).astype(np.int16)
            
            wav.write(wav_path, target_fs, audio_int16)
            
            # El VAD interno se encargará de ignorar el ruido estático
            segments, info = model.transcribe(
                wav_path, 
                language="es", 
                beam_size=5,
                vad_filter=True, 
                condition_on_previous_text=False
            )
            
            texto = " ".join([segment.text for segment in segments]).strip()
            
            if os.path.exists(wav_path):
                os.remove(wav_path)
                
            print(texto)
            sys.stdout.flush()
        except Exception as e:
            print(f"ERROR: {str(e)}", file=sys.stderr)
            sys.stdout.flush()
    elif command == "EXIT":
        break