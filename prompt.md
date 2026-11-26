# Prompt: Minecraft SeedFinder (lokal, Desktop, Chunkbase-ähnlich)

Du bist ein erfahrener Rust- und Tauri-Entwickler. Setze das folgende Projekt **schrittweise** um. Diese Datei ist die vollständige Spezifikation; behandle sie als verbindlich. Beginne mit **Meilenstein M0** und melde dich danach mit Ergebnissen, Benchmarks und offenen Punkten – springe nicht direkt zur fertigen App.

---

## 1. Ziel

Eine **lokal laufende Desktop-App** zum Finden von Minecraft-Java-Seeds, ähnlich Chunkbase, aber mit automatischer Suche. Der Nutzer wählt eine Minecraft-Version, formuliert eigene Regeln (z. B. „Insel am Spawn", „Ruined Portal auf einem Berg") und die App durchsucht Millionen von Seeds parallel und zeigt Treffer mit Vorschau.

**Keine Cloud, kein Minecraft nötig, keine Internetverbindung zur Laufzeit.**

---

## 2. Feste Entscheidungen (nicht neu verhandeln)

| Punkt | Entscheidung |
|---|---|
| Edition | **Nur Java**, alle von Cubiomes unterstützten Hauptversionen (ca. 1.6.4 – 1.21.x), auswählbar |
| Nicht unterstützt | Alpha/Beta/1.0–1.5 (eigene Engine, spätere Phase) |
| App-Form | **Tauri 2 Desktop-App** (Windows, macOS inkl. Apple Silicon, Linux) |
| Stack | Rust-Backend + React/TypeScript-Frontend (Vite) |
| Engine | `cubiomes`-Rust-Crate (docs.rs/cubiomes); Fallback: C-Sourcen via `cc`-Crate kompilieren |
| Seed-Eingabe | Zufällige 64-bit-Seeds durchsuchen (kein fester Bereich nötig) |
| UI-Sprache | Deutsch als Default, Englisch optional |
| Portal-Bedeutung | „Portal" = generiertes **Ruined Portal** als Anker; „freier Portalplatz" später |
| Vorschau-Umfang | Thumbnails in der Trefferliste + große Detailkarte + 3D-Iso-Relief + PNG-Export |
| Vorschau-Timing | Thumbnails für die **ersten 20 Treffer automatisch**, alle weiteren erst auf Klick |

---

## 3. Architektur

```
SeedFinder/
├─ src-tauri/                     # Rust-Backend
│  ├─ src/
│  │  ├─ main.rs / lib.rs         # Tauri-Setup, Commands
│  │  ├─ engine/
│  │  │  ├─ mod.rs
│  │  │  ├─ generator.rs          # Cubiomes-Wrapper (thread-lokaler State)
│  │  │  ├─ seed_iter.rs          # 64-bit Seed-Iterator, Worker-Pool
│  │  │  ├─ rules.rs              # Regelbaum/DAG, Auswertung, Kosten
│  │  │  ├─ anchors.rs            # Anker-Auflösung + Memoization
│  │  │  └─ version.rs            # VersionProfile, Biome-/Struktur-Katalog, Validierung
│  │  ├─ preview/
│  │  │  ├─ grid.rs               # Biom- + Höhenraster (einmal pro Seed)
│  │  │  ├─ render.rs             # Thumbnail, Karte, Iso, PNG
│  │  │  └─ cache.rs
│  │  ├─ store.rs                 # Presets, Favoriten (JSON-Datei oder SQLite)
│  │  └─ events.rs                # Progress/Treffer-Events an Frontend
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ src/                           # React-Frontend
│  ├─ components/
│  │  ├─ VersionPicker.tsx
│  │  ├─ RuleBuilder.tsx
│  │  ├─ SearchDashboard.tsx
│  │  ├─ ResultList.tsx           # inkl. Thumbnails
│  │  ├─ MapView.tsx              # Canvas: Biome, Hillshade, Strukturen, Spawn
│  │  ├─ IsoView.tsx              # 3D-Relief
│  │  └─ Collection.tsx
│  ├─ types.ts                    # Preset/Regel-Typen (spiegelt Rust-Serde)
│  └─ App.tsx
├─ package.json
└─ README.md
```

