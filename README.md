# 💩 Poepende Bram — Pterodactyl & GitHub Deployment

Deze map is speciaal ingericht voor de **Pterodactyl Rust Generic Egg** (`ghcr.io/ptero-eggs/yolks:rust_latest`).
Het bevat de complete Rust backend én de voorgebouwde `dist` web-app, zodat de server direct met `cargo run --release` kan opstarten zonder Node.js nodig te hebben.

---

## 🚀 Stap 1: Dit project op GitHub zetten

1. Open je browser en maak een nieuwe (public of private) repository aan op [GitHub](https://github.com/new), bijvoorbeeld `poepende-bram`.
2. Open je terminal in deze map (`C:\Poepende Bram\Poepende-Bram-Pterodactyl`) en voer uit:

```bash
git init
git add .
git commit -m "Initial commit: Poepende Bram Pterodactyl Rust Server"
git branch -M main
git remote add origin https://github.com/JOUW_GEBRUIKERSNAAM/poepende-bram.git
git push -u origin main
```

---

## 🎛️ Stap 2: Instellen in Pterodactyl Panel

Ga naar de server in het **Pterodactyl Panel** van je vriend:

### 1. Variables / Startup instellingen
Vul de variabelen in:
* **`GIT_ADDRESS`**: De URL van je GitHub repo (bijv. `https://github.com/JOUW_GEBRUIKERSNAAM/poepende-bram`)
* **`BRANCH`**: `main` (of laat leeg voor de standaard branch)
* **`AUTO_UPDATE`**: `1` (zodat bij elke herstart automatisch de nieuwste code van GitHub wordt opgehaald)
* **`USERNAME`** & **`ACCESS_TOKEN`**: *(Alleen nodig als je repo privé is)*: Je GitHub gebruikersnaam en een [Personal Access Token](https://github.com/settings/tokens).

### 2. Startup Detection (Configuratie)
In de Egg configuratie staat:
```json
"startup": {
    "done": [
        "POEPENDE BRAM SERVER IS READY!"
    ]
}
```
*(De server print letterlijk `POEPENDE BRAM SERVER IS READY!` zodra hij gestart is, zodat Pterodactyl direct weet dat de server online is!)*

### 3. Poort & Toegang
Pterodactyl geeft de server automatisch een poort mee via de omgevingsvariabele `SERVER_PORT`. De Rust server pakt deze automatisch op en bindt aan `0.0.0.0`.

---

## 📱 App openen en installeren

Zodra de server in Pterodactyl op groen staat:
1. Open de browser op je telefoon en ga naar:
   `http://SERVER_IP:POORT` (of via het gekoppelde domein met HTTPS)
2. Tik op **Zet op beginscherm** (iPhone) of **App installeren** (Android).
3. De app staat nu direct op je telefoonscherm!
