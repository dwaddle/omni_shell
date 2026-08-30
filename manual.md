# Omni Shell (omni) - Gebruikershandleiding

Omni Shell is een moderne, in Rust geschreven, custom shell die de features van gevestigde shells bundelt in één snelle, efficiënte binary.

## 🌟 Features
* **Vi-mode Standaard:** Bewerk je commandoregel natief via Vim-bindings (`Esc` voor normal mode, `i` voor insert, `w`/`b`/`dd`, etc.).
* **Autosuggestions:** (Fish-style) Toont grijs gedrukte commando's uit je geschiedenis tijdens het typen. Gebruik de pijl-naar-rechts om deze direct te accepteren.
* **Pipelines:** Verbind processen naadloos aan elkaar (`ls -la | grep src`).
* **Background Jobs:** Voeg een `&` toe aan het einde van je commando om het asynchroon in de achtergrond te draaien.
* **Aliases:** Maak custom snelkoppelingen voor lange commando's (in te stellen in je config file).

## ⚙️ Configuratie (omni.toml)
Bij het opstarten zoekt de shell naar `omni.toml` in de huidige map (latere versies kijken naar `~/.config/omni/omni.toml`). Hier kun je aliassen in definiëren.

Voorbeeld `omni.toml`:
```toml
[aliases]
ll = "ls -la"
gs = "git status"
update = "sudo pacman -Syu"
```

## 🛠️ Ingebouwde Commando's (Built-ins)
Naast je standaard OS-commando's ondersteunt Omni Shell de volgende interne operaties:
* `cd <pad>`: Wissel van actieve werkmap.
* `jobs`: Toon een lijst met momenteel draaiende achtergrondprocessen.
* `exit`: Sluit de shell af.

## 🚀 Installatie / Compileren
```bash
cd ~/projecten/omni_shell
cargo build --release
./target/release/omni_shell
```
