import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  AlertTriangle,
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  AudioLines,
  CheckCircle2,
  Blocks,
  ChevronRight,
  CircleHelp,
  Command,
  Cpu,
  Gamepad2,
  Github,
  LayoutDashboard,
  LoaderCircle,
  MonitorPlay,
  PackageOpen,
  Play,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  Terminal,
  Volume2,
  WandSparkles,
  Trash2,
  X,
} from "lucide-react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import logo from "./assets/ws-logo.png";

type ModuleId =
  | "overview"
  | "mod-organizer"
  | "game-library"
  | "app-launcher"
  | "voice-control"
  | "dotfile-sync"
  | "system-tools"
  | "obs";

type ModuleInfo = {
  id: Exclude<ModuleId, "overview">;
  name: string;
  eyebrow: string;
  description: string;
  icon: typeof PackageOpen;
  accent: "cyan" | "orange" | "violet";
};

const modules: ModuleInfo[] = [
  {
    id: "mod-organizer",
    name: "Mod Organizer 2",
    eyebrow: "PROTON / MODDING",
    description: "Prepare MO2 and its Proton dependencies in one place.",
    icon: PackageOpen,
    accent: "cyan",
  },
  {
    id: "game-library",
    name: "Game library",
    eyebrow: "STEAM / NON-STEAM",
    description: "Scan installed Steam games without launching apps or changing files.",
    icon: Gamepad2,
    accent: "orange",
  },
  {
    id: "app-launcher",
    name: "App launcher",
    eyebrow: "WORKSPACE",
    description: "Save favorite app paths locally and launch them with confirmation.",
    icon: Blocks,
    accent: "violet",
  },
  {
    id: "voice-control",
    name: "Ollama voice",
    eyebrow: "LOCAL AI / AUDIO",
    description: "Explore local voice commands powered by Ollama.",
    icon: AudioLines,
    accent: "cyan",
  },
  {
    id: "dotfile-sync",
    name: "Dotfile sync",
    eyebrow: "GITHUB / BACKUP",
    description: "Keep workstation configuration in sync with GitHub.",
    icon: Github,
    accent: "orange",
  },
  {
    id: "system-tools",
    name: "System tools",
    eyebrow: "LINUX / SHORTCUTS",
    description: "Convenient entry points for routine system maintenance.",
    icon: Cpu,
    accent: "violet",
  },
  {
    id: "obs",
    name: "OBS Studio",
    eyebrow: "STREAM / RECORD",
    description: "A future home for your streaming and recording setup.",
    icon: MonitorPlay,
    accent: "cyan",
  },
];

const navItems: { id: ModuleId; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "mod-organizer", label: "Mod Organizer 2", icon: PackageOpen },
  { id: "game-library", label: "Game library", icon: Gamepad2 },
  { id: "app-launcher", label: "App launcher", icon: Blocks },
  { id: "voice-control", label: "Voice control", icon: AudioLines },
  { id: "dotfile-sync", label: "Dotfile sync", icon: Github },
  { id: "system-tools", label: "System tools", icon: Settings2 },
  { id: "obs", label: "OBS Studio", icon: MonitorPlay },
];

function isControlCenterDesktop() {
  return isTauri() && getCurrentWebview().label === "main";
}