---

## 4. Verifizierte Cubiomes-Fakten (bitte nutzen)

Aus `generator.h`, `biomenoise.h`, `finders.h` und README der aktuellen Cubiomes-Version:

- `setupGenerator(&g, MC_VERSION, flags)` – Version pro Kandidat setzen; `applySeed(&g, DIM_OVERWORLD, seed)`; Seeds intern `uint64_t`.
- `getBiomeAt(&g, scale, x, y, z)` – `scale = 1` (Block) oder `4` (Biom-Raster).
- `genBiomes(&g, cache, Range)` + `allocCache()` – Flächenhaftes Biomraster (1:4).
- `getSpawn(&g)` – Spawnpunkt versionsabhängig korrekt (≤1.17 anderes Verfahren als ≥1.18).
- `getStructurePos(type, mc, lower48, rx, rz, &pos)`, `isViableStructurePos(...)`, `getStructureConfig(...)`; Strukturen hängen nur von den unteren 48 Bit ab → Optimierungsmöglichkeit.
- `initFirstStronghold(&it, mc, seed)` / `nextStronghold(&it, &g)` – Strongholds.
- `initSurfaceNoise(&sn, dim, seed)` + `mapApproxHeight(float *y, int *ids, &g, &sn, x, z, w, h)` – **approximierte Oberflächenhöhe im 1:4-Raster, versionsübergreifend**.
- Hilfsfunktionen aus `util.h`: `initBiomeColors()`, `biomesToImage()`, `savePPM()` – für Preview nutzbar.
- **Wichtig:** `mapApproxHeight` ist eine Näherung (besonders ab 1.18). Höhenangaben daher mit Toleranz auswerten und in der UI als „ca." kennzeichnen. Für exakte Berg-Erkennung die **Biom-Kategorie** `mountain` verwenden.

Jeder Worker-Thread benötigt einen **eigenen** Generator-/Noise-State; Cubiomes ist nur bei getrennten States thread-safe.

---

## 5. Such-Engine

1. Zufällige 64-bit-Seeds erzeugen (Thread-Local RNG, z. B. Xoshiro).
2. Pro Seed: `applySeed` → Anker auflösen → Regeln in **Kostenreihenfolge** prüfen → Early-Out beim ersten Fehlschlag.
3. Worker-Pool über `rayon` oder `std::thread` (Thread-Anzahl einstellbar, Default = CPU-Kerne).
4. Progress-Events über Tauri-Channel: geprüfte Seeds, Treffer, seeds/s, ETA,_status (running/paused/done). Pause/Stop ohne Verlust bereits gefundener Treffer.
5. Pro Seed berechnete Werte cachen (Spawn, Biom am Spawn, Strukturpositionen, Höhenraster), damit kombinierte Regeln nichts doppelt rechnen.

Kosten-Reihenfolge (billig → teuer): `spawnBiome` (~µs) → Struktur-Lookup → `biomeNear` → `island`/`mountain`/Höhe.

---

## 6. Regelsystem

Drei Bausteine, verschachtelbar mit `all` / `any` / `not`:

**Anker (WO):**
- `spawn`
- `structure(type, n)` – z. B. 1. `ruined_portal`
- `nearestBiome(biome, from, maxDist)`
- `coord(x, z)`

**Prädikate (WAS am Ort):**
- `island(radius, minWater)` – Wasseranteil (Ozean-/Strandbiome) im Radius
- `mountain` – Biom-Kategorie Gebirge
- `biome(biome|category, anyOf/allOf)`
- `height(min/max)` – **approx**, mit Toleranz
- `waterRatio(min/max)`
- `nearStructure(type, minDist/maxDist)`

**Relationen (WIE zueinander):**
- `distance(a, b, min/max)`
- `at(anker): prädikat`
- `height(a) > height(b)`, `count(structure, radius)`

