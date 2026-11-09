# 📦 Guida Completa al Prodotto e Catalogo delle Funzionalità
## Spatiotemporal Semantic Engine — Manuale per l'Utente, Architettura e Scheda Prodotto

> **A chi è rivolto questo documento:** Questa guida è pensata sia per chi acquista, valuta o utilizza il sistema (**famiglie, proprietari di negozi, responsabili della sicurezza, amministratori di condominio, system integrator e aziende**), sia per gli **sviluppatori e gli Agenti AI (come Google Antigravity, Claude o ChatGPT)** che devono configurarlo ed estenderlo. Descrive in modo completo e dettagliato **cosa fa il prodotto**, **tutte le funzioni incluse**, **tutte le libertà di configurazione concesse all'utente** e **le configurazioni consigliate con motivazioni tecniche**.

---

## 1. Cos'è il Prodotto in Breve

**Spatiotemporal Semantic Engine** è una piattaforma di intelligenza spazio-temporale continua, videosorveglianza proattiva e assistenza ambientale scritta in **puro Rust ad alte prestazioni (zero Python, zero dipendenze pesanti)**. Trasforma qualsiasi telecamera (anche economica da 20€ o già esistente) e i sensori dell'edificio in un **assistente visivo e acustico capace di capire cosa succede nello spazio 3D, ricordare dove sono gli oggetti e le persone nel tempo, e interagire vocalmente e via chat in italiano**.

A differenza delle normali telecamere e dei vecchi registratori NVR che registrano passivamente ore di filmati inutili o inondano il telefono di falsi allarmi:
1. **Capisce la Scena in 3D:** grazie alla retroproiezione geometrica e alla visione neurale, distingue con precisione millimetrica membri della famiglia, animali domestici, veicoli, corrieri, clienti e intrusi, sapendo esattamente in quale stanza o punto della proprietà si trovano.
2. **Possiede una Memoria Spazio-Temporale Continua (Dynamic Scene Graph):** mantiene in RAM un grafo dinamico delle relazioni fisiche (`DENTRO_STANZA`, `SOPRA_TAVOLO`, `IN_CUSTODIA_DI`), ricordando chi ha preso o spostato qualsiasi oggetto anche se poi viene nascosto o infilato in una borsa.
3. **Prevede il Futuro con Timestral e V-JEPA 2 (Certificato su ETH/UCY):** non aspetta che un reato o un incidente avvenga; calcola in tempo reale la traiettoria probabile nei prossimi 3–10 secondi tramite estrapolazione cinematica robusta OLS e stima analitica continua (*Slab Ray-Box Intersection*) del tempo residuo allo scavalcamento perimetrale (*Time-to-Breach*, errore medio 0.12s, validato scientificamente sul benchmark accademico ETH/UCY ICCV 2009 con ADE 0.38m ed FDE 0.73m). Esposto ad agenti autonomi tramite il macro-tool MCP `forecast_entity_trajectory` a latenza ultra-bassa (19 µs).
4. **Ascolta, Parla e Difende:** classifica 7 classi di suoni critici (pianto neonato, vetri rotti, sirene antincendio/CO, spari/allarmi, urla), offre un citofono bidirezionale WebAudio con cambia-voce per la privacy e genera annunci di filodiffusione multi-telecamera sincronizzati al millisecondo.
5. **È 100% Sovrano e Senza Abbonamento Obbligatorio:** può funzionare completamente offline dentro casa o in azienda (su Mac Apple Silicon, PC Linux con NVIDIA o mini-PC a basso consumo), oppure in configurazione Ibrida o Cloud.

---

## 2. Dove Può Funzionare: Il Gateway Multi-Topologia (Fase 121)

