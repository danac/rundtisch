import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useSession, type AuthUser } from './session.tsx'
import {
  conditionalMediationAvailable,
  createPasskey,
  getPasskey,
  passkeySupported,
} from './webauthn.ts'

type ErrorResponse = { error: string }
type CeremonyStart = { flow_id: string; options: unknown }
type PasskeyRow = {
  public_id: string
  label: string | null
  created_at: string
  last_used_at: string | null
}

const inputClassName =
  'mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60'
const primaryButtonClassName =
  'rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const secondaryButtonClassName =
  'rounded-full border border-ring/80 bg-paper px-3 py-1.5 text-sm font-semibold tracking-wide text-ink transition hover:bg-ring/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const panelClassName =
  'w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]'

const query = new URLSearchParams(window.location.search)
const inviteFromQuery = query.get('invite') ?? ''
const recoverFromQuery = query.get('recover') ?? ''

async function readError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as ErrorResponse
    if (body.error) return body.error
  } catch {
    // ignore parse errors
  }
  return `Request failed (${response.status})`
}

function clearAuthQuery() {
  window.history.replaceState({}, '', window.location.pathname)
}

function messageFrom(err: unknown): string {
  if (err instanceof DOMException && err.name === 'NotAllowedError') return 'Passkey cancelled'
  if (err instanceof Error) return err.message
  return 'Network error'
}

function ignoredPasskeyError(err: unknown): boolean {
  return (
    err instanceof DOMException && (err.name === 'AbortError' || err.name === 'NotAllowedError')
  )
}