function App() {
  const [activeModule, setActiveModule] = useState<ModuleId>("overview");
  const [query, setQuery] = useState("");
  const searchInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (event.key === "/" && !(event.target instanceof HTMLInputElement)) {
        event.preventDefault();
        searchInput.current?.focus();
      }
      if (event.key === "Escape" && document.activeElement === searchInput.current) {
        setQuery("");
        searchInput.current?.blur();
      }
    };

    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  const filteredModules = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return modules;
    return modules.filter((module) =>
      `${module.name} ${module.eyebrow} ${module.description}`.toLowerCase().includes(needle),
    );
  }, [query]);

  const activeInfo = modules.find((module) => module.id === activeModule);

  return (
    <div className="app-shell">
      <div className="scanlines" aria-hidden="true" />
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <img src={logo} alt="" />
          </div>
          <div>
            <div className="brand-name">WS<span>CC</span></div>
            <div className="brand-caption">CONTROL CENTER</div>
          </div>
          <span className="brand-version">0.1</span>
        </div>

        <div className="nav-label">COMMAND</div>
        <nav className="primary-nav" aria-label="Main navigation">
          {navItems.map(({ id, label, icon: Icon }) => (
            <button
              className={`nav-item ${activeModule === id ? "active" : ""}`}
              key={id}
              onClick={() => setActiveModule(id)}
              aria-current={activeModule === id ? "page" : undefined}
            >
              <Icon size={17} strokeWidth={1.8} />
              <span>{label}</span>
              {activeModule === id && <span className="nav-indicator" />}
            </button>
          ))}
        </nav>

        <div className="sidebar-bottom">
          <div className="sidebar-status">
            <div className="status-orb" />
            <div>
              <div className="status-title">UI SHELL READY</div>
              <div className="status-subtitle">LOCAL ONLY · NO PROBES</div>
            </div>
          </div>
          <div className="sidebar-footer">
            <span>BUILD 001</span>
            <span>WS // 2026</span>
          </div>
        </div>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div className="breadcrumb">
            <span>WORKSPACE</span>
            <ChevronRight size={13} />
            <strong>{activeInfo?.name.toUpperCase() ?? "OVERVIEW"}</strong>
          </div>
          <div className="topbar-right">
            <label className="search-box">
              <Search size={15} />
              <input
                ref={searchInput}
                type="search"
                placeholder="Filter modules..."
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                aria-label="Filter modules"
              />
              {query && (
                <button className="clear-search" onClick={() => setQuery("")} aria-label="Clear search">
                  <X size={13} />
                </button>
              )}
              {!query && <kbd>/</kbd>}
            </label>
            <div className="topbar-divider" />
            <div className="host-pill"><span /> DESKTOP</div>
            <button
              className="help-button"
              aria-label="Return to overview"
              onClick={() => setActiveModule("overview")}
              title="Return to overview"
            >
              <CircleHelp size={17} />
            </button>
          </div>
        </header>

        <div className="content-scroll">
          {activeInfo ? (
            <ModuleView module={activeInfo} onBack={() => setActiveModule("overview")} />
          ) : (
            <Dashboard
              query={query}
              filteredModules={filteredModules}
              onSelect={setActiveModule}
            />
          )}
          <footer className="page-footer">
            <span><Terminal size={12} /> WS CONTROL CENTER <i>/</i> LOCAL WORKSPACE</span>
            <span>DESIGNED FOR YOUR SETUP <Sparkles size={12} /></span>
          </footer>
        </div>
      </main>
    </div>
  );
}

