# Οδηγός Ενημέρωσης και Απεγκατάστασης στο macOS

Αυτή η σελίδα τεκμηριώνει τις υποστηριζόμενες διαδικασίες ενημέρωσης και απεγκατάστασης του DX στο macOS (OS X).

Τελευταία επαλήθευση: **22 Φεβρουαρίου 2026**.

## 1) Έλεγχος τρέχουσας μεθόδου εγκατάστασης

```bash
which dx
dx --version
```

Τυπικές τοποθεσίες:

- Homebrew: `/opt/homebrew/bin/dx` (Apple Silicon) ή `/usr/local/bin/dx` (Intel)
- Cargo/bootstrap/χειροκίνητη: `~/.cargo/bin/dx`

Αν υπάρχουν και οι δύο, η σειρά `PATH` του shell σας καθορίζει ποια εκτελείται.

## 2) Ενημέρωση στο macOS

### Α) Εγκατάσταση μέσω Homebrew

```bash
brew update
brew upgrade dx
dx --version
```

### Β) Εγκατάσταση μέσω Clone + bootstrap

Από τον τοπικό κλώνο του αποθετηρίου:

```bash
git pull --ff-only
./bootstrap.sh --prefer-prebuilt
dx --version
```

Αν θέλετε ενημέρωση μόνο από πηγαίο κώδικα:

```bash
git pull --ff-only
cargo install --path . --force --locked
dx --version
```

### Γ) Χειροκίνητη εγκατάσταση προκατασκευασμένου binary

Επαναλάβετε τη ροή λήψης/εγκατάστασης με το πιο πρόσφατο αρχείο έκδοσης και επαληθεύστε:

```bash
dx --version
```

## 3) Απεγκατάσταση στο macOS

### Α) Διακοπή και αφαίρεση υπηρεσίας background πρώτα

Αυτό αποτρέπει τη συνέχεια εκτέλεσης του daemon μετά την αφαίρεση του binary.

```bash
dx service stop || true
dx service uninstall || true
```

Αντικείμενα υπηρεσίας που αφαιρούνται από την `service uninstall`:

- `~/Library/LaunchAgents/com.dx.daemon.plist`

### Β) Αφαίρεση binary ανά μέθοδο εγκατάστασης

Homebrew:

```bash
brew uninstall dx
```

Cargo/bootstrap/χειροκίνητη (`~/.cargo/bin/dx`):

```bash
cargo uninstall dx || true
rm -f ~/.cargo/bin/dx
```

### Γ) Προαιρετικά: αφαίρεση τοπικών δεδομένων εκτέλεσης

Εκτελέστε αυτό μόνο αν θέλετε πλήρη εκκαθάριση ρυθμίσεων, προφίλ auth, logs και κατάστασης workspace.

```bash
rm -rf ~/.dx
```

## 4) Επαλήθευση ολοκλήρωσης απεγκατάστασης

```bash
command -v dx || echo "dx binary not found"
pgrep -fl dx || echo "No running dx process"
```

Αν το `pgrep` εξακολουθεί να βρίσκει διεργασία, σταματήστε την χειροκίνητα και ελέγξτε ξανά:

```bash
pkill -f dx
```

## Σχετικά Έγγραφα

- [One-Click Bootstrap](../one-click-bootstrap.md)
- [Αναφορά Εντολών](../commands-reference.md)
- [Αντιμετώπιση Προβλημάτων](../troubleshooting.md)
