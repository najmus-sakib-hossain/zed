
# Inspirations

```bash
git clone https://github.com/nidhaloff/deep-translator && cd deep-translator && rm -rf .git git clone https://github.com/pndurette/gTTS && cd gTTS && rm -rf .git git clone https://github.com/rany2/edge-tts && cd edge-tts && rm -rf .git gtts-cli 'hello master sumon. How are you? What you are doing currently? Are you creating dx??' --output gtts.mp3 edge-tts --text "hello master sumon. How are you? What you are doing currently? Are you creating dx??" --write-media hello.mp3 2>&1 | grep -v "RuntimeError: Event loop is closed" || true
```