Der Regelbaum ist ein **Abhängigkeitsgraph (DAG)**: Anker zuerst (einmal berechnen, memoizen), dann Prädikate an Ankern, dann Relationen. Die Auswertung darf keine Anker mehrfach berechnen.

### Beispiel-Presets

„Insel am Spawn":
```json
{ "all": [ { "at": "spawn", "check": { "island": { "radius": 500, "minWater": 0.7 } } } ] }
```

„Portal auf Berg":
```json
{
  "all": [
    { "at": "spawn", "check": { "island": { "radius": 500, "minWater": 0.7 } } },
    { "let": "portal", "be": {
        "nearest": { "structure": "ruined_portal", "from": "spawn", "maxDist": 2000 }
    } },
    { "at": "portal", "check": { "all": [
        { "biomeCategory": "mountain" },
        { "height": { "min": 100, "approx": true, "tolerance": 10 } }
    ] } }
  ]
}
```

„Dorf am Fuß eines Berges":
```json
{
  "all": [
    { "let": "v", "be": { "nearest": { "structure": "village", "from": "spawn", "maxDist": 1500 } } },
    { "at": "v", "check": { "height": { "max": 75, "approx": true } } },
    { "distance": { "from": "v", "to": { "nearestBiome": "mountain" }, "max": 300 } }
  ]
}
```

### Versionsvalidierung (Pflicht)

- Jede Version hat ein `VersionProfile`: Cubiomes-Enum, gültige Biome-IDs, gültige Strukturtypen, Spawn-Regel-Variante.
- Der Regel-Editor **validiert gegen die gewählte Version**: nicht vorhandene Biome/Strukturen (z. B. `ancient_city` vor 1.19, `pale_garden` vor 1.21.3) werden abgelehnt bzw. ausgegraut und mit Hinweis versehen.
- Regeln, die nur approximiert funktionieren, werden als „experimentell/ca." markiert.

**Empfohlene Standard-Presets:** Insel am Spawn, Portal auf Berg, Dorf am Spawn, Bergbiome im Umkreis, All-Biome-Challenge (möglichst viele Biome in 2000 Blöcken).

---

## 7. Vorschau-Pipeline

Pro Treffer wird **ein** Raster erzeugt und für alle Ausgaben wiederverwendet:

- Gebiet: 1024×1024 Blöcke um den Spawn → 256×256 Zellen bei 1:4.
- `genBiomes()` → Biom-IDs (Farben via `initBiomeColors`/`biomesToImage`).
- `initSurfaceNoise()` + `mapApproxHeight()` → Höhen (Hillshading + Iso).
- `getStructurePos()` → Struktur-Icons, Spawn-Marker.

Aus diesen Daten:
| Ausgabe | Anforderung |
|---|---|
| Thumbnail | 128×128, Biomfarben + leichte Schattierung + Spawn-Punkt; erste 20 Treffer automatisch |
| Detailkarte | Zoom/Pan, Biom-Legende, Struktur-Icons, Koordinaten, Rule-Checkliste mit gemessenen Werten |
| 3D-Iso | Relief aus Höhengrid, leicht rotierbar |
| PNG-Export | Thumbnail, Karte oder Iso als Datei |

Cache pro Seed teilen sich Rule-Engine und Preview. Das Öffnen eines Treffers muss dadurch praktisch instant sein.

