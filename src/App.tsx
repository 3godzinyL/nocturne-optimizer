import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AppWindow,
  CheckCircle2,
  Cog,
  Cpu,
  Gauge,
  HardDrive,
  KeyRound,
  LayoutTemplate,
  LoaderCircle,
  MemoryStick,
  Network,
  Power,
  Rocket,
  Search,
  Shield,
  ShieldAlert,
  Sparkles,
  TimerReset,
  TriangleAlert,
  Wifi,
  Zap
} from "lucide-react";
import { Sidebar, type NavItem } from "./components/Sidebar";
import { api } from "./lib/tauri";
import { ProcessIcon } from "./components/ProcessIcon";
import { MetricGauge } from "./components/MetricGauge";
import { AppPasswordGate } from "./components/AppPasswordGate";
import type {
  AppInventory,
  AutostartItem,
  NetworkRule,
  OptimizationRule,
  ProcessInfo,
  RegistryHealthItem,
  SecurityConfig,
  SecurityRuntime,
  SettingsState,
  SystemSnapshot,
  NetworkOverview
} from "./types";

const navItems: NavItem[] = [
  { id: "overview", label: "01. Przegląd", icon: Gauge },
  { id: "optimization", label: "02. Live optymalizacja", icon: Rocket },
  { id: "autostart", label: "03. Autostart", icon: Power },
  { id: "offline", label: "04. Offline optymalizacja", icon: HardDrive },
  { id: "registry", label: "05. Sprawność reg", icon: KeyRound },
  { id: "security", label: "06. Bezpieczeństwo", icon: Shield },
  { id: "network", label: "07. Sieć", icon: Wifi },
  { id: "settings", label: "08. Ustawienia", icon: Cog }
];

type FamilyDefinition = {
  key: string;
  label: string;
  subtitle: string;
  iconHint: string;
  aliases: string[];
};

const familyDefinitions: FamilyDefinition[] = [
  { key: "discord", label: "Discord", subtitle: "app + updater + helper services", iconHint: "discord", aliases: ["discord.exe", "discord", "discordcanary", "discordptb", "discord/", "discord\\", "squirrel"] },
  { key: "chrome", label: "Google Chrome", subtitle: "browser + Google Update + renderer helpers", iconHint: "chrome", aliases: ["chrome.exe", "google chrome", "google/chrome", "google\\chrome", "googleupdate", "google/update", "google\\update", "chrome_proxy"] },
  { key: "msedge", label: "Microsoft Edge", subtitle: "browser + Edge Update + WebView helpers", iconHint: "msedge", aliases: ["msedge.exe", "microsoft edge", "microsoft/edge", "microsoft\\edge", "edgeupdate", "msedgewebview2"] },
  { key: "firefox", label: "Firefox", subtitle: "browser + Mozilla background tasks", iconHint: "firefox", aliases: ["firefox.exe", "mozilla firefox", "mozilla/firefox", "mozilla\\firefox"] },
  { key: "brave", label: "Brave", subtitle: "browser + Brave update chain", iconHint: "brave", aliases: ["brave.exe", "bravesoftware/brave-browser", "bravesoftware\\brave-browser", "braveupdate"] },
  { key: "opera", label: "Opera", subtitle: "browser + GX / launcher stack", iconHint: "opera", aliases: ["opera.exe", "operagx", "opera gx", "opera/", "opera\\"] },
  { key: "telegram", label: "Telegram", subtitle: "desktop app + updater", iconHint: "telegram", aliases: ["telegram.exe", "telegram desktop", "telegram/", "telegram\\"] },
  { key: "steam", label: "Steam", subtitle: "client + web helper + service", iconHint: "steam", aliases: ["steam.exe", "steamservice", "steamwebhelper", "steam/", "steam\\"] },
  { key: "spotify", label: "Spotify", subtitle: "player + launcher + background helper", iconHint: "spotify", aliases: ["spotify.exe", "spotify/", "spotify\\", "spotifylauncher"] },
  { key: "code", label: "VS Code", subtitle: "editor + extension / helper processes", iconHint: "code", aliases: ["code.exe", "code helper", "visual studio code", "vscode"] },
  { key: "teams", label: "Microsoft Teams", subtitle: "desktop app + updater + web runtime", iconHint: "teams", aliases: ["teams.exe", "microsoft teams", "msteams", "teams/", "teams\\"] },
  { key: "riot", label: "Riot Client", subtitle: "launcher + game bootstrap processes", iconHint: "riot", aliases: ["riot client", "riotclient", "riot/", "riot\\", "valorant"] },
  { key: "epic", label: "Epic Games", subtitle: "launcher + background update services", iconHint: "epic", aliases: ["epicgameslauncher", "epic games", "epic/", "epic\\"] }
];

const popularChoices = ["discord.exe", "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "telegram.exe", "steam.exe"];

const defaultSettings: SettingsState = {
  refreshMs: 3800,
  autoApplyRules: true,
  aggressiveMode: false,
  minimizeToTray: true,
  hudEnabled: false,
  hudHotkey: "Ctrl+Shift+H",
  hudCorner: "top-right",
  hudOpacity: 82,
  hudScale: 100,
  hudShowCpu: true,
  hudShowRam: true,
  hudShowProcesses: true,
  hudShowUptime: true,
  hudShowTopApp: true,
  hudPositionMode: "corner",
  hudX: 32,
  hudY: 32,
  hudWidth: 420,
  hudHeight: 220,
  launchOnLogin: false
};

const defaultSecurity: SecurityConfig = {
  passwordSet: false,
  fileProtection: false,
  protectedApps: [],
  lockEnabled: false,
  lockOnRestore: true,
  lockOnActivate: true,
  graceMinutes: 0,
  appPasswordOnStart: false
};

function fmtGb(current: number, total: number) {
  return `${current.toFixed(1)} / ${total.toFixed(1)} GB`;
}

function fmtUptime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}

function cleanProcessLabel(value: string) {
  return value.replace(/\.exe$/i, "").replace(/[_-]+/g, " ").replace(/\s+/g, " ").trim();
}

function titleCase(value: string) {
  return cleanProcessLabel(value).replace(/\b\w/g, (char) => char.toUpperCase());
}

function familyDefinitionForProcess(proc: Pick<ProcessInfo, "name" | "exe">) {
  const haystack = `${proc.name} ${proc.exe}`.toLowerCase().replace(/\\/g, "/");
  return familyDefinitions.find((family) => family.aliases.some((alias) => haystack.includes(alias)));
}

function familyKeyForProcess(proc: ProcessInfo) {
  const mapped = familyDefinitionForProcess(proc);
  if (mapped) return mapped.key;

  const name = proc.name.toLowerCase();
  const exe = proc.exe.toLowerCase();
  if (name.includes("explorer") || exe.includes("/explorer")) return "explorer";
  if (name.includes("obs") || exe.includes("obs-studio")) return "obs";
  return cleanProcessLabel(name).toLowerCase();
}

type ProcessGroup = {
  key: string;
  name: string;
  subtitle: string;
  iconHint: string;
  cpu: number;
  memoryMb: number;
  foreground: boolean;
  optimizedState: string;
  helpers: string[];
  componentSummary: string[];
  exe: string;
  processes: ProcessInfo[];
  primaryProcess: ProcessInfo;
  rule?: OptimizationRule;
};