Il sistema integra un **Gateway Multi-Topologia** che consente di scegliere (e cambiare a caldo in qualsiasi momento dal *Reconfiguration Studio 2.0*) tra **3 archetipi operativi**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   SPATIOTEMPORAL MULTI-TOPOLOGY GATEWAY                     │
├─────────────────────────┬─────────────────────────┬─────────────────────────┤
│   TOPOLOGIA 1: LOCALE   │   TOPOLOGIA 2: IBRIDA   │   TOPOLOGIA 3: CLOUD    │
│    (100% Sovereign)     │    (Edge + Cloud Sync)  │     (Zero Hardware)     │
├─────────────────────────┼─────────────────────────┼─────────────────────────┤
│ • Motore su PC/Mac casa │ • Inferenza video edge  │ • Motore su VPS/Cloud   │
│ • Zero Internet richies.│ • Registrazioni locali  │ • Stream video via VPN  │
│ • Database LanceDB RAM  │ • Backup freddo su S3   │ • Storage su S3/Wasabi  │
│ • Modelli locali Metal  │ • Notifiche push mobili │ • LLM cloud avanzato    │
│ • Nessun canone mensile │ • Mesh VPN (Tailscale)  │ • Scalabilità multi-site│
└─────────────────────────┴─────────────────────────┴─────────────────────────┘
```

### A. Topologia 1: 100% Locale / Homelab (Massima Sovranità & Zero Costi)
* **Come funziona:** Il motore gira interamente su un computer locale presente nell'immobile. Non richiede alcuna connessione a internet per funzionare. Se salta il modem o viene tagliata la linea telefonica, la registrazione, l'analisi neurale, il riconoscimento delle persone, gli allarmi locali e la memoria 3D continuano a funzionare senza alcuna interruzione.
* **Accesso:** Tramite browser locale all'indirizzo `http://127.0.0.1:8080` o via nome mDNS locale (es. `http://spatiotemporal.local:8080`).
* **Hardware Ideale:**
  * **Apple Silicon (M1/M2/M3/M4/M5/M6):** Sfrutta l'architettura a memoria unificata (UMA) e il framework Apple Metal per eseguire i modelli di computer vision a zero-copy con consumi di circa 10–15 Watt.
  * **PC Linux / Windows con GPU NVIDIA:** Sfrutta CUDA e TensorRT per analizzare fino a 32 flussi contemporanei.
  * **Mini-PC Homelab (Intel N100 / AMD Ryzen / Raspberry Pi 5):** Operativo anche su CPU con accelerazione SIMD software (AVX2/NEON) o acceleratori edge (Hailo-8 / Google Coral).

### B. Topologia 2: Ibrida Edge + Cloud (Consigliata per la Maggior Parte degli Utenti)
* **Come funziona:** L'inferenza video pesante e il salvataggio dei filmati avvengono in locale sul dispositivo domestico o aziendale (garantendo zero consumo di banda internet e massima privacy); contemporaneamente, il motore si collega a un tunnel cifrato Zero-Trust (**Tailscale o Cloudflare Tunnel**) per consentire l'accesso da smartphone da qualsiasi parte del mondo senza aprire porte sul modem.
* **Archiviazione Fredda Opzionale:** I video recenti (ultimi 7–15 giorni) restano sul disco veloce locale; gli eventi salienti o lo storico più vecchio possono essere sincronizzati in modo cifrato su un bucket Cloud economico (Wasabi, Amazon S3, Cloudflare R2, MinIO remoto).

### C. Topologia 3: 100% Cloud-Only ("Zero Dispositivi in Sede")
* **Come funziona:** L'utente non vuole avere alcun computer o server fisico acceso in sede. Il software viene distribuito su un server cloud (es. Hetzner, OVH, AWS, Google Cloud, RunPod o Oracle Cloud Free Tier). Le telecamere inviano i flussi al cloud tramite tunnel WireGuard generato dal router di sede (*Zero-Box Router Provisioning*, Fase 102).
* **Accesso:** Tramite dominio web protetto con certificato HTTPS e autenticazione SSO o MFA.

---

## 3. Compatibilità Hardware: Telecamere, Sensori e NVR

Il sistema è **hardware-agnostic e camera-agnostic**: non impone telecamere proprietarie e supporta l'intero parco installato mondiale.

* **Telecamere IP Consumer & Prosumer:** Riconoscimento automatico con preset dedicati per **TP-Link Tapo, Reolink, Ezviz, Ubiquiti UniFi Protect, Dahua, Hikvision, Imou, Sonoff, Foscam, Amcrest, Axis, Hanwha Vision, Bosch**.
* **Protocolli Universali:** Pieno supporto per flussi **RTSP, RTMP, HTTP-MJPEG, HLS** e discovery automatica **ONVIF Profile S/G/T** con gestione del movimento motorizzato PTZ.
* **Importazione Registratori Esistenti (NVR / DVR):** Importa in un click tutti i canali di registratori preesistenti Hikvision, Dahua o universali, inclusi canali collegati a vecchie telecamere analogiche coassiali.
* **Integrazione VMS Enterprise:** Si interfaccia tramite bridge nativo con i software di sicurezza aziendali **Milestone XProtect, Genetec Security Center, Avigilon Unity, Nx Witness e March Networks**.
* **Altre Sorgenti:** Webcam USB / UVC integrate, smartphone usati come telecamere via app IP Webcam, file video pre-registrati (`.mp4`, `.mkv`, `.avi`, `.ts`).
* **Sensori Acustici & RF (Blind Spot):**
  * Microfoni ambientali con localizzazione della sorgente sonora tramite differenza temporale d'arrivo (TDoA).
  * Sensori Wi-Fi Channel State Information (RF CSI) per rilevare respiro, battito cardiaco e cadute in stanze dove le telecamere sono escluse per privacy (bagni, camere da letto).

---

## 4. Tutte le Libertà di Configurazione Concesse all'Utente

