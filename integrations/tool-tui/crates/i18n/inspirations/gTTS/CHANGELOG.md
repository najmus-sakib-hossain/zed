
# Changelog

## 2.5.4 (2024-11-10)

### Bug Fixes

- langs: Language added: `fr-CA` (French (Canada)) (#490) (defc400)

## 2.5.3 (2024-08-13)

### Bug Fixes

- langs: Languages added: `pt-PT` (Portuguese (Portugal)), `yue` (Cantonese); languages modified: `pa` (Punjabi (Gurmukhi)), `pt` (Portuguese (Brazil)) (72a7e57)

## 2.5.2 (2024-07-20)

### Bug Fixes

- langs: Languages added: `am` (Amharic), `cy` (Welsh), `eu` (Basque), `gl` (Galician), `ha` (Hausa), `lt` (Lithuanian), `pa` (Punjabi) (#477) (b04d6d1)

## 2.5.1 (2024-01-29)

### Bug Fixes

- Flush file after saving (#448) (262c532)

## 2.5.0 (2023-12-20)

### Features

- Add connection timeout + misc improvements (#440) (bcdb79d)

## 2.4.0 (2023-10-03)

### Features

- Add Python 3.12 support (75294b2)
- Remove Python 3.7 support (end-of-life) (75294b2)

## 2.3.2 (2023-04-29)

### Bug Fixes

- Add new error helper for when using a custom (non-`.com`) TLD results in a 404 (5a860ed)
- cli: Add deprecated language fallback support to CLI (5a860ed)

### Documentation

- cli: Fix older invalid example (5a860ed)

## 2.3.1 (2023-01-16)

### Bug Fixes

- test: include missing required `*.txt` test files in dist (#395) (63f10ff)
- loosen dependancies for `click` and `requests`, removes `six` dependancy (#394) (a4ce0c9)
- test: missing `@pytest.mark.net` on net-enabled test (#391) (3667f06)
- test: remove `mock` package test dependancy (#390) (9b54fc1)

## 2.3.0 (2022-11-21)

### Features

- centralizes project metadata and config into a single `pyproject.toml` (25d3c1c)
- drops support for Python 2.7 (long overdue), Python 3.6 (end-of-life) (25d3c1c)
- modernize package config and build/release workflow (25d3c1c)
- Simplify language generator (5dbdf10)

### Bug Fixes

- Languages added: `zh-CN` (Chinese (Simplified)), `zh-TW` (Chinese (Traditional)) (5dbdf10)
- Languages removed: `cy` (Welsh), `eo` (Esperanto), `mk` (Macedonian), `ms` (Malay), `zh-CN` (Chinese) (5dbdf10)

## 2.2.4 (2022-03-14)

### Features

- Added Malay language support (#316)
- Added Hebrew language support (#324)
- Added new `gTTS.stream()` method to stream bytes (#319)

### Misc

- #334

## 2.2.3 (2021-06-17)

### Features

- Added Bulgarian language support (#302)

## 2.2.2 (2021-02-03)

### Features

- Adds a language fallback feature for deprecated languages to maintain compatiblity (e.g. `en-us` becomes `en`). Fallback can be disabled with `lang_check=False` or `--nocheck` for the cli (#267)

### Bugfixes

- Fix Python 2.7 compatiblity (!). Python 2 is long gone, but the cut wasn't communicated for gTTS, so it was restored. Python 2 support will be completely removed in the next major release. (#255)
- Language code case sensitivity is maintained throughout (#267)

### Deprecations and Removals

- The following list of 'hyphenated' language codes no longer work and have been removed: `en-us`, `en-ca`, `en-uk`, `en-gb`, `en-au`, `en-gh`, `en-in`, `en-ie`, `en-nz`, `en-ng`, `en-ph`, `en-za`, `en-tz`, `fr-ca`, `fr-fr`, `pt-br`, `pt-pt`, `es-es`, `es-us`, `zh-cn`, `zh-tw` (#267)
- Removed the `gtts.get_url()` method (outdated since `2.1.0`) (#270)

## 2.2.1 (2020-11-15)

### Bugfixes

- `_package_rpc()` was erroneously packaging the entire text instead of tokenized part (#252)

### Improved Documentation

- Removes reference to automatic retrieval of languages (#250)

### Misc

- #251

## 2.2.0 (2020-11-14)

### Features

- Switch to the newer Google TTS API (thanks to@Boudewijn26!). See his great writeup for more on the methodology and why this was necessary. (#226, #232, #236, #241)

### Deprecations and Removals

- Removed automatic language download from the main code, which has become too unreliable & slow. Languages will still be fetched but a pre-generated list will be shipped with `gTTS`. (#233, #241, #242, #243)
- Because languages are now pre-generated, removed custom TLD support for language URL (which allowed to get language names in other than English) (#245)

### Misc

- #245

## 2.1.2 (2020-11-10)

### Features

- Update `gTTS-token` to `1.1.4` (#238)

### Bugfixes

- Fixed an issue where some tokens could be empty after minimization (#229, #239)

### Improved Documentation

- Grammar, spelling and example fixes (#227)

### Misc

- #218, #230, #231, #239

## 2.1.1 (2020-01-25)

### Bugfixes

- Debug mode now uses a copy of locals() to prevent RuntimeError (#213)

## 2.1.0 (2020-01-01)

### Features

- The `gtts` module-Added the ability to customize the Google Translate URL hostname. This is useful when `google.com` might be blocked within a network but a local or different Google host (e.g. `google.cn`) is not (#143, #203):-New `gTTS()` parameter `tld` to specify the top-level domain to use for the Google hostname, i.e `//translate.google.<tld>` (default: `com`).
- Languages are also now fetched using the same customized hostname.
- Pre-generated TTS API request URLs can now be obtained instead of writing an `mp3` file to disk (for example to be used in an external program):-New `get_urls()` method returns the list of URLs generated by `gTTS`, which can be used in lieu of `write_to_fp()` or `save()`.
- The `gtts-cli` command-line tool-New `--tld` option to match the new `gtts` customizable hostname #200, #207)
- Other-Added Python 3.8 support (#204)

### Bugfixes

- Changed default word-for-word pre-processor (`('M.', 'Monsieur')`) which would substitute any 'm.' for 'monsieur' (e.g. 'them.' became 'themonsieur') (#197)

### Improved Documentation

- Added examples for newer features (#205, #207)

### Misc

- #204, #205, #207

## 2.0.4 (2019-08-29)

### Features

- gTTS is now built as a wheel package (Python 2 & 3) (#181)

### Improved Documentation

- Fixed bad example in docs (#163, #166)

### Misc

- #164, #171, #173, #185

## 2.0.3 (2018-12-15)

### Features

- Added new tokenizer case for ':' preventing cut in the middle of a time notation (#135)

### Misc

- #159

## 2.0.2 (2018-12-09)

### Features

- Added Python 3.7 support, modernization of packaging, testing and CI (#126)

### Bugfixes

- Fixed language retrieval/validation broken from new Google Translate page (#156)

## 2.0.1 (2018-06-20)

### Bugfixes

- Fixed an UnicodeDecodeError when installing gTTS if system locale was not utf-8 (#120)

### Improved Documentation

- Added Pre-processing and tokenizing > Minimizing section about the API's 100 characters limit and how larger tokens are handled (#121)

### Misc

- #122

## 2.0.0 (2018-04-30)

(#108)

### Features

- The `gtts` module-New logger ("gtts") replaces all occurrences of `print()`
- Languages list is now obtained automatically (`gtts.lang`) (#91, #94, #106)
- Added a curated list of language sub-tags that have been observed to provide different dialects or accents (e.g. "en-gb", "fr-ca")
- New `gTTS()` parameter `lang_check` to disable language checking.
- `gTTS()` now delegates the `text` tokenizing to the API request methods (i.e. `write_to_fp()`, `save()`), allowing `gTTS` instances to be modified/reused
- Rewrote tokenizing and added pre-processing (see below)
- New `gTTS()` parameters `pre_processor_funcs` and `tokenizer_func` to configure pre-processing and tokenizing (or use a 3rd party tokenizer)
- Error handling:-Added new exception `gTTSError` raised on API request errors. It attempts to guess what went wrong based on known information and observed behaviour (#60, #106)
- `gTTS.write_to_fp()` and `gTTS.save()` also raise `gTTSError` on [gtts_token]{.title-ref} error
- `gTTS.write_to_fp()` raises `TypeError` when `fp` is not a file-like object or one that doesn't take bytes
- `gTTS()` raises `ValueError` on unsupported languages (and `lang_check` is `True`)
- More fine-grained error handling throughout (e.g. [request failed]{.title-ref} vs. [request successful with a bad response]{.title-ref})
- Tokenizer (and new pre-processors):-Rewrote and greatly expanded tokenizer (`gtts.tokenizer`)
- Smarter token 'cleaning' that will remove tokens that only contain characters that can't be spoken (i.e. punctuation and whitespace)
- Decoupled token minimizing from tokenizing, making the latter usable in other contexts
- New flexible speech-centric text pre-processing
- New flexible full-featured regex-based tokenizer (`gtts.tokenizer.core.Tokenizer`)
- New `RegexBuilder`, `PreProcessorRegex` and `PreProcessorSub` classes to make writing regex-powered text [pre-processors]{.title-ref} and [tokenizer cases]{.title-ref} easier
- Pre-processors:-Re-form words cut by end-of-line hyphens
- Remove periods after a (customizable) list of known abbreviations (e.g. "jr", "sr", "dr") that can be spoken the same without a period
- Perform speech corrections by doing word-for-word replacements from a (customizable) list of tuples
- Tokenizing:-Keep punctuation that modify the inflection of speech (e.g. "?", "!")
- Don't split in the middle of numbers (e.g. "10.5", "20,000,000") (#101)
- Don't split on "dotted" abbreviations and accronyms (e.g. "U.S.A")
- Added Chinese comma ("，"), ellipsis ("...") to punctuation list to tokenize on (#86)
- The `gtts-cli` command-line tool-Rewrote cli as first-class citizen module (`gtts.cli`), powered by Click
- Windows support using [setuptool]{.title-ref}'s [entry_points]{.title-ref}
- Better support for Unicode I/O in Python 2
- All arguments are now pre-validated
- New `--nocheck` flag to skip language pre-checking
- New `--all` flag to list all available languages
- Either the `--file` option or the `<text>` argument can be set to "-" to read from `stdin`
- The `--debug` flag uses logging and doesn't pollute `stdout` anymore

### Bugfixes

- `_minimize()`: Fixed an infinite recursion loop that would occur when a token started with the miminizing delimiter (i.e. a space) (#86)
- `_minimize()`: Handle the case where a token of more than 100 characters did not contain a space (e.g. in Chinese).
- Fixed an issue that fused multiline text together if the total number of characters was less than 100
- Fixed `gtts-cli` Unicode errors in Python 2.7 (famous last words) (#78, #93, #96)

### Deprecations and Removals

- Dropped Python 3.3 support
- Removed `debug` parameter of `gTTS` (in favour of logger)
- `gtts-cli`: Changed long option name of `-o` to `--output` instead of `--destination`
- `gTTS()` will raise a `ValueError` rather than an `AssertionError` on unsupported language

### Improved Documentation

- Rewrote all documentation files as reStructuredText
- Comprehensive documentation writen for Sphinx, published to //gtts.readthedocs.io
- Changelog built with towncrier

### Misc

- Major test re-work
- Language tests can read a `TEST_LANGS` enviromment variable so not all language tests are run every time.
- Added AppVeyor CI for Windows
- PEP 8 compliance

## 1.2.2 (2017-08-15)

### Misc

- Update LICENCE, add to manifest (#77)

## 1.2.1 (2017-08-02)

### Features

- Add Unicode punctuation to the tokenizer (such Chinese and Japanese) (#75)

### Bugfixes

- Fix > 100 characters non-ASCII split, `unicode()` for Python 2 (#71, #73, #75)

## 1.2.0 (2017-04-15)

### Features

- Option for slower read speed (`slow=True` for `gTTS()`, `--slow` for `gtts-cli`) (#40, #41, #64, #67)
- System proxy settings are passed transparently to all http requests (#45, #68)
- Silence SSL warnings from urllib3 (#69)

### Bugfixes

- The text to read is now cut in proper chunks in Python 2 unicode. This broke reading for many languages such as Russian.
- Disabled SSL verify on http requests to accommodate certain firewalls and proxies.
- Better Python 2/3 support in general (#9, #48, #68)

### Deprecations and Removals

- 'pt-br': 'Portuguese (Brazil)' (it was the same as 'pt' and not Brazilian) (#69)

## 1.1.8 (2017-01-15)

### Features

- Added `stdin` support via the '-' `text` argument to `gtts-cli` (#56)

## 1.1.7 (2016-12-14)

### Features

- Added utf-8 support to `gtts-cli` (#52)

## 1.1.6 (2016-07-20)

### Features

- Added 'bn': 'Bengali' (#39, #44)

### Deprecations and Removals

- 'ht': 'Haitian Creole' (removed by Google) (#43)

## 1.1.5 (2016-05-13)

### Bugfixes

- Fixed HTTP 403s by updating the client argument to reflect new API usage (#32, #33)

## 1.1.4 (2016-02-22)

### Features

- Spun-off token calculation to gTTS-Token (#23, #29)

## 1.1.3 (2016-01-24)

### Bugfixes

- `gtts-cli` works with Python 3 (#20)
- Better support for non-ASCII characters (#21, #22)

### Misc

- Moved out gTTS token to its own module (#19)

## 1.1.2 (2016-01-13)

### Features

- Added gTTS token (tk url parameter) calculation (#14, #15, #17)

## 1.0.7 (2015-10-07)

### Features

- Added `stdout` support to `gtts-cli`, text now an argument rather than an option (#10)

## 1.0.6 (2015-07-30)

### Features

- Raise an exception on bad HTTP response (4xx or 5xx) (#8)

### Bugfixes

- Added `client=t` parameter for the api HTTP request (#8)

## 1.0.5 (2015-07-15)

### Features

- `write_to_fp()` to write to a file-like object (#6)

## 1.0.4 (2015-05-11)

### Features

- Added Languages: `zh-yue`: `Chinese (Cantonese)`, `en-uk`: `English (United Kingdom)`, `pt-br`: `Portuguese (Brazil)`, `es-es`: `Spanish (Spain)`, `es-us`: `Spanish (United StateS)`, `zh-cn`: `Chinese (Mandarin/China)`, `zh-tw`: `Chinese (Mandarin/Taiwan)` (#4)

### Bugfixes

- `gtts-cli` print version and pretty printed available languages, language codes are now case insensitive (#4)

## 1.0.3 (2014-11-21)

### Features

- Added Languages: 'en-us': 'English (United States)', 'en-au': 'English (Australia)' (#3)

## 1.0.2 (2014-05-15)

### Features

- Python 3 support

## 1.0.1 (2014-05-15)

### Misc

- SemVer versioning, CI changes

## 1.0 (2014-05-08)

### Features

- Initial release