function Dashboard({
  query,
  filteredModules,
  onSelect,
}: {
  query: string;
  filteredModules: ModuleInfo[];
  onSelect: (id: ModuleId) => void;
}) {
  return (
    <div className="dashboard page-enter">
      <div className="page-kicker"><span className="kicker-line" /> PERSONAL WORKSTATION / CONTROL PLANE</div>
      <section className="hero">
        <div className="hero-content">
          <div className="hero-overline"><span className="live-dot" /> SYSTEM INTERFACE // ONLINE</div>
          <h1>Your rig.<br /><span>Your rules.</span></h1>
          <p>
            One command deck for your games, tools, mods, and the setup that makes them yours.
          </p>
          <button className="hero-action" onClick={() => onSelect("mod-organizer")}>
            <Command size={15} />
            <span>OPEN MOD ORGANIZER</span>
            <ArrowUpRight size={15} />
          </button>
          <div className="hero-coordinates">LOCALHOST&nbsp;&nbsp; · &nbsp;&nbsp;LOCAL WORKSPACE</div>
        </div>
        <div className="hero-art" aria-hidden="true">
          <div className="hero-grid" />
          <div className="hero-ring ring-one" />
          <div className="hero-ring ring-two" />
          <div className="hero-crosshair">+</div>
          <div className="hero-corner corner-tl" />
          <div className="hero-corner corner-br" />
          <img src={logo} alt="" />
          <div className="hero-art-label">WS / CONTROL<br />INTERFACE 001</div>
          <div className="hero-art-index">001</div>
        </div>
        <div className="hero-edge">WSC // LOCAL-FIRST</div>
      </section>

      <section className="status-strip" aria-label="Workspace status">
        <div className="strip-heading"><Activity size={14} /> WORKSPACE STATUS</div>
        <div className="strip-item"><span className="strip-icon cyan"><ShieldCheck size={15} /></span><span>APP SHELL</span><strong>READY</strong></div>
        <div className="strip-item"><span className="strip-icon orange"><Volume2 size={15} /></span><span>INTEGRATIONS</span><strong className="muted-status">NOT CONNECTED</strong></div>
        <div className="strip-end">LOCAL <span>·</span> SAFE MODE</div>
      </section>

      <section className="modules-section">
        <div className="section-heading">
          <div>
            <div className="section-overline">THE TOOLKIT</div>
            <h2>{query ? "Matching modules" : "Build your command deck"}</h2>
          </div>
          <div className="module-count"><span>{String(filteredModules.length).padStart(2, "0")}</span> MODULES <i>/</i> {String(modules.length).padStart(2, "0")}</div>
        </div>
        {filteredModules.length ? (
          <div className="module-grid">
            {filteredModules.map((module, index) => (
              <ModuleCard
                key={module.id}
                module={module}
                index={index + 1}
                onClick={() => onSelect(module.id)}
              />
            ))}
          </div>
        ) : (
          <div className="empty-search">
            <Search size={18} />
            <span>No modules match “{query}”. Try another search.</span>
          </div>
        )}
      </section>

      <section className="bottom-grid">
        <div className="terminal-card">
          <div className="card-topline"><span><Terminal size={14} /> BOOT SEQUENCE</span><span className="terminal-id">LOG // 0001</span></div>
          <div className="terminal-lines">
            <div><span className="terminal-time">001</span><b className="terminal-ok">[ OK ]</b><span>Interface initialized</span></div>
            <div><span className="terminal-time">002</span><b className="terminal-ok">[ OK ]</b><span>Local workspace ready</span></div>
            <div><span className="terminal-time">003</span><b className="terminal-wait">[ WAIT ]</b><span>Integrations awaiting setup</span></div>
            <div className="terminal-cursor-row"><span className="terminal-prompt">ws://</span><span className="terminal-cursor" /></div>
          </div>
        </div>
        <div className="vision-card">
          <div className="vision-icon"><WandSparkles size={17} /></div>
          <div className="vision-body">
            <div className="section-overline">BUILT AROUND YOU</div>
            <h3>One place. Your whole setup.</h3>
            <p>Every module starts local, stays transparent, and waits for your permission before it acts.</p>
          </div>
          <ArrowDownRight className="vision-arrow" size={18} />
          <div className="vision-watermark">WS</div>
        </div>
      </section>
    </div>
  );
}

function ModuleCard({
  module,
  index,
  onClick,
}: {
  module: ModuleInfo;
  index: number;
  onClick: () => void;
}) {
  const Icon = module.icon;
  return (
    <button className={`module-card accent-${module.accent}`} onClick={onClick}>
      <div className="module-card-top">
        <span className="module-index">0{index}</span>
        <span className={`module-status ${module.id === "game-library" || module.id === "app-launcher" ? "module-status-ready" : ""}`}>
          <span />
          {module.id === "game-library" ? "READ-ONLY" : module.id === "app-launcher" ? "LOCAL" : "PLANNED"}
        </span>
      </div>
      <div className="module-icon"><Icon size={19} strokeWidth={1.7} /></div>
      <div className="module-eyebrow">{module.eyebrow}</div>
      <h3>{module.name}</h3>
      <p>{module.description}</p>
      <div className="module-card-bottom">
        <span>OPEN MODULE</span><ChevronRight size={15} />
      </div>
    </button>
  );
}

