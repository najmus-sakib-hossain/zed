import asyncio
import edge_tts
import gc
from typing import List, Dict, Any

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

async def generate_for_voice(text: str, voice: str) -> None:
    print(f"Generating TTS for voice: {voice}")
    communicate = edge_tts.Communicate(text, voice)
    audio_data: bytes = b""
    subtitles: List[Dict[str, Any]] = []
    async for chunk in communicate.stream():
        if chunk["type"] == "audio":
            audio_data += chunk["data"]  # type: ignore
        elif chunk["type"] == "WordBoundary":
            start = chunk["offset"] / 10000000  # type: ignore  # Convert from 100ns ticks to seconds
            end = (chunk["offset"] + chunk["duration"]) / 10000000  # type: ignore
            subtitles.append({
                "start": start,
                "end": end,
                "text": chunk["text"]  # type: ignore
            })
    
    # Write audio file
    audio_filename = f"hello_{voice}.mp3"
    with open(audio_filename, "wb") as f:
        f.write(audio_data)
    
    # Write subtitle file
    subtitle_filename = f"hello_{voice}.srt"
    with open(subtitle_filename, "w", encoding="utf-8") as f:
        for i, sub in enumerate(subtitles, 1):
            # Format timestamps as HH:MM:SS,mmm
            start_h = int(sub["start"] // 3600)
            start_m = int((sub["start"] % 3600) // 60)
            start_s = sub["start"] % 60
            start_str = f"{start_h:02}:{start_m:02}:{start_s:06.3f}".replace(".", ",")
            
            end_h = int(sub["end"] // 3600)
            end_m = int((sub["end"] % 3600) // 60)
            end_s = sub["end"] % 60
            end_str = f"{end_h:02}:{end_m:02}:{end_s:06.3f}".replace(".", ",")
            
            f.write(f"{i}\n{start_str} --> {end_str}\n{sub['text']}\n\n")
    
    print(f"Completed: {audio_filename} and {subtitle_filename}")

async def main() -> None:
    text = "hello master sumon. How are you? What you are doing currently? Are you creating dx??"
    voices = await edge_tts.list_voices()
    english_voices = [voice for voice in voices if voice["Name"].startswith("en-")]
    print(f"Found {len(english_voices)} English voices")
    for voice_info in english_voices:
        voice_name = voice_info["Name"]
        await generate_for_voice(text, voice_name)

asyncio.run(main())
gc.collect()