**Rule-Checkliste:** Für jeden Treffer anzeigen, welche Regeln erfüllt sind, mit konkreten Werten („Wasseranteil 78 % ✓", „Dorf 640 Blöcke entfernt ✓", „Portal ca. Höhe 112 ✓").

---

## 8. UI (Tauri-Fenster)

Drei Hauptbereiche:

1. **Suche:** Versions-Picker (gruppiert: 1.6–1.12 / 1.13–1.17 / 1.18+), Regel-Builder (verschachtelbare Regel-Karten, Presets laden/speichern), Start/Pause/Stop, Fortschrittsbalken mit seeds/s + ETA, Trefferliste mit Score (Default: Anzahl erfüllter Regeln; später gewichtbar), Sortierung.
2. **Karte:** Detailkarte + 3D-Iso-Umschalter, Overlays, PNG-Export, „Seed kopieren".
3. **Sammlung:** Gemerkte Seeds mit Notizen, Tags, Versions- und Regel-Snapshot; Import/Export JSON/CSV.

Einstellungen: Thread-Anzahl, Seed-Rate-Limit, Sprache DE/EN, Theme.

---

## 9. Meilensteine & Acceptance Criteria

- **M0 – Tech-Spike:** Rust-Projekt + Tauri-Shell; `cubiomes`-Crate eingebunden; Test, der für einen bekannten Seed (z. B. `3055141959546`) bei 1.21.4 und 1.16.5 Spawn plus `mapApproxHeight`-Raster ausgibt; Benchmark seeds/s. **Done wenn:** `cargo test` grün, Zahlen plausibel, Benchmark dokumentiert.
- **M1 – Engine-Kern:** Seed-Iterator, `spawnBiome`, `biomeNear`, JSON-Output, Golden-Seed-Tests pro Version. **Done wenn:** bekannte Seeds liefern erwartete Biome.
- **M2 – Regeln & Suche:** DAG, alle Tier-1-Prädikate/Relationen, Anker-Memoization, Version-Adapter + Validierung, Worker-Pool, Progress-Events, Presets. **Done wenn:** die drei Beispiel-Presets laufen und Early-Out messbar greift.
- **M3 – UI:** Version-Picker, Regel-Builder, Trefferliste mit Thumbnails, Detailkarte. **Done wenn:** Suche startbar, Treffer klickbar, Karte korrekt.
- **M4 – Vorschau fertig:** 3D-Iso, PNG-Export, Rule-Checkliste, Favoriten/Notizen/Export, DE/EN, Installer pro OS. **Done wenn:** App ohne Internet nutzbar, Installer gebaut.
- **M5 – optional:** freier Portalplatz als Anker, präzisere Höhenregeln, Beta/1.0–1.5.

---

## 10. Stolpersteine / Qualitätsanforderungen

- Spawn-Logik vor/ab 1.18 unterscheidet sich → versionengenau umsetzen und testen.
- Seed als `int64` in der UI anzeigen, intern `uint64` – Vorzeichen korrekt behandeln.
- Keine blockierenden Operationen im Tauri-UI-Thread; Suche immer im Hintergrund.
- Approximierte Höhen nie als exakt ausgeben; Toleranz + Kennzeichnung.
- Tests: Golden Seeds pro Version, Versionsvalidierung, DAG-Auswertung, Rendering-Snapshots.
- Code in Rust idiomatisch (`Result`, keine `unwrap()` in Produktionspfaden), Frontend mit TypeScript strict.
- Keine externen API-Calls zur Laufzeit; alle Assets lokal.

---

## 11. Erster Schritt für dich

Setze **M0** um:

1. Ordnerstruktur wie oben anlegen, `Cargo.toml`, `tauri.conf.json`, `package.json` minimal.
2. `cubiomes`-Crate einbinden (Alternativ: `cc`-Crate mit den C-Sourcen, falls die Crate nicht verfügbar/kompatibel ist).
3. Rust-Test schreiben: Generator für `MC_1_21_4` und `MC_1_16_5` auf Seed `3055141959546` anwenden, `getSpawn()`, `getBiomeAt(4, ...)` und ein 32×32-`mapApproxHeight`-Raster ausgeben; Werte gegen bekannte Referenzen prüfen (falls keine Referenz vorhanden: Plausibilität + Versionen vergleichen und dokumentieren).
4. Kurzen Benchmark (z. B. 100.000 Seeds, `spawnBiome`) mit seeds/s in die README.

Danach **stoppen und berichten**: was funktioniert, welche Versionen die Crate tatsächlich supportet, gemessene Performance, Blockierte Punkte. Erst nach Freigabe mit M1 weitermachen.