function ModuleView({ module, onBack }: { module: ModuleInfo; onBack: () => void }) {
  const Icon = module.icon;
  return (
    <section className={`module-view page-enter accent-${module.accent}`}>
      <div className="page-kicker"><span className="kicker-line" /> {module.eyebrow} / MODULE 0{modules.findIndex((entry) => entry.id === module.id) + 1}</div>
      <div className="module-view-heading">
        <div className="module-view-icon"><Icon size={22} /></div>
        <div>
          <h1>{module.name}</h1>
          <p>{module.description}</p>
        </div>
        <span className={`planned-badge ${module.id === "game-library" || module.id === "app-launcher" ? "readonly-badge" : ""}`}>
          <span />
          {module.id === "game-library" ? "READ-ONLY" : module.id === "app-launcher" ? "LOCAL CONFIG" : "PLANNED"}
        </span>
      </div>
      {module.id === "game-library" ? (
        <SteamLibrary />
      ) : module.id === "app-launcher" ? (
        <AppLauncher />
      ) : (
        <>
          <div className="module-placeholder">
            <div className="placeholder-grid" />
            <div className="placeholder-orbit orbit-a" />
            <div className="placeholder-orbit orbit-b" />
            <div className="placeholder-center"><Icon size={30} strokeWidth={1.3} /></div>
            <div className="placeholder-label">MODULE INITIALIZATION PENDING</div>
          </div>
          <div className="module-info-grid">
            <div className="module-info-card">
              <div className="section-overline">CURRENT STATE</div>
              <div className="info-value">Not configured</div>
              <p>This module is a UI placeholder. No system data has been read.</p>
            </div>
            <div className="module-info-card">
              <div className="section-overline">NEXT PHASE</div>
              <div className="info-value">Permission-first setup</div>
              <p>Future actions will be explicit, observable, and gated by your approval.</p>
            </div>
          </div>
        </>
      )}
      <button className="back-button" onClick={onBack}><ChevronRight size={14} /> RETURN TO OVERVIEW</button>
    </section>
  );
}

type SteamGame = {
  appId: string;
  name: string;
  libraryPath: string;
  installDir: string | null;
};

type SteamScanResult = {
  steamDetected: boolean;
  librariesScanned: number;
  games: SteamGame[];
  issues: string[];
};

