# Nocturne Optimizer

Windows-first desktop prototype zrobiony w **Tauri + Rust + React/TypeScript**.

## Sekcje
1. Live overview procesów i zużycia CPU/RAM/swap.
2. Live optymalizacja z regułami `Eco / Balanced / Freeze` nakładanymi na procesy w tle.
3. Autostart z wielu źródeł: `HKCU/HKLM Run`, `RunOnce`, foldery Startup, Scheduled Tasks, Services.
4. Offline optymalizacja: trzy presety.
5. Audyt ważnych kluczy rejestru: UAC, Secure Desktop, SmartScreen, LSA PPL.
6. Bezpieczeństwo: hasło, lista chronionych aplikacji, overlay po powrocie aplikacji na foreground, przełącznik file protection.
7. Ustawienia silnika.

## Uruchomienie
```bash
npm install
npm run tauri dev
```

## Budowa release
```bash
npm run build
npm run tauri build
```

## Ważne
- To jest **rozbudowany starter / prototyp produktu**, nie gotowy zamknięty produkt pod wszystkie wersje Windowsa.
- Część akcji związanych z usługami, taskami, autostartem i agresywną optymalizacją może wymagać uruchomienia aplikacji **jako administrator**.
- Tryb `Freeze` zawiesza proces. Nie każda aplikacja lubi wznowienie po suspend/resume.
- Overlay bezpieczeństwa jest realizowany jako osobne okno Tauri nad pulpitem, a nie jako wstrzyknięcie do procesu obcej aplikacji.
- `fileProtection` jest przygotowane pod vault lokalny; możesz rozwinąć je o szyfrowane sekrety na bazie DPAPI lub AES-GCM.

## Roadmapa do v2
- realny tray i background service,
- lepsza enumeracja okien per proces,
- bardziej granularne limity CPU/RAM przez Job Objects,
- pełny vault na dane lokalne,
- signed installer i update channel.
