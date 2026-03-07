import { useEffect, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { ShieldAlert, Unlock } from "lucide-react";
import { api } from "../lib/tauri";

export function GuardOverlay() {
  const [app, setApp] = useState<string>("wybrana aplikacja");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    const sync = async () => {
      const runtime = await api.getSecurityRuntime().catch(() => null);
      if (!runtime?.locked) {
        await win.close().catch(() => undefined);
        return;
      }
      setApp(runtime.lockedApp ?? "chroniona aplikacja");
      if (runtime.overlayBounds) {
        const { x, y, width, height } = runtime.overlayBounds;
        await win.setPosition(new LogicalPosition(x, y)).catch(() => undefined);
        await win.setSize(new LogicalSize(width, height)).catch(() => undefined);
      }
    };
    sync().catch(() => undefined);
    const timer = window.setInterval(() => sync().catch(() => undefined), 350);
    return () => window.clearInterval(timer);
  }, []);

  const unlock = async () => {
    setBusy(true);
    const ok = await api.unlockGuard(password.trim());
    setBusy(false);
    if (ok) {
      setError("");
      await getCurrentWebviewWindow().close().catch(() => undefined);
      return;
    }
    setError("Hasło niepoprawne.");
  };

  return (
    <div className="guard-overlay app-window-guard">
      <div className="guard-backdrop" />
      <div className="guard-frame-glow" />
      <div className="guard-card guard-card-windowfit">
        <div className="guard-icon"><ShieldAlert size={34} /></div>
        <div className="eyebrow">window-bound secure overlay</div>
        <h2>{app} wymaga odblokowania</h2>
        <p>
          Panel jest przywiązany do aktualnych wymiarów chronionego okna i odświeża pozycję w trakcie zmiany rozmiaru.
        </p>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Wpisz hasło"
          onKeyDown={(e) => {
            if (e.key === "Enter") unlock().catch?.(() => undefined);
          }}
        />
        {error ? <div className="inline-error">{error}</div> : null}
        <button className="primary-button" disabled={busy} onClick={() => unlock().catch(() => undefined)}>
          <Unlock size={16} />
          {busy ? "Sprawdzam..." : "Odblokuj"}
        </button>
      </div>
    </div>
  );
}
