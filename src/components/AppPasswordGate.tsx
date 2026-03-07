import { useState } from "react";
import { LockKeyhole } from "lucide-react";

export function AppPasswordGate({
  onSubmit,
  title,
  subtitle
}: {
  onSubmit: (password: string) => Promise<boolean>;
  title: string;
  subtitle: string;
}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setBusy(true);
    const ok = await onSubmit(password);
    setBusy(false);
    if (!ok) {
      setError("Hasło niepoprawne.");
      return;
    }
    setError("");
  };

  return (
    <div className="guard-overlay app-gate">
      <div className="guard-backdrop" />
      <div className="guard-card">
        <div className="guard-icon"><LockKeyhole size={34} /></div>
        <div className="eyebrow">launch protection</div>
        <h2>{title}</h2>
        <p>{subtitle}</p>
        <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Wpisz hasło" />
        {error ? <div className="inline-error">{error}</div> : null}
        <button className="primary-button" disabled={busy} onClick={submit}>{busy ? "Sprawdzam..." : "Odblokuj"}</button>
      </div>
    </div>
  );
}