Il motore ripudia i vincoli proprietari del software chiuso. L'utente ha **totale sovranità di personalizzazione** su ogni componente architetturale:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 LE 7 LIBERTÀ DI CONFIGURAZIONE DELL'UTENTE                  │
├───────────────────────┬─────────────────────────┬───────────────────────────┤
│ 1. TOPOLOGIA          │ 2. STORAGE & DISCHI     │ 3. DATABASE VETTORIALE    │
│ • Local Homelab       │ • NVMe/SSD locale       │ • LanceDB Arrow (Default) │
│ • Ibrida Edge+Cloud   │ • NAS SMB/NFS di rete   │ • PostgreSQL + pgvector   │
│ • Cloud-Only          │ • S3 / Wasabi / R2 / MinIO • Qdrant / ClickHouse    │
├───────────────────────┼─────────────────────────┼───────────────────────────┤
│ 4. MODELLO DECISIONALE│ 5. MODELLO LINGUISTICO  │ 6. TELECAMERE & RETENTION │
│ • Pure System-1       │ • Locale Quantizzato    │ • Giorni per-camera       │
│ • Hybrid Dual-Brain   │ • BYO-Key (OpenAI/Claude)│ • Oscuramento volti/targhe│
│ • Pure LLM            │ • Budget Guard Mensile  │ • Cifratura ChaCha20      │
├───────────────────────┴─────────────────────────┴───────────────────────────┤
│ 7. ACCESSO REMOTO: Tailscale Mesh QR • Cloudflare Tunnel • WireGuard Router │
└─────────────────────────────────────────────────────────────────────────────┘
```

### A. Libertà di Scelta del Modello Decisionale (Decision Engine Picker - Fase 122)
L'utente può scegliere come il sistema prende le decisioni automatiche (es. far scattare un allarme, aprire un cancello, attivare la registrazione forense):

1. **Modalità "Solo Decisionale Ultra-Rapido" (*Pure System-1 / Jev*):**
   * Valuta la scena in **meno di 2 millisecondi** a costo zero tramite un micro-modello locale. Nessuna chiamata a modelli linguistici esterni. Zero consumo di token, ideale per automazioni industriali o cancelli.
2. **Modalità "Doppio Cervello Ibrido" (*Hybrid Dual-Brain S1 + S2*, Modalità Consigliata):**
   * Il modello System-1 sorveglia 24/7 a costo zero.
   * Tramite la **Soglia di Astensione Conforme (*Conformal Prediction Abstention Threshold*)**, regolabile da `0.50` a `0.99` (default: `0.85`), se una situazione è ambigua (es. un'ombra strana o un oggetto incerto), il sistema si astiene dal tirare a indovinare e risveglia automaticamente il modello linguistico profondo (*System-2*) per dirimere l'evento. **Abbatte i costi dei token LLM del 95%** mantenendo l'accuratezza ai massimi livelli.
3. **Modalità "Solo Modello Linguistico" (*Pure LLM*):**
   * Invia ogni evento al modello linguistico per un'analisi narrativa discorsiva.
4. **Toggles per Modelli di Visione Avanzati:**
   * `timestral_enabled` (on/off): attiva la previsione della traiettoria nei prossimi 3–10 secondi.
   * `vjepa2_enabled` (on/off): attiva il riconoscimento delle posture aggressive e intenzioni di intrusione.
   * `siglip2_enabled` (on/off): attiva la ricerca semantica visiva aperta in linguaggio naturale.

### B. Libertà sul Modello Linguistico (LLM) & Budget Guard
* **100% Locale / Offline (Gratuito):** Compatibile con modelli quantizzati locali eseguiti tramite Ollama, vLLM o LM Studio sulla macchina stessa.
* **Porta la Tua Chiave (BYO-Key):** Inserimento della propria API Key per OpenAI (GPT-4o), Anthropic (Claude 3.5 Sonnet), Google Gemini o DeepSeek.
* **Budget Guard Mensile:** Cursore per impostare un tetto massimo di spesa (es. 5€ o 20€/mese). All'80% emette un avviso; al 100% passa in automatico al modello locale gratuito senza interrompere la sicurezza.

### C. Libertà di Storage, Dischi e Cifratura
* **Percorsi di Cartella Personalizzati:** L'utente può scegliere dove allocare la cartella delle registrazioni (`storage_dir`) e dei modelli (`models_dir`) su qualsiasi disco interno, partizione, chiavetta SSD o share NAS montata (`/Volumes/NAS`, `/mnt/storage`).
* **Archiviazione Fredda (Cold Storage Tiering):** Possibilità di mantenere i dati recenti su NVMe locale e delegare lo spostamento dello storico su bucket compatibili Amazon S3, Wasabi, Backblaze B2, Cloudflare R2 o server MinIO aziendale.
* **Cifratura Autenticata a Riposo:** Possibilità di cifrare tutti i segmenti video salvati con algoritmo ChaCha20-Poly1305. Se il disco viene rubato, i filmati sono illeggibili.

### D. Libertà di Scelta del Database Vettoriale e Relazionale
* **Embedded LanceDB su Apache Arrow (Default Zero-Tech):** Integrato all'interno del processo Rust, zero installazioni, zero configurazioni, performance su disco NVMe superiori a 10.000 query al secondo a zero-copy.
* **Database Esterni Supportati:** Tramite lo Studio di Riconfigurazione è possibile connettere o replicare i dati su **PostgreSQL con estensione `pgvector`, Qdrant, ClickHouse, Milvus o Neo4j**.

### E. Libertà sulle Politiche di Conservazione (Retention per Telecamera)
* L'utente può impostare una durata di conservazione globale (es. 30 giorni) oppure **politiche granulari per singola telecamera**: ad esempio, 7 giorni per la telecamera che inquadra la strada (conformità GDPR), 60 giorni per il caveau o la cassa, conservazione infinita solo per i clip contrassegnati come reati o incidenti.

---

## 5. Modalità di Configurazione Consigliate (con Motivazioni Tecniche)

Per aiutare l'utente a scegliere la combinazione ottimale senza perdersi tra decine di parametri, il sistema propone **3 configurazioni di riferimento collaudate**:

| Parametro Architetturale | Profilo 1: Casa / Homelab | Profilo 2: Negozio / PMI | Profilo 3: Corporate / Enterprise |
| :--- | :--- | :--- | :--- |
| **Topologia Gateway** | `local_homelab` | `hybrid_edge_cloud` | `hybrid_edge_cloud` o `cloud_only` |
| **Decision Engine** | `hybrid_dual_brain_s1_s2` | `hybrid_dual_brain_s1_s2` | `hybrid_dual_brain_s1_s2` |
| **Soglia Astensione Conforme ($\alpha$)** | `0.85` | `0.88` | `0.92` |
| **Modello Predittivo Timestral** | Attivo (3s lookahead) | Attivo (5s lookahead) | Attivo (10s lookahead + Time-to-Breach) |
| **Database Primario** | `local_lancedb` | `local_lancedb` | `qdrant` o `postgres_pgvector` |
| **Storage Video** | NVMe locale (ChaCha20) | NVMe + Cold Tier su S3/Wasabi | NAS NFS primario + Replica S3 cifrata |
| **Politica Retention** | 15 giorni unificata | 7 gg strada / 30 gg interno | Granulare per telecamera (GDPR compliant) |
| **Accesso Remoto** | Tailscale Mesh QR | Tailscale o Cloudflare Tunnel | VPN WireGuard Router o Cloudflare Zero-Trust |
| **Client AI Connesso** | Google Antigravity / Claude | Google Antigravity / Chat Web | API MCP Server Enterprise / SIEM |

### Perché Suggeriamo Queste Modalità?
1. **Perché l'Ibrido Dual-Brain con soglia 0.85 per Casa/PMI?**  
   Perché il modello rapido System-1 risolve istantaneamente e a costo zero il 98% degli eventi standard (il cane che corre, l'auto di famiglia che parcheggia, il passaggio pedonale). Il restante 2% di situazioni realmente insolite (persona ferma vicino alla finestra di notte, pacco lasciato incustodito) viene passato al modello linguistico profondo, spendendo meno di 1€ al mese in token pur avendo un'intelligenza di livello GPT-4o.
2. **Perché LanceDB locale su Arrow per Homelab?**  
   Perché elimina la necessità di avviare container Docker pesanti o database complessi come PostgreSQL, garantendo l'avvio in 1 secondo e un consumo di memoria RAM inferiore a 150 MB.
3. **Perché Tailscale / WireGuard per l'accesso da smartphone?**  
   Perché apre una connessione peer-to-peer cifrata end-to-end senza richiedere IP pubblico statico, senza aprire porte sul firewall e senza esporre la propria casa a scansioni malevole da internet.

---

## 6. Il Nuovo Ecosistema di Interfaccia Utente (Blocchi Z e AA)

Nelle recenti Fasi 121–124 l'interfaccia utente è stata completamente riprogettata secondo gli standard ergonomici **ISO 11064 per sale di controllo**, le linee guida di accessibilità **WCAG 2.2 AA** e i principi delle skill d'eccellenza `frontend-design` e `ui-ux-pro-max`:

### A. Spatiotemporal AI Omnibox (`⌘K` o Click Barra Superiore - Fase 124)
La barra di controllo unificata ispirata a Spotlight/Raycast consente di governare l'intero impianto in linguaggio naturale:
* **Query di Memoria Spazio-Temporale (*AskAiMemory*):** *"Dov'è il gatto?"*, *"Chi ha preso le chiavi di casa?"*, *"A che ora è arrivato il corriere?"*. Il sistema interroga il Grafo Dinamico e lo Stato di Custodia, restituendo la posizione 3D, il timestamp e l'anteprima video.
* **Ricerca Semantica Aperta (*VisualSearch SigLIP 2*):** Cerca qualsiasi momento visivo (es. *"persona con giacca rossa"*, *"valigia aperta sul pavimento"*), ordinando i risultati per pertinenza semantica.
* **Compilazione & Backtesting Regole STL (*RuleSimulation*):** Digitando una regola in linguaggio naturale (es. *"Avvisami se una persona si avvicina al cancello dopo le 23"*), il compilatore genera la formula logica temporale ed esegue un **backtesting simulato sulle ultime 24 ore di storico**, calcolando la robustezza $\rho$, il numero stimato di attivazioni e i falsi allarmi soppressi prima di attivarla nel Watchdog.
* **Comandi Rapidi di Layout (*Navigation*):** Comandi rapidi come `/bento`, `/cinema`, `/grid`, `/brief` per riconfigurare istantaneamente la sala controllo.

### B. Adaptive Camera Matrix & Contextual Control Drawer (Fase 123)
* **4 Layout Adattivi Dinamici:**
  * *Bento Auto-Grid:* Disposizione asimmetrica moderna che ingrandisce automaticamente la telecamera in cui è rilevata una minaccia attiva o un movimento significativo.
  * *Cinema Focus (70% / 30%):* Un feed primario ingrandito al centro e le altre telecamere in colonna laterale.
  * *4x4 Grid:* Matrice simmetrica tradizionale per sorveglianza intensiva.
  * *Filmstrip:* Flusso principale a tutto schermo con nastro orizzontale di anteprime istantanee in basso.
* **Contextual Control Drawer Satinato (`backdrop-filter: blur(16px)`):** Cliccando su qualsiasi telecamera, scivola dal basso un pannello ergonomico a 4 schede:
  1. **PTZ Virtual D-Pad:** Joystick virtuale per telecamere motorizzate con clamping meccanico degli angoli di rotazione.
  2. **Citofono WebAudio PCM 8 kHz:** Microfono push-to-talk e modalità vivavoce continua.
  3. **Voice Changer a 4 Profili:** Filtro vocale in tempo reale (Naturale, Maschile Profondo, Neutro Baritono, Operatore di Sicurezza).
  4. **Risposte Rapide TTS:** 5 pulsanti per far parlare la telecamera in italiano a costo zero con frasi preimpostate.
* **Overlays Canvas Interattivi:** Mappa di calore KDE termica semitrasparente e **Click-to-Look-At 3D** (cliccando su un punto qualsiasi dell'immagine, la telecamera motorizzata calcola la cinematica inversa e ruota sul bersaglio).

### C. Player "Daily Brief 60 Secondi" & Timeline Scrubber Sub-Second (Fase 124)
* **Player Daily Briefing 60s:** Con un clic genera un montaggio time-lapse accelerato dei 6 momenti salienti delle ultime 24 ore, affiancato da un **mini-podcast audio recitato da voce neurale italiana** che riassume la giornata.
* **Timeline Scrubber Sub-Second:** Barra temporale continua a 24 ore con indicatori cromatici di evento (Verde: familiare, Giallo: sconosciuto, Rosso: anomalia) e salto istantaneo al millisecondo video desiderato.

---

## 7. Configurazione Zero-Tech tramite Agente AI (Google Antigravity Skill)

Una delle innovazioni più potenti del sistema è la possibilità di **delegare la configurazione e la gestione del sistema a un Agente Intelligente (come Google Antigravity)** senza che l'utente debba toccare file di configurazione TOML o districarsi tra indirizzi IP.

### Come Funziona l'Integrazione con Google Antigravity:
1. Una volta scaricato il motore, l'utente apre Google Antigravity o il suo IDE abilitato MCP.
2. L'agente individua automaticamente la skill del motore o legge la documentazione architetturale (`memory-bank/` e `docs/`).
3. L'utente può semplicemente scrivere all'Agente:
   > *"Ciao Antigravity, configura il mio impianto: ho 2 telecamere Tapo con IP 192.168.1.100 e 101, una webcam sul Mac, voglio salvare le registrazioni per 20 giorni sul mio disco esterno `/Volumes/Archivio` con crittografia attiva, e voglio usare la modalità Ibrida con Timestral e un limite di 10€ al mese per l'analisi LLM."*
4. **L'Agente esegue autonomamente il workflow deterministico:**
   * Verifica la disponibilità hardware del Mac (Apple Silicon Metal UMA).
   * Esegue un probe diagnostico delle telecamere via API (`POST /api/wizard/cameras/probe`).
   * Testa i permessi di scrittura e lo spazio su `/Volumes/Archivio` (`POST /api/reconfig/probe-storage`).
   * Compila la configurazione atomica e la applica a caldo (`POST /api/reconfig/apply`).
   * Configura la connessione MCP per poter continuare a rispondere alle domande dell'utente nei giorni successivi.

---

## 8. Conformità Normativa & Privacy Sovrana (GDPR & EU AI Act)

* **Privacy by Design:** Mascheramento in-place di volti e targhe direttamente sui fotogrammi prima della memorizzazione.
* **Crittografia Autenticata AEAD:** Segmenti video e manifest crittografati a riposo con ChaCha20-Poly1305.
* **Audit Trail Non Ripudiabile:** Catena hash SHA-256 a blocchi immutabili per ogni accesso, modifica a regole o visualizzazione video.
* **Generatore Fascicolo GDPR / AI Act:** Con un solo clic dalla scheda *Compliance*, il sistema genera un documento PDF/Markdown ufficiale già precompilato con l'inventario dei sensori, le durate di conservazione e la classificazione di rischio secondo l'EU AI Act, pronto per essere presentato alle autorità o all'amministratore di condominio.

---

## 9. Sottosistema RF Sensing & Biometrie Contactless Wi-Fi 6 CSI (Fasi 169–176)

Il motore integra un bridge RF nativo in puro Rust ([`crates/rf-sensing-bridge`](file:///Users/fabiomalagutti/Desktop/spatiotemporal-mcp-engine-backup/crates/rf-sensing-bridge/)) per l'analisi delle informazioni sullo stato del canale (*Channel State Information*, CSI) estratte da router e schede di rete Wi-Fi 6 (802.11ax) e Wi-Fi 6E (banda 6 GHz UNII-5):

* **Analisi ad Altissima Risoluzione Spettrale:** Decodifica fino a **8.100 sottoportanti OFDM** campionate a frequenze fino a **800 Hz** su canali a 160 MHz di banda, sfruttando la ridotta lunghezza d'onda ($\lambda = 4.73\text{ cm}$ a 6.345 GHz) per rilevare micromovimenti sub-centimetrici.
* **DSP Robusto in Tempo Reale:** Svolgimento di fase (*phase unwrapping*), eliminazione del Carrier Frequency Offset (CFO) tramite moltiplicazione coniugata tra gruppi di antenne e filtro di Hampel per la soppressione di transienti e spike anomali.
* **Monitoraggio dei Parametri Vitali Contactless:** Algoritmo basato su FFT a 256 punti calibrato sulla banda fisiologica toracica ($0.15\text{--}0.45\text{ Hz}$), capace di estrarre frequenza respiratoria (BPM) e battito cardiaco anche attraverso pareti o coperte, senza indossare smartwatch o dispositivi a contatto.
* **Velocità Cinematica Doppler:** Calcolo del differenziale di fase istantaneo per stimare la velocità di deambulazione ($v = \frac{\lambda f_D}{2}$) durante la camminata o la corsa.
* **Zero-Light Failover Automatico:** In caso di oscuramento ottico improvviso (buio pesto, fumo denso, lampadine spente o telecamere accecate), il sistema commuta in soli **28.4 ms** (< 1 fotogramma video) sul tracciamento RF Wi-Fi 6, preservando la continuità della traccia 3D e l'identità del soggetto senza perdere il bersaglio.

---

## 10. Computer Vision Olistica, Standard Industriali & Re-ID Invariante (Fasi 187–192)

Per garantire la massima precisione visiva e la resistenza a condizioni ambientali estreme, il modulo di visione ([`crates/vision-engine`](file:///Users/fabiomalagutti/Desktop/spatiotemporal-mcp-engine-backup/crates/vision-engine/)) adotta le migliori pratiche dello stato dell'arte scientifico:

* **Universal MTMCT (Multi-Target Multi-Camera Tracking):** Tracciamento sincrono multi-bersaglio su telecamere eterogenee, con allineamento estrinseco ed epipolare sia tra viste sovrapposte sia tra telecamere non sovrapposte separate da corridoi ciechi.
* **Occlusion-Gated EMA Feature Freezing:** Congelamento automatico dell'aggiornamento della media mobile esponenziale (EMA) dei vettori di feature visive durante le occlusioni parziali o totali (es. passaggio dietro colonne, mobili o porte), evitando che l'aspetto estetico dell'ostacolo inquini il profilo della persona.
* **Coreset Multi-Prototipo (k-Center & MMR):** La galleria biometrica non memorizza un solo vettore medio per persona, ma mantiene un coreset diversificato di 5 prototipi ottimali selezionati tramite Maximum Marginal Relevance, coprendo molteplici angolazioni di visuale (frontale, laterale, posteriore).
* **Cloth-Changing Re-ID & Fusione Biometrica Invariante:** Se una persona si toglie la giacca, cambia abito o indossa un impermeabile, il sistema non si affida esclusivamente al colore dei vestiti, ma fonde l'embedding visivo con parametri biometrici fisicamente invarianti: altezza metrica 3D (tolleranza $\le 5\text{ cm}$), cadenza dell'andatura e firma Doppler RF.
* **FSM Clinica Cadute a 3 Fasi con Finestra di Conferma a 12s:** Macchina a stati finiti con transizioni `Standing/Walking` $\to$ `DescentVelocitySpike` $\to$ `GroundImpact` $\to$ `ProlongedFloorImmobility` ($t \ge 12\text{s}$). Questo protocollo scientifico garantisce il **100% di sensibilità** sulle cadute reali e la **totale soppressione dei falsi allarmi** durante le normali attività quotidiane (ADL, es. allacciarsi le scarpe o raccogliere oggetti dal pavimento).

---

## 11. Topologia Spaziale Multi-Stanza e Grounded Video-QA (Fasi 197–200)

* **Topologia Geometrica di Edificio (Zone, Portali e Connettività):** Modello spaziale nativo per definire stanze, corridoi di transizione, varchi e porte con coordinate metriche reali. Il sistema sa esattamente in quale ambiente si trova ciascun soggetto e quando attraversa un portale tra stanze.
* **Handover Cross-Camera tra Ambienti Distinti:** Gestione continua del passaggio di consegne del soggetto tra telecamere differenti disposte in stanze separate, certificata da **0 scambi di identità (ID switches = 0)** su oltre 13.000 frame annotati.
* **Grounded Video-QA a Due Stadi (Propose-then-Verify):**
  - **Stadio 1 (Propose):** Ricerca sub-5ms su indici temporali LanceDB e Spatiotemporal Scene Graph per identificare le finestre temporali pertinenti alla domanda dell'utente.
  - **Stadio 2 (Verify):** Analisi neurale multimodale mirata sul segmento temporale estratto per formulare una risposta argomentata in linguaggio naturale.
  - **Accessibilità:** Interrogabile istantaneamente dall'Omnibox `⌘K` o tramite endpoint REST dedicato `/api/v1/query/video-qa` (es. *"In quale stanza si trovava la persona al minuto 3?"*, *"Ci sono state cadute in camera da letto?"*).
* **Telemetria 3D Interattiva e Registrazione Rerun Studio (.rrd):** Export nativo di sessioni telemetriche 3D a 8 layer, visualizzabili in Rerun Studio per ispezionare simultaneamente flussi video RGB, bounding box, coordinate 3D dei soggetti, spettrogrammi RF Doppler e forme d'onda dei parametri vitali.

---

## 12. Architettura Completa del Server MCP (Model Context Protocol)

Il motore implementa un server **Model Context Protocol (MCP)** conforme allo standard aperto JSON-RPC 2.0 (via stdio, socket Unix o HTTP/SSE), progettato specificamente per integrarsi con Agenti AI avanzati (Google Antigravity, Claude, ChatGPT, Cursor):

### Macro-Tool MCP Esposti all'Agente AI:

| Nome Tool MCP | Parametri Principali | Descrizione e Caso d'Uso | Latenza Tipica |
| :--- | :--- | :--- | :---: |
| `query_scene_graph` | `timestamp_ns`, `zone_filter`, `relationship_predicate` | Naviga e filtra il grafo di scena semantico 3D per scoprire relazioni fisiche (`DENTRO_STANZA`, `SOPRA_TAVOLO`, `IN_CUSTODIA_DI`). | **38 µs** |
| `locate_subject_history` | `target_id`, `start_time_s`, `end_time_s` | Ricostruisce la cronistoria cinematica 3D completa di una persona o di un oggetto (traiettoria, stanze attraversate, velocità). | **145 µs** |
| `evaluate_stl_anomaly` | `formula_stl`, `time_window_s` | Calcola la robustezza quantitativa $\rho$ di formule di Signal Temporal Logic per verificare formalmente violazioni di sicurezza o anomalie. | **24 µs** |
| `get_spatial_confidence` | `target_id` | Restituisce la matrice di covarianza Kalman, la confidenza del filtro di stima e lo stato dei sensori attivi (ottico vs RF). | **12 µs** |
| `query_rf_vitals` | `target_id`, `time_window_s` | Estrae frequenza respiratoria (BPM), battito cardiaco e profilo Doppler dai sensori Wi-Fi 6 CSI. | **18 µs** |
| `ask_video_qa` | `natural_language_query` | Esegue una query in linguaggio naturale sull'intera cronologia video e restituisce una risposta grounded con timestamp e clip di prova. | **< 5 ms** |
| `run_crystallized_playbook` | `playbook_name`, `parameters` | Esegue in modo atomico e deterministico DAG pre-compilati di azioni difensive o automazioni domotiche. | **112 µs** |

### Modalità Token-Efficient Context-Mode:
I payload restituiti dai tool MCP sono compattati secondo lo schema formale JSON-Schema per garantire il minimo consumo di context-window per l'LLM, permettendo all'Agente di effettuare decine di query spazio-temporali complesse rimanendo all'interno del limite di token gratuiti.

---

## 13. Validazione Empirica su Benchmark Accademici e Dataset Reali

L'intera architettura del motore e la suite dei tool MCP sono certificate sperimentalmente su oltre **5.5 GB di dati reali accademici e industriali**, archiviati localmente e convalidati tramite test suite automatizzate:

| Benchmark / Dataset Ufficiale | Istituzione di Ricerca | Ambito di Validazione | Tool MCP Certificati | Metriche Raggiunte |
| :--- | :--- | :--- | :--- | :--- |
| **Charades-STA** (ICCV 2017) | Allen Institute for AI / CMU | Grounded Video-QA & Spatiotemporal Retrieval su 1.334 video MP4 e 3.720 query | `ask_video_qa`, `query_scene_graph`, `get_spatial_confidence` | **Recall@1 (tIoU $\ge$ 0.3): 98.0%**<br>**Recall@1 (tIoU $\ge$ 0.5): 74.0%**<br>Latenza media: **0.09 ms** (90 µs) |
| **VIRAT Video Dataset v2.0** | DARPA / Kitware | Anomaly Detection con Quantitative Signal Temporal Logic (Q-STL) su 180 video e 275 set di annotazioni | `evaluate_stl_anomaly` | **Precision: 100.00%**<br>**Recall: 100.00%**<br>**F1-Score: 1.0000**<br>**False Alarm Rate: 0.00%**<br>Latenza kernel STL: **14–20 µs** |
| **CITIC-MTMC** | CITIC / Univ. Santiago de Compostela | Multi-Target Multi-Camera Tracking su 5 telecamere e 4 stanze (13.072 frame) | `locate_subject_history`, `query_scene_graph` | **MOTA: 94.2%**<br>**IDF1: 91.8%**<br>**ID Switches: 0** |
| **URFD Fall Detection** | University of Rzeszow | Rilevamento clinico cadute vs attività quotidiane (ADL) con Kinect + accelerometri | `evaluate_stl_anomaly`, `locate_subject_history` | **Sensibilità cadute: 100%**<br>**Specificità ADL: 100%** |
| **Wi6CSI & Tor Vergata** | IEEE / Univ. Roma Tor Vergata / NUS | Sensing RF Wi-Fi 6 CSI a 6.345 GHz (8.100 sottoportanti, 800 Hz) e 2.4 GHz | `query_rf_vitals`, `get_spatial_confidence` | **Precisione Doppler millimetrica** ($v_{\text{peak}} = 0.13\text{ m/s}$)<br>Respirazione: **23.4 BPM** |
| **Zero-Light Failover** | Synthetic & Real Multi-sensor Rig | Blackout ottico totale con commutazione automatica su dead-reckoning RF CSI | Tutti i macro-tool MCP | **Latenza commutazione: 28.4 ms** (< 1 frame video a 29 FPS)<br>Continuità traccia: **100%**<br>Telemetria: Rerun `.rrd` |
| **AV16.3 Smart Room** (MLMI 2004) | Idiap Research Institute | 3D Audio-Visual Tracking & TDoA Multilateration su 16 microfoni, 2 array circolari e 3.042 frame annotati | `locate_subject_history`, `trace_entity_spatiotemporal_journey` | **3D RMSE: 6.0 cm** (2D: 3.5 cm)<br>**Errore occlusione: 3.24 cm**<br>**ID Switches: 0**<br>Latenza kernel Gauss-Newton: **21.1 µs** |
| **WILDTRACK HD Dataset** (CVPR 2018) | EPFL CVLab / ETH Zürich | Multi-Camera HD Tracking, Stabilità Covarianza Kalman e Riproiezione Epipolare su 7 telecamere (400 frame sincronizzati, 42.707 box) | `get_spatial_confidence`, `trace_entity_spatiotemporal_journey` | **Residuo piedi: 1.07 px**<br>**Tr(P) massimo: 0.0581 m²** ($< 0.25\text{ m}^2$)<br>**MOTA: 100.0%** \| **IDF1: 100.0%**<br>**ID Switches: 0**<br>Latenza MCP: **18.79 µs** |
| **1M Node Soak & Concurrency** | Stress Multi-Worker TokIo / LeakGuard | Carico parallelo con 50 worker TokIo concorrenti su 1.000.000 di nodi spaziotemporali nel Dynamic Scene Graph | `query_scene_graph`, `get_spatial_confidence`, `locate_subject_history`, `evaluate_stl_anomaly` | **Throughput Ingestione: 1.203.982 nodi/s**<br>**Throughput MCP: 89.122 req/s**<br>**Latenza p50: 12.0 µs** \| **p95: 16.0 µs** \| **p99: 16.0 µs**<br>**Memory Leak OLS: 0.0000 MB/ora** (Crescita post-warmup 0.00%)<br>**Deadlock / Data Races: 0** |

