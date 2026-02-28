
# gTTS

gTTS (Google Text-to-Speech), a Python library and CLI tool to interface with Google Translate's text-to-speech API. Write spoken `mp3` data to a file, a file-like object (bytestring) for further audio manipulation, or `stdout`. //gtts.readthedocs.io/

## Features

- Customizable speech-specific sentence tokenizer that allows for unlimited lengths of text to be read, all while keeping proper intonation, abbreviations, decimals and more;
- Customizable text pre-processors which can, for example, provide pronunciation corrections;

### Installation

```
$ pip install gTTS ```


### Quickstart


Command Line:
```
$ gtts-cli 'hello' --output hello.mp3 ```
Module:
```
>>> from gtts import gTTS
>>> tts = gTTS('hello')
>>> tts.save('hello.mp3')
```
See //gtts.readthedocs.io/ for documentation and examples.

### Disclaimer

This project is not affiliated with Google or Google Cloud. Breaking upstream changes can occur without notice. This project is leveraging the undocumented Google Translate speech functionality and is different from Google Cloud Text-to-Speech.

### Project

- Questions & community
- Changelog (CHANGELOG.md)
- Contributing (CONTRIBUTING.rst)

### Licence

The MIT License (MIT) (LICENSE) Copyright © 2014-2024 Pierre Nicolas Durette & Contributors
