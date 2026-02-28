
# i18n Library Architecture

## Overview

This Rust library converts three Python packages into two comprehensive Rust modules:

### Python Sources → Rust Modules

+-------------------+----------+------+-------------+---------+
| Python            | Package  | Rust | Module      | Purpose |
+===================+==========+======+=============+=========+
| `deep-translator` | `locale` | Text | translation | across  |
+-------------------+----------+------+-------------+---------+



## Design Philosophy

The library follows these principles from the original Python implementations: -Trait-based Design: Similar to Python's base classes, we use Rust traits for polymorphism -Async-first: All I/O operations use `async/await` for better performance -Error Handling: Comprehensive error types instead of Python exceptions -Type Safety: Leveraging Rust's type system for compile-time guarantees

## Module Structure

### 1. Locale Module (`src/locale/`)

Converts `deep-translator` functionality to Rust. @tree:locale[] Key Components:

#### Translator Trait

```rust


#[async_trait]


pub trait Translator: Send + Sync { async fn translate(&self, text: &str) -> Result<String>;
async fn translate_batch(&self, texts: &[&str]) -> Result<Vec<String>>;
fn get_supported_languages(&self) -> Vec<&'static str>;
fn is_language_supported(&self, language: &str) -> bool;
fn source(&self) -> &str;
fn target(&self) -> &str;
}
```
This trait mirrors Python's `BaseTranslator` class with methods for: -Single text translation -Batch translation -Language validation -Language support queries

#### Implementation Comparison

Python (deep-translator):
```python
class GoogleTranslator(BaseTranslator):
def translate(self, text: str) -> str:


# HTTP request to Google Translate


response = requests.get(url, params=params)


# Parse HTML response


soup = BeautifulSoup(response.text)
return soup.find('div', class_='t0').text ```
Rust (i18n):
```rust
impl Translator for GoogleTranslator { async fn translate(&self, text: &str) -> Result<String> { // Async HTTP request let response = self.client.get(url).query(&params).send().await?;
// Parse HTML with scraper let document = Html::parse_document(&response.text().await?);
let selector = Selector::parse("div.t0").unwrap();
Ok(document.select(&selector).next().unwrap().text().collect())
}
}
```


### 2. TTS Module (`src/tts/`)


Combines `edge-tts` and `gTTS` functionality. @tree:tts[] Key Components:


#### TextToSpeech Trait


```rust

#[async_trait]

pub trait TextToSpeech: Send + Sync { async fn synthesize(&self, text: &str) -> Result<Vec<u8>>;
async fn save(&self, text: &str, path: &Path) -> Result<()>;
fn get_supported_languages(&self) -> Vec<&'static str>;
fn is_language_supported(&self, language: &str) -> bool;
}
```


#### Google TTS Implementation


Python (gTTS):
```python
class gTTS:
def save(self, savefile):
with open(savefile, 'wb') as f:
for chunk in self.stream():
f.write(chunk)
def stream(self):
for part in self._tokenize(self.text):
response = requests.post(url, data=self._package_rpc(part))

# Extract base64 audio from response

audio = base64.b64decode(match.group(1))
yield audio ```
Rust (i18n):
```rust
impl TextToSpeech for GoogleTTS { async fn synthesize(&self, text: &str) -> Result<Vec<u8>> { let parts = self.tokenize(text);
let mut audio_data = Vec::new();
for part in parts { let response = self.client.post(&url).body(data).send().await?;
let audio = self.extract_audio(&response.text().await?)?;
audio_data.extend(audio);
}
Ok(audio_data)
}
}
```

#### Edge TTS Implementation

Python (edge-tts):
```python
async def stream():
async with websocket.connect(url) as ws:
await ws.send(config_message)
await ws.send(ssml_message)
async for message in ws:
if message.type == aiohttp.WSMsgType.BINARY:
yield {'type': 'audio', 'data': message.data}
```
Rust (i18n):
```rust
impl TextToSpeech for EdgeTTS { async fn synthesize(&self, text: &str) -> Result<Vec<u8>> { let (ws_stream, _) = connect_async(&url).await?;
let (mut write, mut read) = ws_stream.split();
write.send(Message::Text(config)).await?;
write.send(Message::Text(ssml)).await?;
let mut audio = Vec::new();
while let Some(msg) = read.next().await { if let Message::Binary(data) = msg? { audio.extend(&data[header_len..]);
}
}
Ok(audio)
}
}
```

