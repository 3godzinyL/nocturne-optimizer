import { useEffect, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ShieldAlert, Unlock } from "lucide-react";
import { api } from "../lib/tauri";

export function GuardOverlay() {
  const [app, setApp] = useState<string>("wybrana aplikacja");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    const timer = window.setInterval(async () => {
      const runtime = await api.getSecurityRuntime();
      if (!runtime.locked) {
        getCurrentWebviewWindow().close().catch(() => undefined);
        return;
      }
      setApp(runtime.lockedApp ?? "chroniona aplikacja");
    }, 700);
    return () => window.clearInterval(timer);
  }, []);

  const unlock = async () => {
    const ok = await api.unlockGuard(password);
    if (ok) {
      setError("");
      getCurrentWebviewWindow().close().catch(() => undefined);
      return;
    }
    setError("Hasło niepoprawne.");
  };

  return (
    <div className="guard-overlay">
      <div className="guard-backdrop" />
      <div className="guard-card">
        <div className="guard-icon"><ShieldAlert size={34} /></div>
        <div className="eyebrow">secure reopen shield</div>
        <h2>{app} wymaga odblokowania</h2>
        <p>
          To okno wróciło na pierwszy plan po utracie fokusu. Wpisz hasło, aby zdjąć blokadę i wrócić do pracy.
        </p>
        <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Wpisz hasło" />
        {error ? <div className="inline-error">{error}</div> : null}
        <button className="primary-button" onClick={unlock}>
          <Unlock size={16} />
          Odblokuj
        </button>
      </div>
    </div>
  );
}