function SteamLibrary() {
  const [scan, setScan] = useState<SteamScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [scanning, setScanning] = useState(false);
  const desktopAvailable = isControlCenterDesktop();

  const runScan = async () => {
    setScanning(true);
    setError(null);
    try {
      setScan(await invoke<SteamScanResult>("scan_steam_library"));
      setFilter("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setScanning(false);
    }
  };

  const filteredGames = scan?.games.filter((game) =>
    `${game.name} ${game.appId}`.toLowerCase().includes(filter.trim().toLowerCase()),
  ) ?? [];

  return (
    <section className="steam-browser" aria-label="Steam game library">
      <div className="steam-toolbar">
        <div>
          <div className="section-overline">LOCAL LIBRARY SCAN</div>
          <h2>Installed Steam games</h2>
          <p>Reads Steam app manifests from standard Linux library locations. Nothing is launched or changed.</p>
        </div>
        <button
          className="scan-button"
          onClick={runScan}
          disabled={!desktopAvailable || scanning}
          title={desktopAvailable ? "Scan Steam library folders" : "Run the Tauri desktop app to scan local files"}
        >
          {scanning ? <LoaderCircle size={15} className="spin" /> : <RefreshCw size={15} />}
          {scanning ? "SCANNING..." : scan ? "SCAN AGAIN" : "SCAN LIBRARIES"}
        </button>
      </div>
      {!desktopAvailable && (
        <div className="steam-notice">
          <AlertTriangle size={15} />
          <span>Local scanning is available in the Tauri desktop app. Start it with <code>npm run tauri dev</code>.</span>
        </div>
      )}
      {error && <div className="steam-error" role="alert"><AlertTriangle size={15} />{error}</div>}
      {scan && (
        <div className="steam-results" aria-live="polite">
          <div className="steam-results-bar">
            <div className={`steam-detection ${scan.steamDetected ? "detected" : ""}`}>
              <span /> {scan.steamDetected ? "STEAM DETECTED" : "STEAM NOT FOUND"}
            </div>
            <div className="steam-result-count">
              <strong>{scan.games.length}</strong> GAMES
              <i />
              <strong>{scan.librariesScanned}</strong> LIBRARIES SCANNED
            </div>
            {scan.games.length > 0 && (
              <label className="game-filter">
                <Search size={14} />
                <input
                  type="search"
                  value={filter}
                  onChange={(event) => setFilter(event.target.value)}
                  placeholder="Filter games..."
                  aria-label="Filter Steam games"
                />
              </label>
            )}
          </div>
          {!scan.steamDetected ? (
            <div className="game-empty">
              <Gamepad2 size={21} />
              <div><strong>Steam was not found</strong><span>Checked standard Linux Steam install locations for this user.</span></div>
            </div>
          ) : scan.games.length === 0 ? (
            <div className="game-empty">
              <Gamepad2 size={21} />
              <div><strong>No installed games found</strong><span>Steam was detected, but no installed app manifests were found in its libraries.</span></div>
            </div>
          ) : filteredGames.length === 0 ? (
            <div className="game-empty">
              <Search size={18} />
              <div><strong>No matching games</strong><span>Try a different title or app ID.</span></div>
            </div>
          ) : (
            <div className="steam-game-list">
              {filteredGames.map((game) => (
                <article className="steam-game-card" key={game.appId}>
                  <div className="steam-game-icon"><Gamepad2 size={18} /></div>
                  <div className="steam-game-details">
                    <h3>{game.name}</h3>
                    <span>APP ID {game.appId}{game.installDir ? ` · ${game.installDir}` : ""}</span>
                    <small title={game.libraryPath}>{game.libraryPath}</small>
                  </div>
                  <span className="installed-tag">INSTALLED</span>
                </article>
              ))}
            </div>
          )}
          {scan.issues.length > 0 && (
            <details className="steam-issues">
              <summary><AlertTriangle size={13} /> {scan.issues.length} scan warning{scan.issues.length === 1 ? "" : "s"}</summary>
              <ul>{scan.issues.map((issue, index) => <li key={`${index}-${issue}`}>{issue}</li>)}</ul>
            </details>
          )}
        </div>
      )}
    </section>
  );
}

type LauncherApp = {
  name: string;
  executable: string;
};

type LaunchResult = {
  processId: number;
};

function AppLauncher() {
  const [apps, setApps] = useState<LauncherApp[]>([]);
  const [name, setName] = useState("");
  const [executable, setExecutable] = useState("");
  const [pendingLaunch, setPendingLaunch] = useState<LauncherApp | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);
  const desktopAvailable = isControlCenterDesktop();

  useEffect(() => {
    if (!desktopAvailable) {
      setLoading(false);
      return;
    }

    let active = true;
    invoke<LauncherApp[]>("list_launcher_apps")
      .then((entries) => {
        if (active) setApps(entries);
      })
      .catch((reason: unknown) => {
        if (active) setLoadError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [desktopAvailable]);

  const handleAdd = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSaving(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await invoke<LauncherApp[]>("add_launcher_app", { name, executable });
      setApps(updated);
      setName("");
      setExecutable("");
      setNotice("App saved to your local launcher.");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  };

  const handleRemove = async (entry: LauncherApp) => {
    setRemoving(entry.executable);
    setError(null);
    setNotice(null);
    try {
      setApps(await invoke<LauncherApp[]>("remove_launcher_app", { executable: entry.executable }));
      setNotice(`${entry.name} was removed from your launcher.`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setRemoving(null);
    }
  };

  const confirmLaunch = async () => {
    if (!pendingLaunch) return;
    setLaunching(true);
    setLaunchError(null);
    try {
      const result = await invoke<LaunchResult>("launch_launcher_app", {
        executable: pendingLaunch.executable,
      });
      setNotice(`${pendingLaunch.name} started (process ${result.processId}).`);
      setPendingLaunch(null);
    } catch (reason) {
      setLaunchError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLaunching(false);
    }
  };

  return (
    <section className="launcher-workspace" aria-label="App launcher">
      <div className="launcher-add-panel">
        <div className="launcher-panel-heading">
          <div>
            <div className="section-overline">LOCAL APP SHORTCUTS</div>
            <h2>Add an application</h2>
            <p>Save a full executable path. The app starts directly without a shell or custom arguments.</p>
          </div>
          <div className="launcher-add-icon"><Plus size={18} /></div>
        </div>
        <form className="launcher-add-form" onSubmit={handleAdd}>
          <label>
            <span>DISPLAY NAME</span>
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Firefox"
              maxLength={80}
              required
              disabled={!desktopAvailable || saving}
            />
          </label>
          <label>
            <span>EXECUTABLE PATH</span>
            <input
              value={executable}
              onChange={(event) => setExecutable(event.target.value)}
              placeholder="/usr/bin/firefox"
              autoComplete="off"
              required
              disabled={!desktopAvailable || saving}
            />
          </label>
          <button className="launcher-save-button" type="submit" disabled={!desktopAvailable || saving}>
            {saving ? <LoaderCircle size={14} className="spin" /> : <Plus size={14} />}
            {saving ? "SAVING..." : "ADD APP"}
          </button>
        </form>
        {!desktopAvailable && (
          <div className="steam-notice">
            <AlertTriangle size={15} />
            <span>App management is available in the Tauri desktop app. Start it with <code>npm run tauri dev</code>.</span>
          </div>
        )}
      </div>

      {error && <div className="steam-error" role="alert"><AlertTriangle size={15} />{error}</div>}
      {loadError && <div className="steam-error" role="alert"><AlertTriangle size={15} />{loadError}</div>}
      {notice && <div className="launcher-notice" role="status"><CheckCircle2 size={15} />{notice}</div>}

      <div className="launcher-list-panel">
        <div className="launcher-list-heading">
          <div>
            <div className="section-overline">YOUR SHORTCUTS</div>
            <h2>Saved applications</h2>
          </div>
          <span className="launcher-count">{String(apps.length).padStart(2, "0")} APPS</span>
        </div>
        {loading ? (
          <div className="launcher-empty"><LoaderCircle size={17} className="spin" /> Loading saved apps...</div>
        ) : !desktopAvailable ? (
          <div className="launcher-empty"><AlertTriangle size={17} /> Open the Tauri desktop app to load saved applications.</div>
        ) : loadError ? (
          <div className="launcher-empty"><AlertTriangle size={17} /> Saved apps could not be loaded. Resolve the error above and reopen this module.</div>
        ) : apps.length === 0 ? (
          <div className="launcher-empty">
            <Blocks size={19} />
            <span>No apps saved yet. Add an executable above to create your first shortcut.</span>
          </div>
        ) : (
          <div className="launcher-app-list">
            {apps.map((entry) => (
              <article className="launcher-app-card" key={entry.executable}>
                <div className="launcher-app-icon"><Blocks size={18} /></div>
                <div className="launcher-app-details">
                  <h3>{entry.name}</h3>
                  <code title={entry.executable}>{entry.executable}</code>
                </div>
                <button
                  className="launcher-run-button"
                  type="button"
                  disabled={!desktopAvailable || removing === entry.executable}
                  onClick={() => {
                    setPendingLaunch(entry);
                    setLaunchError(null);
                    setError(null);
                    setNotice(null);
                  }}
                >
                  <Play size={13} fill="currentColor" /> LAUNCH
                </button>
                <button
                  className="launcher-remove-button"
                  type="button"
                  aria-label={`Remove ${entry.name}`}
                  title={`Remove ${entry.name}`}
                  disabled={!desktopAvailable || removing === entry.executable}
                  onClick={() => handleRemove(entry)}
                >
                  {removing === entry.executable ? <LoaderCircle size={14} className="spin" /> : <Trash2 size={14} />}
                </button>
              </article>
            ))}
          </div>
        )}
      </div>

      {pendingLaunch && (
        <div className="launcher-dialog-backdrop">
          <section className="launcher-dialog" role="dialog" aria-modal="true" aria-labelledby="launch-dialog-title">
            <div className="section-overline">USER CONFIRMATION REQUIRED</div>
            <h2 id="launch-dialog-title">Launch {pendingLaunch.name}?</h2>
            <p>This starts the saved executable directly. No shell is used.</p>
            <code title={pendingLaunch.executable}>{pendingLaunch.executable}</code>
            {launchError && <div className="steam-error" role="alert"><AlertTriangle size={15} />{launchError}</div>}
            <div className="launcher-dialog-actions">
              <button
                className="launcher-cancel-button"
                type="button"
                disabled={launching}
                onClick={() => setPendingLaunch(null)}
              >
                CANCEL
              </button>
              <button
                className="launcher-run-button"
                type="button"
                disabled={launching}
                onClick={confirmLaunch}
              >
                {launching ? <LoaderCircle size={14} className="spin" /> : <Play size={13} fill="currentColor" />}
                {launching ? "STARTING..." : "CONFIRM LAUNCH"}
              </button>
            </div>
          </section>
        </div>
      )}
    </section>
  );
}

export default App;
