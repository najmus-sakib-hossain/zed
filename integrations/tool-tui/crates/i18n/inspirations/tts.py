import asyncio
import gc
import edge_tts

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

async def main():
    text = "hello master sumon. How are you? What you are doing currently? Are you creating dx??"
    voice = "en-US-AriaNeural"  # Default voice; change if needed
    communicate = edge_tts.Communicate(text, voice)
    await communicate.save("edge_tts.mp3")
    voices = await edge_tts.list_voices()
    print(voices)

asyncio.run(main())
gc.collect()