export function LoginForm() {
  const { api, establish } = useSession()
  const recovering = recoverFromQuery.length > 0
  const [mode, setMode] = useState<'login' | 'register'>(inviteFromQuery ? 'register' : 'login')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [alias, setAlias] = useState('')
  const [passkeyLabel, setPasskeyLabel] = useState('')
  const [inviteToken, setInviteToken] = useState(inviteFromQuery)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const canPasskey = passkeySupported()
  const conditionalAbort = useRef<AbortController | null>(null)

  function begin() {
    setBusy(true)
    setError(null)
  }

  async function finishSession(response: Response) {
    if (!response.ok) {
      setError(await readError(response))
      return
    }
    const body = (await response.json()) as { user: AuthUser }
    setEmail(body.user.email)
    setPassword('')
    clearAuthQuery()
    establish(body.user)
  }

  useEffect(() => {
    if (mode !== 'login' || recovering || !canPasskey) return
    const controller = new AbortController()
    conditionalAbort.current = controller
    let cancelled = false
    void (async () => {
      if (!(await conditionalMediationAvailable()) || controller.signal.aborted) return
      try {
        const started = await api('/api/auth/passkeys/login/options', {
          method: 'POST',
          body: '{}',
          signal: controller.signal,
        })
        if (!started.ok || cancelled) return
        const ceremony = (await started.json()) as CeremonyStart
        const credential = await getPasskey(ceremony.options, {
          mediation: 'conditional',
          signal: controller.signal,
        })
        if (cancelled) return
        const finished = await api('/api/auth/passkeys/login', {
          method: 'POST',
          body: JSON.stringify({ flow_id: ceremony.flow_id, credential }),
        })
        if (cancelled) return
        await finishSession(finished)
      } catch (err) {
        if (cancelled || controller.signal.aborted || ignoredPasskeyError(err)) return
        setError(messageFrom(err))
      }
    })()
    return () => {
      cancelled = true
      controller.abort()
      if (conditionalAbort.current === controller) conditionalAbort.current = null
    }
  }, [mode, recovering, canPasskey])

  async function handleLogin(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    conditionalAbort.current?.abort()
    begin()
    const loginEmail =
      String(new FormData(event.currentTarget).get('email') ?? '').trim() || email.trim()
    try {
      const response = await api('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email: loginEmail, password }),
      })
      await finishSession(response)
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function handleRegister(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    begin()
    try {
      const response = await api('/api/auth/register/password', {
        method: 'POST',
        body: JSON.stringify({
          token: inviteToken.trim(),
          password,
          alias: alias.trim() || undefined,
        }),
      })
      await finishSession(response)
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function handleRecoverPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    begin()
    try {
      const response = await api('/api/auth/reset', {
        method: 'POST',
        body: JSON.stringify({ token: recoverFromQuery, password }),
      })
      await finishSession(response)
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function runCeremony(
    optionsPath: string,
    finishPath: string,
    body: unknown,
    kind: 'create' | 'get',
  ) {
    begin()
    try {
      const started = await api(optionsPath, {
        method: 'POST',
        body: JSON.stringify(body),
      })
      if (!started.ok) {
        setError(await readError(started))
        return
      }
      const ceremony = (await started.json()) as CeremonyStart
      const credential =
        kind === 'create' ? await createPasskey(ceremony.options) : await getPasskey(ceremony.options)
      const finished = await api(finishPath, {
        method: 'POST',
        body: JSON.stringify({ flow_id: ceremony.flow_id, credential }),
      })
      await finishSession(finished)
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  const heading = recovering ? 'Set a new credential' : mode === 'register' ? 'Register' : 'Log in'

  return (
    <section className={panelClassName} aria-labelledby="login-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="login-heading" className="text-sm font-semibold tracking-wide text-ink">
          {heading}
        </h2>
        {recovering ? null : (
          <button
            type="button"
            className={secondaryButtonClassName}
            onClick={() => {
              setMode(mode === 'register' ? 'login' : 'register')
              setError(null)
            }}
            disabled={busy}
          >
            {mode === 'register' ? 'Have an account?' : 'Have an invitation?'}
          </button>
        )}
      </div>

      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}

      {recovering ? (
        <form onSubmit={(event) => void handleRecoverPassword(event)} className="mt-4">
          <p className="text-sm text-ink-muted">Choose a new password or a passkey for this account.</p>
          <label className="mt-4 block text-sm text-ink-muted">
            New password
            <input
              type="password"
              name="password"
              autoComplete="new-password"
              required
              minLength={15}
              maxLength={256}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className={inputClassName}
              disabled={busy}
            />
          </label>
          <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
            Save password
          </button>
          <label className="mt-4 block text-sm text-ink-muted">
            Passkey label (optional)
            <input
              type="text"
              maxLength={64}
              value={passkeyLabel}
              onChange={(event) => setPasskeyLabel(event.target.value)}
              placeholder="MacBook Touch ID"
              className={inputClassName}
              disabled={busy}
            />
          </label>
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={busy || !canPasskey}
            onClick={() =>
              void runCeremony(
                '/api/auth/reset/passkey/options',
                '/api/auth/reset/passkey',
                { token: recoverFromQuery, label: passkeyLabel.trim() || undefined },
                'create',
              )
            }
          >
            {canPasskey ? 'Use a passkey instead' : 'Passkeys are not available in this browser'}
          </button>
        </form>
      ) : (
        <form
          onSubmit={mode === 'register' ? handleRegister : handleLogin}
          className="mt-4"
        >
          <div className="space-y-4">
            {mode === 'register' ? (
              <label className="block text-sm text-ink-muted">
                Invitation
                <input
                  type="text"
                  name="invitation"
                  autoComplete="off"
                  required
                  value={inviteToken}
                  onChange={(event) => setInviteToken(event.target.value)}
                  className={inputClassName}
                  disabled={busy}
                />
              </label>
            ) : (
              <label className="block text-sm text-ink-muted">
                Email
                <input
                  type="email"
                  name="email"
                  autoComplete="username webauthn"
                  required
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  className={inputClassName}
                  disabled={busy}
                />
              </label>
            )}
            {mode === 'register' ? (
              <>
                <label className="block text-sm text-ink-muted">
                  Alias
                  <input
                    type="text"
                    name="alias"
                    autoComplete="nickname"
                    value={alias}
                    onChange={(event) => setAlias(event.target.value)}
                    className={inputClassName}
                    disabled={busy}
                  />
                </label>
                <label className="block text-sm text-ink-muted">
                  Passkey label (optional)
                  <input
                    type="text"
                    maxLength={64}
                    value={passkeyLabel}
                    onChange={(event) => setPasskeyLabel(event.target.value)}
                    placeholder="MacBook Touch ID"
                    className={inputClassName}
                    disabled={busy}
                  />
                </label>
              </>
            ) : null}
            <label className="block text-sm text-ink-muted">
              Password
              <input
                type="password"
                name="password"
                autoComplete={mode === 'register' ? 'new-password' : 'current-password'}
                required
                minLength={15}
                maxLength={256}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                className={inputClassName}
                disabled={busy}
              />
            </label>
          </div>
          <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
            {mode === 'register' ? 'Register' : 'Log in'}
          </button>
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={busy || !canPasskey || (mode === 'register' && inviteToken.trim() === '')}
            onClick={() => {
              if (mode === 'register') {
                void runCeremony(
                  '/api/auth/register/passkey/options',
                  '/api/auth/register/passkey',
                  {
                    token: inviteToken.trim(),
                    alias: alias.trim() || undefined,
                    label: passkeyLabel.trim() || undefined,
                  },
                  'create',
                )
                return
              }
              conditionalAbort.current?.abort()
              void runCeremony(
                '/api/auth/passkeys/login/options',
                '/api/auth/passkeys/login',
                {},
                'get',
              )
            }}
          >
            {canPasskey
              ? mode === 'register'
                ? 'Register with a passkey'
                : 'Log in with a passkey'
              : 'Passkeys are not available in this browser'}
          </button>
        </form>
      )}
    </section>
  )
}

export function AccountPanel() {
  const { api, user, logout, logoutAll } = useSession()
  const [passkeys, setPasskeys] = useState<PasskeyRow[]>([])
  const [passkeyLabel, setPasskeyLabel] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const canPasskey = passkeySupported()

  async function loadPasskeys() {
    const response = await api('/api/auth/passkeys')
    if (!response.ok) {
      setError(await readError(response))
      return
    }
    const body = (await response.json()) as { passkeys: PasskeyRow[] }
    setPasskeys(body.passkeys)
  }

  useEffect(() => {
    void loadPasskeys()
  }, [])

  async function addPasskey() {
    setBusy(true)
    setError(null)
    try {
      const started = await api('/api/auth/passkeys/register/options', {
        method: 'POST',
        body: JSON.stringify({ label: passkeyLabel.trim() || undefined }),
      })
      if (!started.ok) {
        setError(await readError(started))
        return
      }
      const ceremony = (await started.json()) as CeremonyStart
      const credential = await createPasskey(ceremony.options)
      const finished = await api('/api/auth/passkeys/register', {
        method: 'POST',
        body: JSON.stringify({ flow_id: ceremony.flow_id, credential }),
      })
      if (!finished.ok) {
        setError(await readError(finished))
        return
      }
      setPasskeyLabel('')
      await loadPasskeys()
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function removePasskey(publicId: string) {
    setBusy(true)
    setError(null)
    try {
      const response = await api(`/api/auth/passkeys/${publicId}`, { method: 'DELETE' })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      await loadPasskeys()
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={panelClassName} aria-labelledby="account-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="account-heading" className="text-sm font-semibold tracking-wide text-ink">
          Account
        </h2>
        <div className="flex gap-2">
          <button type="button" className={secondaryButtonClassName} onClick={() => void logoutAll()}>
            Log out everywhere
          </button>
          <button type="button" className={secondaryButtonClassName} onClick={() => void logout()}>
            Log out
          </button>
        </div>
      </div>
      <p className="mt-4 text-sm text-ink">{user?.alias}</p>
      <p className="mt-1 text-sm text-ink-muted">{user?.email}</p>
      <p className="mt-1 break-all font-mono text-xs text-ink-muted">{user?.public_id}</p>
      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}
      <div className="mt-5 border-t border-ring/50 pt-5">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold tracking-wide text-ink">Passkeys</h3>
          <div className="flex items-end gap-2">
            <label className="text-xs text-ink-muted">
              Label (optional)
              <input
                type="text"
                maxLength={64}
                value={passkeyLabel}
                onChange={(event) => setPasskeyLabel(event.target.value)}
                placeholder="MacBook Touch ID"
                className={`${inputClassName} min-w-48`}
                disabled={busy}
              />
            </label>
            <button
              type="button"
              className={secondaryButtonClassName}
              disabled={busy || !canPasskey}
              onClick={() => void addPasskey()}
            >
              Add
            </button>
          </div>
        </div>
        {passkeys.length === 0 ? (
          <p className="mt-3 text-sm text-ink-muted">No passkeys yet.</p>
        ) : (
          <ul className="mt-3 space-y-2">
            {passkeys.map((passkey) => (
              <li
                key={passkey.public_id}
                className="flex items-center justify-between gap-3 text-sm"
              >
                <span>
                  <span className="block text-ink">{passkey.label || 'Passkey'}</span>
                  <span className="block text-xs text-ink-muted">
                    Added {passkey.created_at.slice(0, 10)}
                  </span>
                </span>
                <button
                  type="button"
                  className={secondaryButtonClassName}
                  disabled={busy}
                  onClick={() => void removePasskey(passkey.public_id)}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