## Error Handling

### Python Approach (Exceptions)

```python
class LanguageNotSupportedException(BaseError):
pass try:
translation = translator.translate(text)
except LanguageNotSupportedException as e:
print(f"Error: {e}")
```

### Rust Approach (Result Types)

```rust


#[derive(Error, Debug)]


pub enum I18nError {


#[error("Language not supported: {0}")]


LanguageNotSupported(String), // ... other variants }
match translator.translate(text).await { Ok(translation) => println!("{}", translation), Err(I18nError::LanguageNotSupported(lang)) => { eprintln!("Error: {}", lang);
}
}
```

## Dependencies

### Translation (replacing Python requests + BeautifulSoup)

- `reqwest`: Async HTTP client (replaces `requests`)
- `scraper`: HTML parsing (replaces `BeautifulSoup`)
- `serde_json`: JSON handling

### TTS (replacing Python aiohttp + websockets)

- `tokio-tungstenite`: WebSocket client (replaces `aiohttp.ws_connect`)
- `base64`: Base64 encoding/decoding
- `regex`: Pattern matching for audio extraction

### Common

- `async-trait`: Trait async methods
- `thiserror`: Error derive macros
- `serde`: Serialization

## Key Differences from Python

### 1. Async Runtime

- Python: Built-in `asyncio`
- Rust: Uses `tokio` runtime, must be explicitly initialized

### 2. Error Handling

- Python: Exceptions with try/except
- Rust: Result types with match/? operator

### 3. Memory Management

- Python: Garbage collected
- Rust: Ownership system, no GC overhead

### 4. Type System

- Python: Dynamic typing
- Rust: Static typing with compile-time guarantees

### 5. Concurrency

- Python: GIL limits true parallelism
- Rust: True parallel execution with Send/Sync

## JSON Integration

The library is designed for JSON workflows:
```rust
// Parse JSON let messages: Vec<Message> = serde_json::from_str(json_str)?;
// Process for msg in messages { let translated = translator.translate(&msg.text).await?;
tts.save(&translated, &path).await?;
}
// Serialize back to JSON let output = serde_json::to_string_pretty(&results)?;
```

## Performance Characteristics

+-----------+----------+------------+-------------+
| Operation | Python   | Rust       | Improvement |
+===========+==========+============+=============+
| HTTP      | requests | Sequential | Concurrent  |
+-----------+----------+------------+-------------+



## Future Enhancements

Potential additions inspired by the Python packages: -More Providers: -DeepL translator -Libre translator -Yandex translator -TTS Features: -Voice listing and selection -Subtitle generation -Streaming audio output -Advanced Features: -Language detection -Text preprocessing -Audio post-processing

## Testing Strategy

Following Python package patterns:
```rust


#[cfg(test)]


mod tests {


#[tokio::test]


async fn test_google_translate() { let translator = GoogleTranslator::new("en", "es").unwrap();
let result = translator.translate("Hello").await.unwrap();
assert!(!result.is_empty());
}
}
```

## Documentation Standards

Following Rust conventions:
```rust
/// Translates text using Google Translate /// /// # Arguments /// * `text` - The text to translate /// /// # Examples /// ```no_run /// let translator = GoogleTranslator::new("en", "es")?;
/// let result = translator.translate("Hello").await?;
/// ```
/// /// # Errors /// Returns `I18nError::LanguageNotSupported` if language is invalid ```


## Conclusion


This architecture successfully converts three mature Python packages into a cohesive Rust library while: -Maintaining familiar APIs -Leveraging Rust's performance and safety -Providing async-first interfaces -Supporting JSON workflows -Enabling true parallel processing