function groupProcesses(processes: ProcessInfo[], rules: OptimizationRule[]): ProcessGroup[] {
  const map = new Map<string, ProcessGroup>();
  for (const proc of processes) {
    const key = familyKeyForProcess(proc);
    const family = familyDefinitions.find((item) => item.key === key);
    const current = map.get(key) ?? {
      key,
      name: family?.label ?? titleCase(proc.displayName || proc.name),
      subtitle: family?.subtitle ?? "main process + detected helper chain",
      iconHint: family?.iconHint || proc.iconHint || proc.name,
      cpu: 0,
      memoryMb: 0,
      foreground: false,
      optimizedState: proc.optimizedState,
      helpers: [],
      componentSummary: [],
      exe: proc.exe,
      processes: [],
      primaryProcess: proc,
      rule: undefined
    };
    current.cpu += proc.cpu;
    current.memoryMb += proc.memoryMb;
    current.foreground ||= proc.foreground;
    if (proc.optimizedState !== "Normal") current.optimizedState = proc.optimizedState;
    current.helpers.push(`${proc.displayName} ${proc.exe}`.trim());
    current.processes.push(proc);
    if (!current.exe && proc.exe) current.exe = proc.exe;
    if (
      proc.foreground
      || proc.cpu > current.primaryProcess.cpu
      || proc.memoryMb > current.primaryProcess.memoryMb
    ) {
      current.primaryProcess = proc;
      current.exe = proc.exe || current.exe;
      if (!family) current.name = titleCase(proc.displayName || proc.name);
    }

    const partLabel = titleCase(proc.displayName || proc.name);
    if (!current.componentSummary.includes(partLabel)) {
      current.componentSummary.push(partLabel);
    }

    current.rule = current.rule ?? rules.find((rule) => rule.enabled && (rule.familyKey === key || rule.processName.toLowerCase().includes(key) || key.includes(rule.processName.toLowerCase().replace(/\.exe$/i, ""))));
    map.set(key, current);
  }
  return Array.from(map.values()).sort((a, b) => {
    if (!!a.rule !== !!b.rule) return a.rule ? -1 : 1;
    if (a.foreground !== b.foreground) return a.foreground ? -1 : 1;
    if (Math.abs(b.cpu - a.cpu) > 0.2) return b.cpu - a.cpu;
    return b.memoryMb - a.memoryMb;
  });
}

