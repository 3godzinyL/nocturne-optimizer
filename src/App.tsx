import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  AppWindow,
  Cog,
  Gauge,
  HardDrive,
  KeyRound,
  Power,
  Rocket,
  Shield,
  Sparkles
} from "lucide-react";
import { Sidebar, type NavItem } from "./components/Sidebar";
import { api } from "./lib/tauri";
import type {
  AutostartItem,
  OptimizationRule,
  RegistryHealthItem,
  SecurityConfig,
  SecurityRuntime,
  SettingsState,
  SystemSnapshot
} from "./types";

const navItems: NavItem[] = [
  { id: "overview", label: "01. Przegląd", icon: Gauge },
  { id: "optimization", label: "02. Live optymalizacja", icon: Rocket },
  { id: "autostart", label: "03. Autostart", icon: Power },
  { id: "offline", label: "04. Offline optymalizacja", icon: HardDrive },
  { id: "registry", label: "05. Sprawność reg", icon: KeyRound },
  { id: "security", label: "06. Bezpieczeństwo", icon: Shield },
  { id: "settings", label: "07. Ustawienia", icon: Cog }
];

const popularChoices = ["discord.exe", "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "telegram.exe", "steam.exe"];

function fmtGb(current: number, total: number) {
  return `${current.toFixed(1)} / ${total.toFixed(1)} GB`;
}

export default function App() {
  const [active, setActive] = useState("overview");
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [rules, setRules] = useState<OptimizationRule[]>([]);
  const [autostart, setAutostart] = useState<AutostartItem[]>([]);
  const [registry, setRegistry] = useState<RegistryHealthItem[]>([]);
  const [securityConfig, setSecurityConfig] = useState<SecurityConfig>({ passwordSet: false, fileProtection: false, protectedApps: [], lockEnabled: false });
  const [securityRuntime, setSecurityRuntime] = useState<SecurityRuntime>({ locked: false, presentPopularApps: [] });
  const [settings, setSettings] = useState<SettingsState>({ refreshMs: 1500, autoApplyRules: true, aggressiveMode: false, minimizeToTray: true });
  const [draftProcess, setDraftProcess] = useState("chrome.exe");
  const [draftMode, setDraftMode] = useState<OptimizationRule["mode"]>("Balanced");
  const [draftPassword, setDraftPassword] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [info, setInfo] = useState("Silnik działa w trybie ciągłego monitoringu.");

  const processOptions = useMemo(() => {
    const fromRuntime = securityRuntime.presentPopularApps;
    const fromSnapshot = (snapshot?.processes ?? []).map((p) => p.name.toLowerCase());
    return Array.from(new Set([...popularChoices, ...fromRuntime, ...fromSnapshot])).sort();
  }, [securityRuntime.presentPopularApps, snapshot?.processes]);

  const refreshAll = async () => {
    const [snap, ruleRows, runtime] = await Promise.all([api.getSystemSnapshot(), api.getRules(), api.getSecurityRuntime()]);
    setSnapshot(snap);
    setRules(ruleRows);
    setSecurityRuntime(runtime);
    if (runtime.locked) window.dispatchEvent(new Event("nocturne:open-guard"));
  };

  useEffect(() => {
    Promise.all([
      refreshAll(),
      api.listAutostartItems().then(setAutostart),
      api.getRegistryHealth().then(setRegistry),
      api.getSecurityConfig().then(setSecurityConfig),
      api.getSettings().then(setSettings)
    ]).catch((error) => setInfo(String(error)));
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      refreshAll().catch(() => undefined);
    }, settings.refreshMs || 1500);
    return () => window.clearInterval(timer);
  }, [settings.refreshMs]);

  const addRule = async () => {
    const next: OptimizationRule[] = [
      ...rules,
      {
        id: crypto.randomUUID(),
        processName: draftProcess,
        mode: draftMode,
        requireBackground: true,
        autoResume: true,
        enabled: true
      }
    ];
    setRules(next);
    await api.saveRules(next);
    setInfo(`Dodano regułę dla ${draftProcess} (${draftMode}).`);
  };

  const toggleRule = async (id: string) => {
    const next = rules.map((rule) => (rule.id === id ? { ...rule, enabled: !rule.enabled } : rule));
    setRules(next);
    await api.saveRules(next);
  };

  const removeRule = async (id: string) => {
    const next = rules.filter((rule) => rule.id !== id);
    setRules(next);
    await api.saveRules(next);
  };

  const refreshAutostart = async () => setAutostart(await api.listAutostartItems());
  const toggleAutostart = async (item: AutostartItem) => {
    setBusy(item.id);
    await api.toggleAutostartItem(item, !item.enabled);
    await refreshAutostart();
    setBusy(null);
  };

  const runPreset = async (presetId: string) => {
    setBusy(presetId);
    const result = await api.runOfflinePreset(presetId);
    setInfo(result.details);
    setBusy(null);
  };

  const saveSecurity = async () => {
    await api.saveSecurityConfig({
      password: draftPassword || undefined,
      fileProtection: securityConfig.fileProtection,
      protectedApps: securityConfig.protectedApps,
      lockEnabled: securityConfig.lockEnabled
    });
    setDraftPassword("");
    const fresh = await api.getSecurityConfig();
    setSecurityConfig(fresh);
    setInfo("Zapisano konfigurację bezpieczeństwa.");
  };

  const saveAppSettings = async () => {
    await api.saveSettings(settings);
    setInfo("Zapisano ustawienia silnika.");
  };

  const topProcesses = (snapshot?.processes ?? []).slice(0, 8);

  return (
    <div className="shell-layout">
      <Sidebar items={navItems} active={active} onSelect={setActive} />
      <main className="content-area">
        <header className="topbar shell-card">
          <div>
            <div className="eyebrow">ultra dark dashboard</div>
            <h2>{navItems.find((item) => item.id === active)?.label}</h2>
          </div>
          <div className="topbar-meta">
            <Sparkles size={16} />
            {info}
          </div>
        </header>

        {active === "overview" && (
          <section className="page-grid">
            <div className="hero shell-card glowing">
              <div className="hero-copy">
                <div className="eyebrow">live machine pulse</div>
                <h3>Obecne zużycie komponentów</h3>
                <p>
                  Front cały czas odświeża dane z silnika. Na górze masz puls systemu, niżej top procesy i status procesu.
                </p>
              </div>
              <div className="hero-badges">
                <div className="stat-chip"><Activity size={14} /> CPU {snapshot?.cpuUsage.toFixed(1) ?? "0.0"}%</div>
                <div className="stat-chip"><HardDrive size={14} /> RAM {snapshot ? fmtGb(snapshot.ramUsedGb, snapshot.ramTotalGb) : "0 / 0 GB"}</div>
                <div className="stat-chip"><AppWindow size={14} /> Procesów {snapshot?.processes.length ?? 0}</div>
              </div>
            </div>
            <div className="stats-grid">
              <StatCard title="CPU" value={`${snapshot?.cpuUsage.toFixed(1) ?? "0.0"}%`} sub="średnie użycie" />
              <StatCard title="RAM" value={snapshot ? fmtGb(snapshot.ramUsedGb, snapshot.ramTotalGb) : "0 / 0 GB"} sub="pamięć operacyjna" />
              <StatCard title="Swap" value={snapshot ? fmtGb(snapshot.swapUsedGb, snapshot.swapTotalGb) : "0 / 0 GB"} sub="przestrzeń wymiany" />
              <StatCard title="Uptime" value={`${Math.floor((snapshot?.uptimeSeconds ?? 0) / 3600)}h`} sub="od startu systemu" />
            </div>
            <div className="shell-card table-card wide">
              <div className="section-head">
                <div>
                  <div className="eyebrow">top resource pressure</div>
                  <h3>Top procesy</h3>
                </div>
              </div>
              <table className="data-table">
                <thead><tr><th>Proces</th><th>PID</th><th>CPU</th><th>RAM</th><th>Stan</th></tr></thead>
                <tbody>
                  {topProcesses.map((proc) => (
                    <tr key={proc.pid}>
                      <td>
                        <div className="row-title">{proc.name}</div>
                        <div className="row-sub">{proc.exe}</div>
                      </td>
                      <td>{proc.pid}</td>
                      <td>{proc.cpu.toFixed(1)}%</td>
                      <td>{proc.memoryMb.toFixed(0)} MB</td>
                      <td><span className={`status-pill ${proc.foreground ? "is-live" : proc.optimizedState !== "Normal" ? "is-optimized" : ""}`}>{proc.optimizedState}</span></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}

        {active === "optimization" && (
          <section className="page-grid">
            <div className="shell-card wide">
              <div className="section-head">
                <div>
                  <div className="eyebrow">rule engine</div>
                  <h3>Live optymalizacja</h3>
                </div>
              </div>
              <div className="rule-builder">
                <div className="input-group">
                  <label>Aplikacja</label>
                  <select value={draftProcess} onChange={(e) => setDraftProcess(e.target.value)}>
                    {processOptions.map((name) => <option key={name} value={name}>{name}</option>)}
                  </select>
                </div>
                <div className="input-group">
                  <label>Stopień ograniczenia</label>
                  <select value={draftMode} onChange={(e) => setDraftMode(e.target.value as OptimizationRule["mode"])}>
                    <option value="Eco">Eco</option>
                    <option value="Balanced">Balanced</option>
                    <option value="Freeze">Freeze</option>
                  </select>
                </div>
                <label className="check-line"><input type="checkbox" checked readOnly /> tylko gdy proces nie jest na aktywnym oknie</label>
                <label className="check-line"><input type="checkbox" checked readOnly /> auto resume po wejściu na okno</label>
                <button className="primary-button" onClick={addRule}>Dodaj regułę</button>
              </div>
              <div className="rule-list">
                {rules.map((rule) => (
                  <div key={rule.id} className="rule-card">
                    <div>
                      <div className="row-title">{rule.processName}</div>
                      <div className="row-sub">{rule.mode} / background only / autoresume</div>
                    </div>
                    <div className="rule-actions">
                      <button className="ghost-button" onClick={() => toggleRule(rule.id)}>{rule.enabled ? "Wyłącz" : "Włącz"}</button>
                      <button className="ghost-button danger" onClick={() => removeRule(rule.id)}>Usuń</button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            <div className="shell-card wide table-card">
              <div className="section-head">
                <div>
                  <div className="eyebrow">process visibility + rule state</div>
                  <h3>Tabela procesów</h3>
                </div>
              </div>
              <table className="data-table">
                <thead><tr><th>Proces</th><th>CPU</th><th>RAM</th><th>Widoczność</th><th>Stan</th><th>Optymalizacja</th></tr></thead>
                <tbody>
                  {(snapshot?.processes ?? []).map((proc) => (
                    <tr key={proc.pid}>
                      <td><div className="row-title">{proc.name}</div><div className="row-sub">PID {proc.pid}</div></td>
                      <td>{proc.cpu.toFixed(1)}%</td>
                      <td>{proc.memoryMb.toFixed(0)} MB</td>
                      <td>{proc.foreground ? "Aktywny" : "W tle"}</td>
                      <td>{proc.status}</td>
                      <td><span className={`status-pill ${proc.optimizedState !== "Normal" ? "is-optimized" : ""}`}>{proc.optimizedState}</span></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}

        {active === "autostart" && (
          <section className="page-grid">
            <div className="shell-card wide table-card">
              <div className="section-head">
                <div>
                  <div className="eyebrow">registry + startup folders + tasks + services</div>
                  <h3>Autostart</h3>
                </div>
                <button className="ghost-button" onClick={refreshAutostart}>Odśwież</button>
              </div>
              <table className="data-table">
                <thead><tr><th>Nazwa</th><th>Źródło</th><th>Typ</th><th>Ścieżka / komenda</th><th>Stan</th><th>Akcja</th></tr></thead>
                <tbody>
                  {autostart.map((item) => (
                    <tr key={item.id}>
                      <td><div className="row-title">{item.name}</div><div className="row-sub">{item.details}</div></td>
                      <td>{item.source}</td>
                      <td>{item.itemType}</td>
                      <td className="path-cell">{item.path}</td>
                      <td><span className={`status-pill ${item.enabled ? "is-live" : ""}`}>{item.enabled ? "Włączony" : "Wyłączony"}</span></td>
                      <td><button className="ghost-button" disabled={busy === item.id} onClick={() => toggleAutostart(item)}>{item.enabled ? "Wyłącz" : "Włącz"}</button></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}

        {active === "offline" && (
          <section className="page-grid cards-3">
            <PresetCard
              title="Temp clean & cache sweep"
              description="Czyści tempy użytkownika i systemowe śmieci, żeby szybko zrzucić trochę syfu z dysku i odzyskać responsywność."
              action={() => runPreset("clean_temp")}
              busy={busy === "clean_temp"}
            />
            <PresetCard
              title="Quiet background mode"
              description="Przycina kilka typowych usług tła i restartuje explorer, kiedy chcesz wyciszyć system do pracy lub grania."
              action={() => runPreset("background_quiet")}
              busy={busy === "background_quiet"}
            />
            <PresetCard
              title="Debloat lite"
              description="Wyłącza część mniej potrzebnych funkcji konsumenckich Windowsa i proponuje lżejszy profil codziennej pracy."
              action={() => runPreset("debloat_lite")}
              busy={busy === "debloat_lite"}
            />
          </section>
        )}

        {active === "registry" && (
          <section className="page-grid">
            <div className="shell-card wide table-card">
              <div className="section-head">
                <div>
                  <div className="eyebrow">uac / smartscreen / lsa / elevation</div>
                  <h3>Sprawność kluczy rejestru</h3>
                </div>
                <button className="ghost-button" onClick={() => api.getRegistryHealth().then(setRegistry)}>Przeskanuj</button>
              </div>
              <table className="data-table">
                <thead><tr><th>Klucz</th><th>Obecnie</th><th>Zalecane</th><th>Ocena</th><th>Opis</th></tr></thead>
                <tbody>
                  {registry.map((row, index) => (
                    <tr key={`${row.keyPath}-${index}`}>
                      <td><div className="row-title">{row.valueName}</div><div className="row-sub">{row.keyPath}</div></td>
                      <td>{row.current}</td>
                      <td>{row.recommended}</td>
                      <td><span className={`status-pill ${row.healthy ? "is-live" : "danger"}`}>{row.healthy ? "OK" : "Uwaga"}</span></td>
                      <td>{row.meaning}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}

        {active === "security" && (
          <section className="page-grid">
            <div className="shell-card wide">
              <div className="section-head">
                <div>
                  <div className="eyebrow">app reopen guard</div>
                  <h3>Zabezpieczenie aplikacji</h3>
                </div>
              </div>
              <div className="security-grid">
                <div className="security-pane">
                  <label className="check-line"><input type="checkbox" checked={securityConfig.lockEnabled} onChange={(e) => setSecurityConfig({ ...securityConfig, lockEnabled: e.target.checked })} /> blokada po powrocie okna na pierwszy plan</label>
                  <label className="check-line"><input type="checkbox" checked={securityConfig.fileProtection} onChange={(e) => setSecurityConfig({ ...securityConfig, fileProtection: e.target.checked })} /> zabezpieczenie danych w pliku (tryb vault)</label>
                  <label className="input-group">
                    <span>Nowe hasło</span>
                    <input type="password" value={draftPassword} onChange={(e) => setDraftPassword(e.target.value)} placeholder={securityConfig.passwordSet ? "ustaw nowe hasło" : "ustaw hasło"} />
                  </label>
                  <button className="primary-button" onClick={saveSecurity}>Zapisz zabezpieczenia</button>
                </div>
                <div className="security-pane">
                  <div className="row-title">Popularne wykryte aplikacje</div>
                  <div className="app-tags">
                    {processOptions.map((name) => {
                      const activeTag = securityConfig.protectedApps.includes(name);
                      return (
                        <button
                          key={name}
                          className={`tag-button ${activeTag ? "active" : ""}`}
                          onClick={() => {
                            const next = activeTag ? securityConfig.protectedApps.filter((app) => app !== name) : [...securityConfig.protectedApps, name];
                            setSecurityConfig({ ...securityConfig, protectedApps: next });
                          }}
                        >
                          {name}
                        </button>
                      );
                    })}
                  </div>
                  <div className="row-sub">Aktywna blokada: {securityRuntime.locked ? `TAK / ${securityRuntime.lockedApp}` : "nie"}</div>
                </div>
              </div>
            </div>
          </section>
        )}

        {active === "settings" && (
          <section className="page-grid cards-2">
            <div className="shell-card">
              <div className="section-head"><div><div className="eyebrow">engine cadence</div><h3>Silnik i polling</h3></div></div>
              <label className="input-group"><span>Interwał odświeżania (ms)</span><input type="number" value={settings.refreshMs} onChange={(e) => setSettings({ ...settings, refreshMs: Number(e.target.value) })} /></label>
              <label className="check-line"><input type="checkbox" checked={settings.autoApplyRules} onChange={(e) => setSettings({ ...settings, autoApplyRules: e.target.checked })} /> automatycznie nakładaj reguły</label>
              <label className="check-line"><input type="checkbox" checked={settings.aggressiveMode} onChange={(e) => setSettings({ ...settings, aggressiveMode: e.target.checked })} /> agresywny monitoring</label>
              <label className="check-line"><input type="checkbox" checked={settings.minimizeToTray} onChange={(e) => setSettings({ ...settings, minimizeToTray: e.target.checked })} /> minimalizuj do tray</label>
              <button className="primary-button" onClick={saveAppSettings}>Zapisz ustawienia</button>
            </div>
            <div className="shell-card">
              <div className="section-head"><div><div className="eyebrow">notes</div><h3>Uwagi</h3></div></div>
              <ul className="notes-list">
                <li>Freeze używa zawieszania procesu, więc część aplikacji może po wznowieniu chwilę dochodzić do siebie.</li>
                <li>Autostart i część presetów może wymagać uruchomienia aplikacji jako administrator.</li>
                <li>Overlay bezpieczeństwa działa jako osobne okno Tauri nad pulpitem.</li>
              </ul>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}

function StatCard({ title, value, sub }: { title: string; value: string; sub: string }) {
  return (
    <div className="shell-card stat-card">
      <div className="eyebrow">{title}</div>
      <div className="stat-value">{value}</div>
      <div className="row-sub">{sub}</div>
    </div>
  );
}

function PresetCard({ title, description, action, busy }: { title: string; description: string; action: () => void; busy: boolean }) {
  return (
    <div className="shell-card preset-card glowing">
      <div className="eyebrow">offline preset</div>
      <h3>{title}</h3>
      <p>{description}</p>
      <button className="primary-button" disabled={busy} onClick={action}>{busy ? "Pracuję..." : "Uruchom"}</button>
    </div>
  );
}
