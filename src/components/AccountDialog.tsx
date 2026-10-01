import { useEffect, useState, type FormEvent } from "react";
import { api, type AuthStatus } from "../api";
import { longDate } from "../format";
import { Dialog } from "./Dialog";
import { LANGUAGES, useI18n, type Lang } from "../i18n";

interface AccountDialogProps {
  auth: AuthStatus;
  onClose: () => void;
  /** Called after anything that changes the account or settings. */
  onAuthChanged: () => Promise<void>;
  onSignOut: () => void;
}

export function AccountDialog({ auth, onClose, onAuthChanged, onSignOut }: AccountDialogProps) {
  const [showSettings, setShowSettings] = useState(!auth.configured);
  const { t, locale } = useI18n();

  return (
    <Dialog title={auth.email ? t.account : t.syncWithSupabase} onClose={onClose}>
      {auth.email ? (
        <div className="account-info">
          <div className="field-static">
            <span className="field-label">{t.signedInAs}</span>
            <strong>{auth.email}</strong>
          </div>
          <div className="field-static">
            <span className="field-label">{t.lastSync}</span>
            <span>{auth.lastSyncAt ? longDate(auth.lastSyncAt, locale) : t.notSynced}</span>
          </div>
          <div className="dialog-actions">
            <button className="btn" onClick={onSignOut}>
              {t.signOut}
            </button>
            <button className="btn btn-primary" onClick={onClose}>
              {t.done}
            </button>
          </div>
        </div>
      ) : (
        <>
          {auth.configured && !showSettings && (
            <SignInForm
              onDone={async () => {
                await onAuthChanged();
                onClose();
              }}
            />
          )}
          {(showSettings || !auth.configured) && (
            <SettingsForm
              onSaved={async () => {
                await onAuthChanged();
                setShowSettings(false);
              }}
            />
          )}
          {auth.configured && (
            <button className="link-btn" onClick={() => setShowSettings((s) => !s)}>
              {showSettings ? t.backToSignIn : t.configureSupabase}
            </button>
          )}
        </>
      )}
      <LanguagePicker />
    </Dialog>
  );
}

function SignInForm({ onDone }: { onDone: () => Promise<void> }) {
  const [mode, setMode] = useState<"signIn" | "signUp">("signIn");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const { t, errorText } = useI18n();

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setInfo(null);
    try {
      if (mode === "signIn") {
        await api.signIn(email, password);
        await onDone();
      } else {
        const res = await api.signUp(email, password);
        if (res.kind === "signedIn") await onDone();
        else {
          setInfo(t.confirmSent(res.email));
          setMode("signIn");
        }
      }
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form className="form" onSubmit={submit}>
      <p className="dialog-text">
        {t.signInIntro}
      </p>
      <label className="field">
        <span className="field-label">{t.email}</span>
        <input type="email" required autoFocus value={email} onChange={(e) => setEmail(e.target.value)} />
      </label>
      <label className="field">
        <span className="field-label">{t.password}</span>
        <input
          type="password"
          required
          minLength={6}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </label>
      {error && <p className="form-error">{error}</p>}
      {info && <p className="form-info">{info}</p>}
      <div className="dialog-actions">
        <button
          type="button"
          className="link-btn"
          onClick={() => {
            setMode(mode === "signIn" ? "signUp" : "signIn");
            setError(null);
          }}
        >
          {mode === "signIn" ? t.noAccount : t.haveAccount}
        </button>
        <button type="submit" className="btn btn-primary" disabled={busy}>
          {busy ? t.working : mode === "signIn" ? t.signIn : t.signUp}
        </button>
      </div>
    </form>
  );
}

function SettingsForm({ onSaved }: { onSaved: () => Promise<void> }) {
  const [url, setUrl] = useState("");
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const { t, errorText } = useI18n();

  useEffect(() => {
    api.getSettings().then((s) => {
      setUrl(s.supabaseUrl);
      setKey(s.supabaseAnonKey);
    });
  }, []);

  return (
    <form
      className="form"
      onSubmit={async (e) => {
        e.preventDefault();
        setError(null);
        try {
          await api.saveSettings({ supabaseUrl: url, supabaseAnonKey: key });
          await onSaved();
        } catch (err) {
          setError(errorText(err));
        }
      }}
    >
      <p className="dialog-text">
        {t.settingsIntro}
      </p>
      <label className="field">
        <span className="field-label">{t.projectUrl}</span>
        <input
          required
          placeholder="https://xxxx.supabase.co"
          value={url}
          spellCheck={false}
          onChange={(e) => setUrl(e.target.value)}
        />
      </label>
      <label className="field">
        <span className="field-label">{t.anonKey}</span>
        <input required value={key} spellCheck={false} onChange={(e) => setKey(e.target.value)} />
      </label>
      {error && <p className="form-error">{error}</p>}
      <div className="dialog-actions">
        <button type="submit" className="btn btn-primary">
          {t.save}
        </button>
      </div>
    </form>
  );
}

function LanguagePicker() {
  const { t, lang, setLang } = useI18n();
  return (
    <label className="language-row">
      <span className="field-label">{t.language}</span>
      <select value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
        {LANGUAGES.map((l) => (
          <option key={l.id} value={l.id}>
            {l.label}
          </option>
        ))}
      </select>
    </label>
  );
}
