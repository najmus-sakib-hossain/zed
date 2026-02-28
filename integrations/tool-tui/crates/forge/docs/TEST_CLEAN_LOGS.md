
# Test Clean Logging

This is a test file to verify the new clean logging format. The logs should now show: -No emojis -No ANSI colors -Simple format: `[+]`, `[-]`, `[~]`, `[NEW]`, `[DEL]`, `[RENAME]` -Clean file names and line numbers -Short content previews (max 50 chars) Example expected output:
```
Logging initialized: logs/forge.log Watching for file changes...
[+] test.rs L10:5 "let x = 42;"
[~] config.json L3:8 "{ \"enabled\": true }"
[NEW] readme.md (25 lines)
[DEL] old-file.txt
[RENAME] temp.rs -> final.rs ```
Much cleaner and easier to read!
