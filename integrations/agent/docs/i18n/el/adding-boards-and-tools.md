# Προσθήκη Πλακετών και Εργαλείων — Οδηγός Υλικού DX

Αυτός ο οδηγός εξηγεί πώς να προσθέσετε νέες πλακέτες υλικού και προσαρμοσμένα εργαλεία στο DX.

## Γρήγορη Εκκίνηση: Προσθήκη Πλακέτας μέσω CLI

```bash
# Προσθήκη πλακέτας (ενημερώνει το ~/.dx/config.toml)

dx peripheral add nucleo-f401re /dev/ttyACM0
dx peripheral add arduino-uno /dev/cu.usbmodem12345
dx peripheral add rpi-gpio native   # για Raspberry Pi GPIO (Linux)

# Επανεκκίνηση του δαίμονα (daemon) για εφαρμογή

dx daemon --host 127.0.0.1 --port 42617
```

## Υποστηριζόμενες Πλακέτες

| Πλακέτα          | Μεταφορά (Transport) | Παράδειγμα Διαδρομής          |
|------------------|----------------------|-------------------------------|
| nucleo-f401re    | serial               | /dev/ttyACM0, /dev/cu.usbmodem* |
| arduino-uno      | serial               | /dev/ttyACM0, /dev/cu.usbmodem* |
| arduino-uno-q    | bridge               | (Uno Q IP)                    |
| rpi-gpio         | native               | native                        |
| esp32            | serial               | /dev/ttyUSB0                  |

## Χειροκίνητη Ρύθμιση (Manual Config)

Επεξεργαστείτε το αρχείο ~/.dx/config.toml:

```toml
[peripherals]
enabled = true
datasheet_dir = "docs/datasheets" # προαιρετικό: RAG για "άναψε το κόκκινο led" → pin 13

[[peripherals.boards]]
board = "nucleo-f401re"
transport = "serial"
path = "/dev/ttyACM0"
baud = 115200

[[peripherals.boards]]
board = "arduino-uno"
transport = "serial"
path = "/dev/cu.usbmodem12345"
baud = 115200
```

## Προσθήκη Φύλλου Δεδομένων (RAG)

Τοποθετήστε αρχεία .md ή .txt στο docs/datasheets/ (ή στον δικό σας κατάλογο datasheet_dir). Ονομάστε τα αρχεία βάσει της πλακέτας: nucleo-f401re.md, arduino-uno.md.

### Ψευδώνυμα Ακροδεκτών (Προτεινόμενο)

Προσθέστε μια ενότητα `## Pin Aliases` ώστε ο πράκτορας να μπορεί να αντιστοιχίσει το "red led" στον ακροδέκτη 13:

```markdown
# Η Πλακέτα Μου

## Pin Aliases

| ψευδώνυμο   | ακροδέκτης |
|-------------|------------|
| red_led     | 13         |
| builtin_led | 13         |
| user_led    | 5          |
```

Ή χρησιμοποιήστε μορφή κλειδιού-τιμής:

```markdown
## Pin Aliases

red_led: 13
builtin_led: 13
```

### Φύλλα Δεδομένων PDF

Με τη δυνατότητα rag-pdf, το DX μπορεί να ευρετηριάσει αρχεία PDF:

```bash
cargo build --features hardware,rag-pdf
```

Τοποθετήστε τα PDF στον κατάλογο των datasheet. Το περιεχόμενό τους εξάγεται και τεμαχίζεται (chunked) για το RAG.

## Προσθήκη Νέου Τύπου Πλακέτας

1. Δημιουργήστε ένα φύλλο δεδομένων — docs/datasheets/my-board.md με ψευδώνυμα ακροδεκτών και πληροφορίες GPIO.
2. Προσθήκη στις ρυθμίσεις — dx peripheral add my-board /dev/ttyUSB0
3. Υλοποίηση περιφερειακού (προαιρετικό) — Για προσαρμοσμένα πρωτόκολλα, υλοποιήστε το trait Peripheral στο src/peripherals/ και καταχωρίστε το στο create_peripheral_tools.

Δείτε το docs/hardware-peripherals-design.md για τον πλήρη σχεδιασμό.

## Προσθήκη Προσαρμοσμένου Εργαλείου

1. Υλοποιήστε το trait Tool στο src/tools/.
2. Καταχωρίστε το στο create_peripheral_tools (για εργαλεία υλικού) ή στο μητρώο εργαλείων του πράκτορα.
3. Προσθέστε μια περιγραφή εργαλείου στα tool_descs του πράκτορα στο src/agent/loop_.rs.

## Αναφορά CLI

| Εντολή | Περιγραφή |
|---------|-------------|
| dx peripheral list | Λίστα ρυθμισμένων πλακετών |
| dx peripheral add <board> <path> | Προσθήκη πλακέτας (εγγραφή στο config) |
| dx peripheral flash | Μεταφόρτωση (flash) υλικολογισμικού Arduino |
| dx peripheral flash-nucleo | Μεταφόρτωση (flash) υλικολογισμικού Nucleo |
| dx hardware discover | Λίστα συσκευών USB |
| dx hardware info | Πληροφορίες chip μέσω probe-rs |

## Αντιμετώπιση Προβλημάτων

- Η σειριακή θύρα δεν βρέθηκε — Σε macOS χρησιμοποιήστε το /dev/cu.usbmodem*. Σε Linux χρησιμοποιήστε το /dev/ttyACM0 ή το /dev/ttyUSB0.
- Μεταγλώττιση με υποστήριξη υλικού — cargo build --features hardware
- Probe-rs για Nucleo — cargo build --features hardware,probe