export default function App() {
  const [active, setActive] = useState("overview");
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [rules, setRules] = useState<OptimizationRule[]>([]);
  const [autostart, setAutostart] = useState<AutostartItem[]>([]);
  const [registry, setRegistry] = useState<RegistryHealthItem[]>([]);
  const [securityConfig, setSecurityConfig] = useState<SecurityConfig>(defaultSecurity);
  const [securityRuntime, setSecurityRuntime] = useState<SecurityRuntime>({ locked: false, presentPopularApps: [], overlayBounds: null });
  const [settings, setSettings] = useState<SettingsState>(defaultSettings);
  const [inventory, setInventory] = useState<AppInventory>({ windowsApps: [], installedPrograms: [] });
  const [networkOverview, setNetworkOverview] = useState<NetworkOverview>({ adapters: [], rules: [] });
  const [draftProcess, setDraftProcess] = useState("chrome");
  const [draftMode, setDraftMode] = useState<OptimizationRule["mode"]>("Balanced");
  const [draftPassword, setDraftPassword] = useState("");
  const [draftNetworkProcess, setDraftNetworkProcess] = useState("chrome.exe");
  const [draftNetworkLimit, setDraftNetworkLimit] = useState(4096);
  const [busy, setBusy] = useState<string | null>(null);
  const [info, setInfo] = useState("Silnik działa lżej: żywe snapshoty idą z cache, cięższe zakładki i skany są lazy-loadowane.");
  const [securitySearch, setSecuritySearch] = useState("");
  const [processSearch, setProcessSearch] = useState("");
  const [startupUnlocked, setStartupUnlocked] = useState(false);
  const [selectedGroupKey, setSelectedGroupKey] = useState<string>("");
  const [hudDesignerOpen, setHudDesignerOpen] = useState(false);
  const [loadedTabs, setLoadedTabs] = useState<Record<string, boolean>>({ overview: true, optimization: true });
  const [loadingTabs, setLoadingTabs] = useState<Record<string, boolean>>({});

  const liveBusyRef = useRef(false);
  const staticBusyRef = useRef<Record<string, boolean>>({});

  const processGroups = useMemo(() => groupProcesses(snapshot?.processes ?? [], rules), [snapshot?.processes, rules]);
  const selectedGroup = useMemo(() => processGroups.find((group) => group.key === selectedGroupKey) ?? processGroups[0], [processGroups, selectedGroupKey]);
  const liveChoices = useMemo(() => {
    const map = new Map<string, { key: string; label: string; subtitle: string; iconHint: string; active: boolean }>();
    for (const group of processGroups) {
      map.set(group.key, {
        key: group.key,
        label: group.name,
        subtitle: `${group.processes.length} skladowych · ${group.cpu.toFixed(1)}% CPU`,
        iconHint: group.iconHint,
        active: true
      });
    }
    for (const family of familyDefinitions) {
      if (!map.has(family.key)) {
        map.set(family.key, {
          key: family.key,
          label: family.label,
          subtitle: family.subtitle,
          iconHint: family.iconHint,
          active: false
        });
      }
    }
    return Array.from(map.values()).sort((a, b) => Number(b.active) - Number(a.active) || a.label.localeCompare(b.label));
  }, [processGroups]);

  const processOptions = useMemo(() => {
    const groupNames = processGroups.map((group) => `${group.key}.exe`);
    const fromRuntime = securityRuntime.presentPopularApps;
    return Array.from(new Set([...popularChoices, ...groupNames, ...fromRuntime])).sort();
  }, [processGroups, securityRuntime.presentPopularApps]);

  const refreshLive = async () => {
    if (liveBusyRef.current) return;
    liveBusyRef.current = true;
    try {
      const [snap, runtime] = await Promise.all([api.getSystemSnapshot(), api.getSecurityRuntime()]);
      setSnapshot(snap);
      setSecurityRuntime(runtime);
      if (runtime.locked) window.dispatchEvent(new Event("nocturne:open-guard"));
    } catch (error) {
      setInfo(String(error));
    } finally {
      liveBusyRef.current = false;
    }
  };

  const refreshRules = async () => {
    try {
      const ruleRows = await api.getRules();
      setRules(ruleRows.map((rule) => ({
        cpuLimitPct: 65,
        ramLimitPct: 70,
        diskLimitPct: 60,
        gpuLimitPct: 55,
        familyKey: rule.processName.replace(/\.exe$/i, ""),
        ...rule
      })));
    } catch (error) {
      setInfo(String(error));
    }
  };

  const refreshCore = async () => {
    try {
      const [security, appSettings, selfAutostart] = await Promise.all([
        api.getSecurityConfig(),
        api.getSettings(),
        api.getSelfAutostart()
      ]);
      setSecurityConfig(security);
      setSettings({ ...defaultSettings, ...appSettings, launchOnLogin: selfAutostart });
      setStartupUnlocked(!security.appPasswordOnStart || !security.passwordSet);
    } catch (error) {
      setInfo(String(error));
    }
  };

  const ensureTabData = async (tabId: string) => {
    if (loadedTabs[tabId] || staticBusyRef.current[tabId]) return;
    staticBusyRef.current[tabId] = true;
    setLoadingTabs((current) => ({ ...current, [tabId]: true }));
    try {
      if (tabId === "autostart") setAutostart(await api.listAutostartItems());
      if (tabId === "registry") setRegistry(await api.getRegistryHealth());
      if (tabId === "offline") setInventory(await api.getAppInventory());
      if (tabId === "security" || tabId === "settings") await refreshCore();
      if (tabId === "network") setNetworkOverview(await api.getNetworkOverview());
      setLoadedTabs((current) => ({ ...current, [tabId]: true }));
    } catch (error) {
      setInfo(String(error));
    } finally {
      staticBusyRef.current[tabId] = false;
      setLoadingTabs((current) => ({ ...current, [tabId]: false }));
    }
  };

  useEffect(() => {
    refreshLive().catch(() => undefined);
    refreshRules().catch(() => undefined);
    refreshCore().catch(() => undefined);
  }, []);

  useEffect(() => {
    ensureTabData(active).catch(() => undefined);
  }, [active]);

  useEffect(() => {
    const isLiveTab = active === "overview" || active === "optimization";
    if (!isLiveTab) return;
    const timer = window.setInterval(() => {
      refreshLive().catch(() => undefined);
    }, Math.max(2600, settings.refreshMs || 2600));
    return () => window.clearInterval(timer);
  }, [active, settings.refreshMs]);


  useEffect(() => {
    if (!selectedGroupKey && processGroups[0]) setSelectedGroupKey(processGroups[0].key);
  }, [processGroups, selectedGroupKey]);

  useEffect(() => {
    if (!liveChoices.length) return;
    if (!liveChoices.some((choice) => choice.key === draftProcess)) {
      setDraftProcess(liveChoices[0].key);
    }
  }, [liveChoices, draftProcess]);

  const filteredGroups = useMemo(() => {
    const query = processSearch.trim().toLowerCase();
    if (!query) return processGroups;
    return processGroups.filter((group) =>
      group.name.toLowerCase().includes(query)
      || group.exe.toLowerCase().includes(query)
      || group.helpers.some((helper) => helper.toLowerCase().includes(query))
      || group.componentSummary.some((helper) => helper.toLowerCase().includes(query))
    );
  }, [processGroups, processSearch]);
  const filteredLiveChoices = useMemo(() => {
    const query = processSearch.trim().toLowerCase();
    if (!query) return liveChoices;
    return liveChoices.filter((choice) => choice.label.toLowerCase().includes(query) || choice.subtitle.toLowerCase().includes(query) || choice.key.toLowerCase().includes(query));
  }, [liveChoices, processSearch]);

  const securityCandidates = useMemo(() => {
    const query = securitySearch.trim().toLowerCase();
    const base = processOptions.map((name) => ({ name, active: securityConfig.protectedApps.includes(name.toLowerCase()) || securityConfig.protectedApps.includes(name.replace(/\.exe$/i, "")) }));
    return query ? base.filter((item) => item.name.toLowerCase().includes(query)) : base;
  }, [processOptions, securityConfig.protectedApps, securitySearch]);

  const addRule = async () => {
    const baseKey = draftProcess.replace(/\.exe$/i, "").toLowerCase();
    const processName = `${baseKey}.exe`;
    const displayLabel = liveChoices.find((choice) => choice.key === baseKey)?.label ?? processName;
    const current = rules.filter((rule) => rule.familyKey !== baseKey && rule.processName !== processName);
    const next: OptimizationRule[] = [{
      id: crypto.randomUUID(),
      processName,
      familyKey: baseKey,
      mode: draftMode,
      requireBackground: true,
      autoResume: true,
      enabled: true,
      cpuLimitPct: selectedGroup?.rule?.cpuLimitPct ?? 65,
      ramLimitPct: selectedGroup?.rule?.ramLimitPct ?? 70,
      diskLimitPct: selectedGroup?.rule?.diskLimitPct ?? 60,
      gpuLimitPct: selectedGroup?.rule?.gpuLimitPct ?? 55
    }, ...current];
    setRules(next);
    await api.saveRules(next);
    setInfo(`Dodano grupową regułę dla ${displayLabel}. Obejmuje całą rodzinę procesu i helpery po ścieżce EXE.`);
  };

  const updateSelectedRuleLimits = async (field: keyof Pick<OptimizationRule, "cpuLimitPct" | "ramLimitPct" | "diskLimitPct" | "gpuLimitPct">, value: number) => {
    if (!selectedGroup) return;
    const key = selectedGroup.key;
    const currentRule = rules.find((rule) => rule.familyKey === key || rule.processName.toLowerCase().includes(key));
    const baseRule = currentRule ?? {
      id: crypto.randomUUID(),
      processName: `${key}.exe`,
      familyKey: key,
      mode: selectedGroup.rule?.mode ?? "Balanced",
      requireBackground: true,
      autoResume: true,
      enabled: true,
      cpuLimitPct: selectedGroup.rule?.cpuLimitPct ?? 65,
      ramLimitPct: selectedGroup.rule?.ramLimitPct ?? 70,
      diskLimitPct: selectedGroup.rule?.diskLimitPct ?? 60,
      gpuLimitPct: selectedGroup.rule?.gpuLimitPct ?? 55
    };
    const next = currentRule
      ? rules.map((rule) => rule.id === currentRule.id ? { ...rule, [field]: value } : rule)
      : [{ ...baseRule, [field]: value }, ...rules];
    setRules(next);
    await api.saveRules(next);
  };

  const toggleRule = async (id: string) => {
    const next = rules.map((rule) => rule.id === id ? { ...rule, enabled: !rule.enabled } : rule);
    setRules(next);
    await api.saveRules(next);
  };

  const removeRule = async (id: string) => {
    const next = rules.filter((rule) => rule.id !== id);
    setRules(next);
    await api.saveRules(next);
  };

  const refreshAutostart = async () => {
    setBusy("autostart-refresh");
    setLoadingTabs((current) => ({ ...current, autostart: true }));
    try {
      setAutostart(await api.listAutostartItems());
      setLoadedTabs((current) => ({ ...current, autostart: true }));
    } finally {
      setLoadingTabs((current) => ({ ...current, autostart: false }));
      setBusy(null);
    }
  };

  const toggleAutostart = async (item: AutostartItem) => {
    setBusy(item.id);
    try {
      await api.toggleAutostartItem(item, !item.enabled);
      await refreshAutostart();
      setInfo(`${item.name}: ${item.enabled ? "wyłączono" : "włączono"} autostart.`);
    } finally {
      setBusy(null);
    }
  };

  const runPreset = async (presetId: string) => {
    setBusy(presetId);
    setLoadingTabs((current) => ({ ...current, offline: true }));
    try {
      const result = await api.runOfflinePreset(presetId);
      setInfo(result.details);
      setInventory(await api.getAppInventory());
      setLoadedTabs((current) => ({ ...current, offline: true }));
    } finally {
      setLoadingTabs((current) => ({ ...current, offline: false }));
      setBusy(null);
    }
  };

  const saveSecurity = async () => {
    await api.saveSecurityConfig({
      password: draftPassword || undefined,
      fileProtection: securityConfig.fileProtection,
      protectedApps: securityConfig.protectedApps.map((x) => x.toLowerCase()),
      lockEnabled: securityConfig.lockEnabled,
      lockOnRestore: securityConfig.lockOnRestore,
      lockOnActivate: securityConfig.lockOnActivate,
      graceMinutes: securityConfig.graceMinutes,
      appPasswordOnStart: securityConfig.appPasswordOnStart
    });
    setDraftPassword("");
    const fresh = await api.getSecurityConfig();
    setSecurityConfig(fresh);
    if (fresh.appPasswordOnStart && fresh.passwordSet) setStartupUnlocked(false);
    setInfo("Zapisano konfigurację bezpieczeństwa i globalne hasło do chronionych aplikacji.");
  };

  const saveAppSettings = async () => {
    const { launchOnLogin, ...rest } = settings;
    await api.saveSettings({ ...rest, launchOnLogin });
    await api.setSelfAutostart(launchOnLogin);
    setInfo("Zapisano ustawienia aplikacji i HUD.");
  };

  const saveNetworkRules = async (next: NetworkRule[]) => {
    setNetworkOverview((current) => ({ ...current, rules: next }));
    await api.saveNetworkRules(next);
  };

  const addNetworkRule = async () => {
    const next = [{ id: crypto.randomUUID(), processName: draftNetworkProcess, limitKbps: draftNetworkLimit, enabled: true, note: "UI rule / WFP-ready plan" }, ...networkOverview.rules];
    await saveNetworkRules(next);
    setInfo(`Dodano regułę sieciową dla ${draftNetworkProcess}.`);
  };

  const criticalCount = registry.filter((item) => !item.healthy && item.severity === "critical").length;
  const warningsCount = registry.filter((item) => !item.healthy).length;
  const cpuPercent = snapshot?.cpuUsage ?? 0;
  const ramPercent = snapshot ? (snapshot.ramUsedGb / Math.max(snapshot.ramTotalGb, 0.1)) * 100 : 0;
  const swapPercent = snapshot ? (snapshot.swapUsedGb / Math.max(snapshot.swapTotalGb || 0.1, 0.1)) * 100 : 0;
  const autostartPending = !!loadingTabs.autostart && !loadedTabs.autostart;
  const offlinePending = !!loadingTabs.offline && !loadedTabs.offline;
  const networkPending = !!loadingTabs.network && !loadedTabs.network;

  if (!startupUnlocked) {
    return (
      <AppPasswordGate
        title="Nocturne wymaga hasła startowego"
        subtitle="Ta ochrona działa po starcie aplikacji. Nie zastępuje logowania Windows i nie blokuje Secure Desktop."
        onSubmit={async (password) => {
          const ok = await api.verifyPassword(password.trim());
          if (ok) setStartupUnlocked(true);
          return ok;
        }}
      />
    );
  }

  return (
    <div className="shell-layout">
      <Sidebar items={navItems} active={active} onSelect={setActive} />
      <main className="content-area">
        <header className="topbar shell-card">
          <div>
            <div className="eyebrow">ultra dark dashboard</div>
            <h2>{navItems.find((item) => item.id === active)?.label}</h2>
          </div>
          <div className="topbar-meta"><Sparkles size={16} /> {info}</div>
        </header>

        {active === "overview" && (
          <section className="page-grid page-scrollable">
            <div className="overview-hero shell-card wide">
              <div className="hero-backdrop" />
              <div className="hero-main">
                <div className="eyebrow">shadow telemetry / rebuilt overview</div>
                <h3>Live optimization jest teraz budowane wokół rodzin aplikacji i jednego wybranego profilu, a nie surowego spamu procesów.</h3>
                <p>HUD przełącza się z backendu, helpery są składane pod główną aplikację, a cięższe sekcje dociągają dane dopiero po wejściu w zakładkę.</p>
                <div className="hero-chip-row">
                  <StatusChip icon={Cpu} label="CPU" value={`${cpuPercent.toFixed(1)}%`} />
                  <StatusChip icon={MemoryStick} label="RAM" value={snapshot ? fmtGb(snapshot.ramUsedGb, snapshot.ramTotalGb) : "0 / 0 GB"} />
                  <StatusChip icon={AppWindow} label="Grupy" value={String(processGroups.length)} />
                  <StatusChip icon={LayoutTemplate} label="HUD" value={settings.hudEnabled ? `${settings.hudHotkey}` : "OFF"} />
                </div>
              </div>
              <div className="hero-side">
                <PressureCard title="CPU pressure" value={`${cpuPercent.toFixed(1)}%`} percent={cpuPercent} tone="violet" />
                <PressureCard title="RAM pressure" value={`${ramPercent.toFixed(1)}%`} percent={ramPercent} tone="pink" />
                <PressureCard title="Swap pressure" value={`${swapPercent.toFixed(1)}%`} percent={swapPercent} tone="blue" />
                <div className="hero-selected-card">
                  <div className="eyebrow">selected profile first</div>
                  <div className="row-title">{selectedGroup ? selectedGroup.name : "Brak wybranej rodziny"}</div>
                  <div className="row-sub">
                    {selectedGroup
                      ? `${selectedGroup.processes.length} skladowych · ${selectedGroup.rule ? selectedGroup.rule.mode : "brak reguly"}`
                      : "Przejdz do Live optymalizacja i wybierz glowne app family."}
                  </div>
                </div>
              </div>
            </div>

            <div className="stats-grid wide">
              <MetricGauge label="⚡ CPU" value={`${cpuPercent.toFixed(1)}%`} percent={cpuPercent} sub="rdzenie / pressure" />
              <MetricGauge label="🧠 RAM" value={snapshot ? `${snapshot.ramUsedGb.toFixed(1)} GB` : "0 GB"} percent={ramPercent} sub="pamięć operacyjna" />
              <MetricGauge label="💿 Swap" value={snapshot ? `${snapshot.swapUsedGb.toFixed(1)} GB` : "0 GB"} percent={swapPercent} sub="wymiana / pagefile" />
              <MetricGauge label="⏱ Uptime" value={fmtUptime(snapshot?.uptimeSeconds ?? 0)} percent={Math.min(100, ((snapshot?.uptimeSeconds ?? 0) / 86400) * 100)} sub="od startu systemu" />
            </div>

            <div className="shell-card wide overview-lower-grid">
              <div className="overview-panel">
                <div className="section-head compact-head"><div><div className="eyebrow">live rails</div><h3>Linie obciążenia</h3></div></div>
                <UsageRail label="CPU" value={`${cpuPercent.toFixed(1)}%`} percent={cpuPercent} />
                <UsageRail label="RAM" value={snapshot ? fmtGb(snapshot.ramUsedGb, snapshot.ramTotalGb) : "0 / 0 GB"} percent={ramPercent} />
                <UsageRail label="Swap" value={snapshot ? fmtGb(snapshot.swapUsedGb, snapshot.swapTotalGb) : "0 / 0 GB"} percent={swapPercent} />
                <UsageRail label="HUD opacity" value={`${settings.hudOpacity}%`} percent={settings.hudOpacity} />
                <div className="row-sub">Skrót HUD: <strong>{settings.hudHotkey}</strong> · tryb: {settings.hudPositionMode === "custom" ? `custom ${settings.hudX},${settings.hudY}` : settings.hudCorner}</div>
              </div>
              <div className="overview-panel heavy-groups-panel">
                <div className="section-head compact-head"><div><div className="eyebrow">main app groups</div><h3>Najcięższe grupy aplikacji</h3></div></div>
                <div className="scroll-panel process-spotlight-list">
                  {processGroups.slice(0, 8).map((group, index) => (
                    <button
                      key={group.key}
                      className={`process-spotlight-item ${selectedGroup?.key === group.key ? "active" : ""}`}
                      onClick={() => {
                        setSelectedGroupKey(group.key);
                        setActive("optimization");
                      }}
                    >
                      <div className="spot-rank">0{index + 1}</div>
                      <div className="process-cell">
                        <ProcessIcon name={group.iconHint} />
                        <div>
                          <div className="row-title">{group.name}</div>
                          <div className="row-sub">{group.processes.length} procesów · {group.componentSummary.slice(0, 3).join(" · ")}</div>
                        </div>
                      </div>
                      <div className="spot-metrics">
                        <span>{group.cpu.toFixed(1)}% CPU</span>
                        <span>{group.memoryMb.toFixed(0)} MB</span>
                        <span className={`status-pill ${group.foreground ? "is-live" : group.optimizedState !== "Normal" ? "is-optimized" : ""}`}>{group.foreground ? "Aktywna" : group.optimizedState}</span>
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </section>
        )}

        {active === "optimization" && (
          <section className="page-grid page-scrollable">
            <div className="shell-card wide table-card gradient-panel">
              <div className="section-head"><div><div className="eyebrow">selected profile comes first</div><h3>Live optymalizacja</h3></div></div>
              <div className="live-command-deck">
                <div className="live-family-panel">
                  <div className="section-head compact-head">
                    <div>
                      <div className="eyebrow">grouped app families</div>
                      <h3>Wybierz główną aplikację</h3>
                    </div>
                    <label className="search-box"><Search size={16} /><input value={processSearch} onChange={(e) => setProcessSearch(e.target.value)} placeholder="Szukaj rodziny / helpera / ścieżki" /></label>
                  </div>
                  <div className="family-choice-grid">
                    {filteredLiveChoices.length ? filteredLiveChoices.slice(0, 12).map((choice) => (
                      <button
                        key={choice.key}
                        className={`family-choice-card ${draftProcess === choice.key ? "active" : ""}`}
                        onClick={() => {
                          setDraftProcess(choice.key);
                          if (processGroups.find((group) => group.key === choice.key)) {
                            setSelectedGroupKey(choice.key);
                          }
                        }}
                      >
                        <div className="process-cell">
                          <ProcessIcon name={choice.iconHint} />
                          <div>
                            <div className="row-title">{choice.label}</div>
                            <div className="row-sub">{choice.subtitle}</div>
                          </div>
                        </div>
                        <span className={`status-pill ${choice.active ? "is-live" : ""}`}>{choice.active ? "live" : "preset"}</span>
                      </button>
                    )) : <EmptyState title="Brak dopasowanych rodzin" description="Zmniejsz filtr lub wybierz jedną z aktywnych rodzin procesów z tabeli." />}
                  </div>
                </div>

                <div className="live-mode-panel shell-card inset-card">
                  <div className="eyebrow">throttle mode</div>
                  <h3>Stopień ograniczenia</h3>
                  <div className="mode-chip-row">
                    {(["Eco", "Balanced", "Freeze"] as OptimizationRule["mode"][]).map((mode) => (
                      <button
                        key={mode}
                        className={`mode-chip ${draftMode === mode ? "active" : ""}`}
                        onClick={() => setDraftMode(mode)}
                      >
                        <span>{mode}</span>
                        <small>{mode === "Freeze" ? "twarde zatrzymanie w tle" : mode === "Balanced" ? "nizszy priorytet bez ubijania" : "najlzejszy priorytet i trim"}</small>
                      </button>
                    ))}
                  </div>
                  <div className="mini-note-stack">
                    <div className="mini-note">Reguła działa tylko wtedy, gdy proces nie jest aktywnym oknem.</div>
                    <div className="mini-note">Po powrocie do aplikacji Nocturne robi auto-resume.</div>
                  </div>
                  <button className="primary-button" onClick={() => addRule().catch(() => undefined)}><Zap size={16} /> Dodaj / nadpisz regułę</button>
                </div>
              </div>

              <div className="optimization-grid">
                <div className="optimization-right shell-card inset-card selected-profile-card">
                  <div className="section-head compact-head"><div><div className="eyebrow">selected profile</div><h3>CPU / RAM / Disk / GPU</h3></div></div>
                  {selectedGroup ? (
                    <>
                      <div className="process-cell focus-process-card">
                        <ProcessIcon name={selectedGroup.iconHint} />
                        <div>
                          <div className="row-title">{selectedGroup.name}</div>
                          <div className="row-sub">{selectedGroup.processes.length} składników · główny proces: {selectedGroup.primaryProcess.displayName}</div>
                        </div>
                      </div>
                      <div className="component-pill-row">
                        {selectedGroup.componentSummary.slice(0, 8).map((part) => (
                          <span key={part} className="component-pill">{part}</span>
                        ))}
                      </div>
                      <SliderRow label="CPU" value={selectedGroup.rule?.cpuLimitPct ?? 65} onChange={(value) => updateSelectedRuleLimits("cpuLimitPct", value).catch(() => undefined)} />
                      <SliderRow label="RAM" value={selectedGroup.rule?.ramLimitPct ?? 70} onChange={(value) => updateSelectedRuleLimits("ramLimitPct", value).catch(() => undefined)} />
                      <SliderRow label="Disk" value={selectedGroup.rule?.diskLimitPct ?? 60} onChange={(value) => updateSelectedRuleLimits("diskLimitPct", value).catch(() => undefined)} />
                      <SliderRow label="GPU" value={selectedGroup.rule?.gpuLimitPct ?? 55} onChange={(value) => updateSelectedRuleLimits("gpuLimitPct", value).catch(() => undefined)} />
                      <div className="row-sub">Jedna reguła obejmuje cały pakiet procesu: główny EXE, updatery, web helpery i procesy potomne rozpoznane po rodzinie aplikacji.</div>
                    </>
                  ) : <div className="row-sub">Wybierz rodzinę procesów z listy poniżej.</div>}
                </div>

                <div className="optimization-left">
                  <div className="section-head compact-head">
                    <div><div className="eyebrow">rule board</div><h3>Reguły i tabela rodzin procesów</h3></div>
                  </div>
                  <div className="rule-list">
                    {rules.length ? rules.map((rule) => (
                      <div key={rule.id} className="rule-card">
                        <div className="process-cell">
                          <ProcessIcon name={rule.familyKey || rule.processName} />
                          <div>
                            <div className="row-title">{familyDefinitions.find((family) => family.key === (rule.familyKey || rule.processName.replace(/\.exe$/i, "")))?.label ?? rule.processName}</div>
                            <div className="row-sub">CPU {rule.cpuLimitPct}% · RAM {rule.ramLimitPct}% · Disk {rule.diskLimitPct}% · GPU {rule.gpuLimitPct}%</div>
                          </div>
                        </div>
                        <div className="rule-actions">
                          <span className={`status-pill ${rule.enabled ? "is-optimized" : ""}`}>{rule.enabled ? rule.mode : "OFF"}</span>
                          <button className="ghost-button" onClick={() => toggleRule(rule.id).catch(() => undefined)}>{rule.enabled ? "Wyłącz" : "Włącz"}</button>
                          <button className="ghost-button danger" onClick={() => removeRule(rule.id).catch(() => undefined)}>Usuń</button>
                        </div>
                      </div>
                    )) : <EmptyState title="Brak reguł live" description="Wybierz rodzinę aplikacji wyżej i zapisz pierwszy profil ograniczeń." />}
                  </div>
                  <div className="scroll-panel table-scroll-panel live-table-shell">
                    <table className="data-table live-data-table">
                      <thead><tr><th>Rodzina aplikacji</th><th>Skład</th><th>Obciążenie</th><th>Profil</th><th>Stan</th></tr></thead>
                      <tbody>
                        {filteredGroups.map((group) => (
                          <tr key={group.key} className={selectedGroup?.key === group.key ? "row-highlighted" : ""} onClick={() => setSelectedGroupKey(group.key)}>
                            <td>
                              <div className="process-cell">
                                <ProcessIcon name={group.iconHint} />
                                <div>
                                  <div className="row-title">{group.name}</div>
                                  <div className="row-sub">{group.primaryProcess.displayName} · {group.exe || group.key}</div>
                                </div>
                              </div>
                            </td>
                            <td>
                              <div className="component-pill-row compact">
                                {group.componentSummary.slice(0, 5).map((part) => <span key={part} className="component-pill">{part}</span>)}
                              </div>
                            </td>
                            <td>
                              <div className="table-metric-stack">
                                <strong>{group.cpu.toFixed(1)}% CPU</strong>
                                <span>{group.memoryMb.toFixed(0)} MB RAM</span>
                              </div>
                            </td>
                            <td><span className={`status-pill ${group.rule ? "is-optimized" : ""}`}>{group.rule ? `${group.rule.mode} profile` : "brak reguły"}</span></td>
                            <td><span className={`status-pill ${group.foreground ? "is-live" : group.optimizedState !== "Normal" ? "is-optimized" : ""}`}>{group.foreground ? "Aktywna" : group.optimizedState}</span></td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            </div>
          </section>
        )}

        {active === "autostart" && (
          <section className="page-grid page-scrollable">
            <div className="shell-card wide table-card">
              <div className="section-head">
                <div><div className="eyebrow">registry / startup / wow64 / policy / task / service</div><h3>Autostart</h3></div>
                <button className="ghost-button" onClick={() => refreshAutostart().catch(() => undefined)}>{busy === "autostart-refresh" ? "Odświeżam..." : "Odśwież"}</button>
              </div>
              {autostartPending ? (
                <SectionLoading title="Ładuję wpisy autostartu" description="Sprawdzam rejestr, foldery Startup, polityki, scheduled tasks i usługi." />
              ) : (
                <div className="scroll-panel table-scroll-panel tall-scroll">
                  <table className="data-table">
                    <thead><tr><th>Pozycja</th><th>Źródło</th><th>Stan</th><th>Ścieżka</th><th>Akcja</th></tr></thead>
                    <tbody>
                      {autostart.map((item) => (
                        <tr key={item.id}>
                          <td>
                            <div className="process-cell">
                              <ProcessIcon name={item.iconHint || item.name} />
                              <div><div className="row-title">{item.name}</div><div className="row-sub">{item.itemType} · {item.details}</div></div>
                            </div>
                          </td>
                          <td>{item.source}</td>
                          <td><span className={`status-pill ${item.enabled ? "is-live" : "danger"}`}>{item.enabled ? "On" : "Off"}</span></td>
                          <td className="path-cell">{item.path}</td>
                          <td><button className="ghost-button" disabled={busy === item.id} onClick={() => toggleAutostart(item).catch(() => undefined)}>{busy === item.id ? "Pracuję..." : item.enabled ? "Wyłącz" : "Włącz"}</button></td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </section>
        )}

        {active === "offline" && (
          <section className="page-grid page-scrollable">
            <div className="card-flow wide cards-3">
              <PresetCard title="Temp cleaner" description="Wyczyść katalogi tymczasowe i zbij śmieci po buildach / update'ach." action={() => runPreset("clean_temp").catch(() => undefined)} busy={busy === "clean_temp"} />
              <PresetCard title="Background quiet" description="Przycisz część usług i odśwież shell dla lżejszego tła." action={() => runPreset("background_quiet").catch(() => undefined)} busy={busy === "background_quiet"} />
              <PresetCard title="Debloat lite" description="Wyłącz część podsuwanych sugestii i śmieciowego content delivery Windows." action={() => runPreset("debloat_lite").catch(() => undefined)} busy={busy === "debloat_lite"} />
            </div>

            {offlinePending ? (
              <div className="wide">
                <SectionLoading title="Ładuję offline inventory" description="Skanuję aplikacje systemowe, programy desktopowe i mapowanie autostartu." />
              </div>
            ) : null}

            <div className="shell-card wide table-card">
              <div className="section-head"><div><div className="eyebrow">windows junk inventory</div><h3>Aplikacje systemowe Windows</h3></div></div>
              <div className="scroll-panel table-scroll-panel tall-scroll">
                <table className="data-table">
                  <thead><tr><th>Aplikacja</th><th>Zainstalowana</th><th>Status</th><th>Autostart</th><th>Perms</th><th>Ścieżka</th></tr></thead>
                  <tbody>
                    {inventory.windowsApps.map((app) => (
                      <tr key={app.id}>
                        <td><div className="process-cell"><ProcessIcon name={app.iconHint || app.name} /><div><div className="row-title">{app.name}</div><div className="row-sub">{app.publisher}</div></div></div></td>
                        <td><input type="checkbox" checked={app.installed} readOnly /></td>
                        <td>{app.status}</td>
                        <td><input type="checkbox" checked={app.startupEnabled} readOnly /></td>
                        <td>{app.permissionsSummary}</td>
                        <td className="path-cell">{app.path}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            <div className="shell-card wide table-card">
              <div className="section-head"><div><div className="eyebrow">installed software inventory</div><h3>Zainstalowane aplikacje</h3></div></div>
              <div className="scroll-panel table-scroll-panel tall-scroll">
                <table className="data-table">
                  <thead><tr><th>Aplikacja</th><th>Typ</th><th>Autostart</th><th>Perms</th><th>Wersja</th><th>Ścieżka</th></tr></thead>
                  <tbody>
                    {inventory.installedPrograms.map((app) => (
                      <tr key={app.id}>
                        <td><div className="process-cell"><ProcessIcon name={app.iconHint || app.name} /><div><div className="row-title">{app.name}</div><div className="row-sub">{app.publisher}</div></div></div></td>
                        <td>{app.kind}</td>
                        <td><input type="checkbox" checked={app.startupEnabled} readOnly /></td>
                        <td>{app.permissionsSummary}</td>
                        <td>{app.version}</td>
                        <td className="path-cell">{app.path}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </section>
        )}

        {active === "registry" && (
          <section className="page-grid page-scrollable">
            <div className="shell-card wide table-card">
              <div className="section-head">
                <div><div className="eyebrow">34+ krytycznych punktów</div><h3>Sprawność kluczy rejestru</h3></div>
                <div className="summary-inline">
                  <span className="status-pill danger">Krytyczne: {criticalCount}</span>
                  <span className="status-pill">Łączne uwagi: {warningsCount}</span>
                  <button className="ghost-button" onClick={() => api.runRegistryAuditConsole("scan").then(() => setInfo("Otworzono konsolę sprawdzania rejestru.")).catch((e) => setInfo(String(e)))}>Sprawdź</button>
                  <button className="primary-button" onClick={() => api.runRegistryAuditConsole("repair").then(() => setInfo("Otworzono konsolę naprawy rejestru.")).catch((e) => setInfo(String(e)))}>Napraw</button>
                </div>
              </div>
              <div className="scroll-panel table-scroll-panel tall-scroll">
                <table className="data-table">
                  <thead><tr><th>Klucz</th><th>Obecnie</th><th>Zalecane</th><th>Ocena</th><th>Opis</th></tr></thead>
                  <tbody>
                    {registry.map((row, index) => (
                      <tr key={`${row.keyPath}-${index}`}>
                        <td><div className="row-title">{row.valueName}</div><div className="row-sub">{row.keyPath}</div></td>
                        <td>{row.current}</td>
                        <td>{row.recommended}</td>
                        <td><span className={`status-pill ${row.healthy ? "is-live" : row.severity === "critical" ? "danger" : ""}`}>{row.healthy ? "OK" : row.severity}</span></td>
                        <td>{row.meaning}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </section>
        )}

        {active === "security" && (
          <section className="page-grid page-scrollable">
            <div className="shell-card wide table-card">
              <div className="section-head"><div><div className="eyebrow">global password for protected apps</div><h3>Bezpieczeństwo</h3></div></div>
              <div className="security-grid">
                <div className="security-pane">
                  <label className="check-line"><input type="checkbox" checked={securityConfig.lockEnabled} onChange={(e) => setSecurityConfig({ ...securityConfig, lockEnabled: e.target.checked })} /> blokada dla chronionych aplikacji</label>
                  <label className="check-line"><input type="checkbox" checked={securityConfig.lockOnRestore} onChange={(e) => setSecurityConfig({ ...securityConfig, lockOnRestore: e.target.checked })} /> wymagaj hasła po powrocie do okna</label>
                  <label className="check-line"><input type="checkbox" checked={securityConfig.lockOnActivate} onChange={(e) => setSecurityConfig({ ...securityConfig, lockOnActivate: e.target.checked })} /> wymagaj hasła po ponownej aktywacji</label>
                  <label className="check-line"><input type="checkbox" checked={securityConfig.fileProtection} onChange={(e) => setSecurityConfig({ ...securityConfig, fileProtection: e.target.checked })} /> zabezpieczenie danych w pliku (vault flag)</label>
                  <label className="check-line"><input type="checkbox" checked={securityConfig.appPasswordOnStart} onChange={(e) => setSecurityConfig({ ...securityConfig, appPasswordOnStart: e.target.checked })} /> hasło przy starcie Nocturne</label>
                  <label className="input-group"><span>Grace period (min)</span><input type="number" min={0} max={240} value={securityConfig.graceMinutes} onChange={(e) => setSecurityConfig({ ...securityConfig, graceMinutes: Number(e.target.value) })} /></label>
                  <label className="input-group"><span>Globalne hasło do chronionych aplikacji</span><input type="password" value={draftPassword} onChange={(e) => setDraftPassword(e.target.value)} placeholder={securityConfig.passwordSet ? "ustaw nowe hasło" : "ustaw hasło"} /></label>
                  <button className="primary-button" onClick={() => saveSecurity().catch(() => undefined)}><ShieldAlert size={16} /> Zapisz zabezpieczenia</button>
                  <div className="row-sub">Overlay pilnuje rozmiaru chronionego okna i odświeża pozycję. Dodane aplikacje są widoczne obok jako aktywne reguły.</div>
                </div>
                <div className="security-pane">
                  <div className="section-head compact-head"><div><div className="row-title">Dodane aplikacje</div><div className="row-sub">To właśnie te reguły mają być tu stale widoczne.</div></div></div>
                  <div className="app-tags">
                    {securityConfig.protectedApps.length ? securityConfig.protectedApps.map((item) => (
                      <span key={item} className="tag-button active"><ProcessIcon name={item} className="process-glyph process-glyph-inline" />{item}</span>
                    )) : <div className="row-sub">Brak dodanych chronionych aplikacji.</div>}
                  </div>
                  <label className="search-box"><Search size={16} /><input value={securitySearch} onChange={(e) => setSecuritySearch(e.target.value)} placeholder="Szukaj procesu" /></label>
                  <div className="scroll-panel candidate-grid">
                    {securityCandidates.map((item) => {
                      const activeTag = securityConfig.protectedApps.includes(item.name.toLowerCase()) || securityConfig.protectedApps.includes(item.name.replace(/\.exe$/i, "").toLowerCase());
                      return (
                        <button
                          key={item.name}
                          className={`tag-button ${activeTag ? "active" : ""}`}
                          onClick={() => {
                            const normalized = item.name.replace(/\.exe$/i, "").toLowerCase();
                            const next = activeTag
                              ? securityConfig.protectedApps.filter((app) => app !== item.name.toLowerCase() && app !== normalized)
                              : [...securityConfig.protectedApps, normalized];
                            setSecurityConfig({ ...securityConfig, protectedApps: next });
                          }}
                        >
                          <ProcessIcon name={item.name} className="process-glyph process-glyph-inline" />
                          {item.name}
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

        {active === "network" && (
          <section className="page-grid page-scrollable">
            <div className="shell-card wide table-card">
              <div className="section-head"><div><div className="eyebrow">adaptery i plan ograniczeń</div><h3>Sieć</h3></div><button className="primary-button" onClick={() => api.runNetworkTune().then((result) => setInfo(result.summary)).catch((e) => setInfo(String(e)))}><Network size={16} /> Popraw sieć</button></div>
              {networkPending ? (
                <SectionLoading title="Ładuję sieć" description="Pobieram adaptery, przepływ i zapisane profile limitów." />
              ) : (
                <div className="network-grid">
                  <div className="security-pane">
                    <div className="section-head compact-head"><div><div className="row-title">Nowa reguła sieciowa</div><div className="row-sub">Plan limitu dla procesu. Zapisuje profil i gotowość pod dalsze egzekwowanie.</div></div></div>
                    <label className="input-group"><span>Proces</span><select className="dark-select" value={draftNetworkProcess} onChange={(e) => setDraftNetworkProcess(e.target.value)}>{processOptions.map((name) => <option key={name} value={name}>{name}</option>)}</select></label>
                    <label className="input-group"><span>Limit (KB/s)</span><input type="number" min={64} step={64} value={draftNetworkLimit} onChange={(e) => setDraftNetworkLimit(Number(e.target.value))} /></label>
                    <button className="primary-button" onClick={() => addNetworkRule().catch(() => undefined)}>Dodaj regułę</button>
                    <div className="scroll-panel candidate-grid">
                      {networkOverview.rules.length ? networkOverview.rules.map((rule) => (
                        <div key={rule.id} className="rule-card compact-rule-card">
                          <div><div className="row-title">{rule.processName}</div><div className="row-sub">{rule.limitKbps} KB/s · {rule.note}</div></div>
                          <button className="ghost-button" onClick={() => saveNetworkRules(networkOverview.rules.filter((item) => item.id !== rule.id)).catch(() => undefined)}>Usuń</button>
                        </div>
                      )) : <EmptyState title="Brak profili sieciowych" description="Dodaj pierwszy limit dla procesu, aby przygotować profil pod dalsze egzekwowanie." />}
                    </div>
                  </div>
                  <div className="shell-card inset-card">
                    <div className="section-head compact-head"><div><div className="row-title">Rozdysponowanie sieci</div><div className="row-sub">Aktywne adaptery i ich statystyki I/O.</div></div></div>
                    <div className="scroll-panel table-scroll-panel tall-scroll">
                      <table className="data-table">
                        <thead><tr><th>Adapter</th><th>Status</th><th>Link</th><th>IPv4</th><th>TX</th><th>RX</th></tr></thead>
                        <tbody>
                          {networkOverview.adapters.map((adapter) => (
                            <tr key={adapter.name}>
                              <td><div className="row-title">{adapter.name}</div><div className="row-sub">{adapter.macAddress}</div></td>
                              <td>{adapter.status}</td>
                              <td>{adapter.linkSpeed}</td>
                              <td>{adapter.ipv4}</td>
                              <td>{adapter.sentMb.toFixed(1)} MB</td>
                              <td>{adapter.receivedMb.toFixed(1)} MB</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </section>
        )}

        {active === "settings" && (
          <section className="page-grid page-scrollable cards-2">
            <div className="shell-card table-card">
              <div className="section-head"><div><div className="eyebrow">engine / HUD / autorun</div><h3>Pełne ustawienia</h3></div></div>
              <label className="input-group"><span>Interwał odświeżania (ms)</span><input type="number" min={1400} value={settings.refreshMs} onChange={(e) => setSettings({ ...settings, refreshMs: Number(e.target.value) })} /></label>
              <label className="check-line"><input type="checkbox" checked={settings.autoApplyRules} onChange={(e) => setSettings({ ...settings, autoApplyRules: e.target.checked })} /> automatycznie nakładaj reguły</label>
              <label className="check-line"><input type="checkbox" checked={settings.aggressiveMode} onChange={(e) => setSettings({ ...settings, aggressiveMode: e.target.checked })} /> agresywny monitoring</label>
              <label className="check-line"><input type="checkbox" checked={settings.minimizeToTray} onChange={(e) => setSettings({ ...settings, minimizeToTray: e.target.checked })} /> minimalizuj do tray</label>
              <label className="check-line"><input type="checkbox" checked={settings.launchOnLogin} onChange={(e) => setSettings({ ...settings, launchOnLogin: e.target.checked })} /> uruchamiaj Nocturne po zalogowaniu do Windows</label>
              <label className="check-line"><input type="checkbox" checked={settings.hudEnabled} onChange={(e) => setSettings({ ...settings, hudEnabled: e.target.checked })} /> HUD aktywny</label>
              <label className="input-group"><span>Skrót HUD</span><input value={settings.hudHotkey} onChange={(e) => setSettings({ ...settings, hudHotkey: e.target.value })} /></label>
              <div className="section-head compact-head">
                <div><div className="eyebrow">hud section</div><h3>HUD overlay</h3></div>
                <div className="button-row">
                  <button className="ghost-button" disabled={!settings.hudEnabled} onClick={() => saveAppSettings().then(() => api.toggleHudWindow()).catch((error) => setInfo(String(error)))}>Pokaż / ukryj HUD</button>
                  <button className="ghost-button" onClick={() => setHudDesignerOpen(true)}>Tryb ustawiania</button>
                </div>
              </div>
              <label className="input-group"><span>Pozycjonowanie</span><select className="dark-select" value={settings.hudPositionMode} onChange={(e) => setSettings({ ...settings, hudPositionMode: e.target.value as SettingsState["hudPositionMode"] })}><option value="corner">róg</option><option value="custom">custom XY</option></select></label>
              <label className="input-group"><span>Róg HUD</span><select className="dark-select" value={settings.hudCorner} onChange={(e) => setSettings({ ...settings, hudCorner: e.target.value as SettingsState["hudCorner"] })}><option value="top-left">lewy górny</option><option value="top-right">prawy górny</option><option value="bottom-left">lewy dolny</option><option value="bottom-right">prawy dolny</option></select></label>
              <label className="input-group"><span>Przezroczystość HUD ({settings.hudOpacity}%)</span><input type="range" min={30} max={100} value={settings.hudOpacity} onChange={(e) => setSettings({ ...settings, hudOpacity: Number(e.target.value) })} /></label>
              <label className="input-group"><span>Skala HUD ({settings.hudScale}%)</span><input type="range" min={70} max={150} value={settings.hudScale} onChange={(e) => setSettings({ ...settings, hudScale: Number(e.target.value) })} /></label>
              <label className="input-group"><span>Szerokość HUD ({settings.hudWidth}px)</span><input type="range" min={320} max={680} value={settings.hudWidth} onChange={(e) => setSettings({ ...settings, hudWidth: Number(e.target.value) })} /></label>
              <label className="input-group"><span>Wysokość HUD ({settings.hudHeight}px)</span><input type="range" min={160} max={420} value={settings.hudHeight} onChange={(e) => setSettings({ ...settings, hudHeight: Number(e.target.value) })} /></label>
              <div className="slider-grid-2">
                <label className="input-group"><span>HUD X</span><input type="range" min={0} max={1200} value={settings.hudX} onChange={(e) => setSettings({ ...settings, hudX: Number(e.target.value) })} /></label>
                <label className="input-group"><span>HUD Y</span><input type="range" min={0} max={720} value={settings.hudY} onChange={(e) => setSettings({ ...settings, hudY: Number(e.target.value) })} /></label>
              </div>
              <div className="checkbox-grid-2">
                <label className="check-line"><input type="checkbox" checked={settings.hudShowCpu} onChange={(e) => setSettings({ ...settings, hudShowCpu: e.target.checked })} /> pokazuj CPU</label>
                <label className="check-line"><input type="checkbox" checked={settings.hudShowRam} onChange={(e) => setSettings({ ...settings, hudShowRam: e.target.checked })} /> pokazuj RAM</label>
                <label className="check-line"><input type="checkbox" checked={settings.hudShowProcesses} onChange={(e) => setSettings({ ...settings, hudShowProcesses: e.target.checked })} /> pokazuj liczbę procesów</label>
                <label className="check-line"><input type="checkbox" checked={settings.hudShowUptime} onChange={(e) => setSettings({ ...settings, hudShowUptime: e.target.checked })} /> pokazuj uptime</label>
                <label className="check-line"><input type="checkbox" checked={settings.hudShowTopApp} onChange={(e) => setSettings({ ...settings, hudShowTopApp: e.target.checked })} /> pokazuj top app</label>
              </div>
              <button className="primary-button" onClick={() => saveAppSettings().catch(() => undefined)}>Zapisz ustawienia</button>
            </div>
            <div className="shell-card table-card">
              <div className="section-head"><div><div className="eyebrow">security + limits</div><h3>Uwagi techniczne</h3></div></div>
              <div className="warning-stack">
                <div className="warning-card"><CheckCircle2 size={18} /> HUD działa jako globalne okno overlay na cały ekran, ale sterowanie jego położeniem robisz z sekcji ustawień.</div>
                <div className="warning-card"><TriangleAlert size={18} /> Ochrona procesów jest przypięta do okna i hasła aplikacji, ale nie zastępuje logowania Windows ani Secure Desktop.</div>
                <div className="warning-card"><TriangleAlert size={18} /> Reguły sieciowe są zapisywane jako profil aplikacji; pełne egzekwowanie per-proces bandwidth shaping na Windows wymaga głębszej integracji z WFP.</div>
              </div>
            </div>
          </section>
        )}
      </main>

      {hudDesignerOpen ? (
        <div className="designer-overlay">
          <div className="designer-backdrop" />
          <div className="designer-panel shell-card">
            <div className="section-head compact-head"><div><div className="eyebrow">hud placement mode</div><h3>Przyciemnij wszystko i ustaw HUD</h3></div><button className="ghost-button" onClick={() => setHudDesignerOpen(false)}>Zamknij</button></div>
            <div className="designer-stage">
              <div className="designer-screen">
                <div className="designer-hud-preview" style={{ width: settings.hudWidth / 2, minHeight: settings.hudHeight / 2, left: settings.hudPositionMode === "custom" ? settings.hudX / 2 : undefined, top: settings.hudPositionMode === "custom" ? settings.hudY / 2 : undefined, opacity: settings.hudOpacity / 100 }}>
                  <div className="eyebrow">preview</div>
                  <div className="row-title">Nocturne HUD</div>
                  <div className="row-sub">Zapisz, a taki panel pojawi się na pulpicie.</div>
                </div>
              </div>
              <div className="designer-controls">
                <label className="input-group"><span>X ({settings.hudX}px)</span><input type="range" min={0} max={1400} value={settings.hudX} onChange={(e) => setSettings({ ...settings, hudPositionMode: "custom", hudX: Number(e.target.value) })} /></label>
                <label className="input-group"><span>Y ({settings.hudY}px)</span><input type="range" min={0} max={900} value={settings.hudY} onChange={(e) => setSettings({ ...settings, hudPositionMode: "custom", hudY: Number(e.target.value) })} /></label>
                <label className="input-group"><span>Szerokość ({settings.hudWidth}px)</span><input type="range" min={320} max={680} value={settings.hudWidth} onChange={(e) => setSettings({ ...settings, hudWidth: Number(e.target.value) })} /></label>
                <label className="input-group"><span>Wysokość ({settings.hudHeight}px)</span><input type="range" min={160} max={420} value={settings.hudHeight} onChange={(e) => setSettings({ ...settings, hudHeight: Number(e.target.value) })} /></label>
                <button className="primary-button" onClick={() => saveAppSettings().then(() => setHudDesignerOpen(false)).catch(() => undefined)}>Zapisz układ HUD</button>
              </div>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function PresetCard({ title, description, action, busy }: { title: string; description: string; action: () => void; busy: boolean }) {
  return (
    <div className="shell-card preset-card glowing">
      <div className="eyebrow">offline preset</div>
      <h3>{title}</h3>
      <p>{description}</p>
      <button className="primary-button" disabled={busy} onClick={action}>{busy ? <><LoaderCircle size={16} className="spin" /> Pracuję...</> : "Uruchom"}</button>
    </div>
  );
}

function SectionLoading({ title, description }: { title: string; description: string }) {
  return (
    <div className="section-loading shell-card">
      <div className="section-loading-icon"><LoaderCircle size={20} className="spin" /></div>
      <div>
        <div className="row-title">{title}</div>
        <div className="row-sub">{description}</div>
      </div>
    </div>
  );
}

function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <div className="empty-state">
      <div className="row-title">{title}</div>
      <div className="row-sub">{description}</div>
    </div>
  );
}

function StatusChip({ icon: Icon, label, value }: { icon: typeof Activity; label: string; value: string }) {
  return <div className="hero-chip"><Icon size={15} /><span>{label}</span><strong>{value}</strong></div>;
}

function PressureCard({ title, value, percent, tone }: { title: string; value: string; percent: number; tone: "violet" | "pink" | "blue" }) {
  return <div className={`pressure-card ${tone}`}><div className="pressure-top"><span>{title}</span><strong>{value}</strong></div><div className="pressure-bar"><div style={{ width: `${Math.max(6, Math.min(100, percent))}%` }} /></div></div>;
}

function UsageRail({ label, value, percent }: { label: string; value: string; percent: number }) {
  return <div className="usage-rail"><div className="usage-rail-top"><span>{label}</span><strong>{value}</strong></div><div className="usage-rail-bar"><div style={{ width: `${Math.max(4, Math.min(100, percent))}%` }} /></div></div>;
}

function SliderRow({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <label className="input-group slider-row">
      <span>{label} <strong>{value}%</strong></span>
      <input type="range" min={5} max={100} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  );
}
