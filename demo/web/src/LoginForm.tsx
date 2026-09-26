import { useState, type FormEvent } from 'react'
import { decodeJwt, useSession, type AuthUser } from './session.tsx'

type ErrorResponse = { error: string }

const inputClassName =
  'mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60'
const primaryButtonClassName =
  'rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const secondaryButtonClassName =
  'rounded-full border border-ring/80 bg-paper px-3 py-1.5 text-sm font-semibold tracking-wide text-ink transition hover:bg-ring/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const panelClassName =
  'w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]'

async function readError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as ErrorResponse
    if (body.error) return body.error
  } catch {
    // ignore parse errors
  }
  return `Request failed (${response.status})`
}

export function LoginForm() {
  const { api, establish } = useSession()
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [alias, setAlias] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [status, setStatus] = useState<string | null>(null)
  const [activationToken, setActivationToken] = useState<string | null>(null)

  async function handleRegister(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/register', {
        method: 'POST',
        body: JSON.stringify({
          email,
          password,
          alias: alias.trim() || undefined,
        }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as {
        activation_token: string
        result: AuthUser
      }
      setActivationToken(body.activation_token)
      setStatus(`Registered ${body.result.email}. Activate the email token, then log in.`)
      setMode('login')
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleActivate() {
    if (!activationToken) return
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/activate', {
        method: 'POST',
        body: JSON.stringify({ token: activationToken }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      setActivationToken(null)
      setStatus('Email verified. You can log in.')
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleLogin(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as {
        access_token: string
        user: AuthUser
      }
      setPassword('')
      establish(body.access_token, body.user)
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={panelClassName} aria-labelledby="login-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="login-heading" className="text-sm font-semibold tracking-wide text-ink">
          {mode === 'register' ? 'Register' : 'Log in'}
        </h2>
        <button
          type="button"
          className={secondaryButtonClassName}
          onClick={() => {
            setMode(mode === 'register' ? 'login' : 'register')
            setError(null)
            setStatus(null)
          }}
          disabled={busy}
        >
          {mode === 'register' ? 'Have an account?' : 'Need an account?'}
        </button>
      </div>

      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}
      {status ? <p className="mt-3 text-sm text-ink-muted">{status}</p> : null}

      <form onSubmit={mode === 'register' ? handleRegister : handleLogin} className="mt-4">
        <div className="space-y-4">
          <label className="block text-sm text-ink-muted">
            Email
            <input
              type="email"
              name="email"
              autoComplete="username"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className={inputClassName}
              disabled={busy}
            />
          </label>
          {mode === 'register' ? (
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
      </form>

      {activationToken ? (
        <div className="mt-5 border-t border-ring/50 pt-5">
          <p className="text-sm text-ink-muted">Demo activation token (no mailer in v1):</p>
          <p className="mt-2 break-all font-mono text-xs text-ink">{activationToken}</p>
          <button
            type="button"
            className={`mt-4 w-full ${primaryButtonClassName}`}
            onClick={() => void handleActivate()}
            disabled={busy}
          >
            Activate email
          </button>
        </div>
      ) : null}
    </section>
  )
}

export function TokenPanel() {
  const { accessToken, user, logout } = useSession()
  const decoded = accessToken ? decodeJwt(accessToken) : null

  return (
    <section className={panelClassName} aria-labelledby="token-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="token-heading" className="text-sm font-semibold tracking-wide text-ink">
          Session
        </h2>
        <button type="button" className={secondaryButtonClassName} onClick={() => void logout()}>
          Log out
        </button>
      </div>
      <p className="mt-4 text-xs tracking-wide text-ink-muted">Public user id</p>
      <p className="mt-1 break-all font-mono text-sm text-ink">{user?.public_id}</p>
      <p className="mt-4 text-xs tracking-wide text-ink-muted">Access token</p>
      <p className="mt-1 break-all font-mono text-xs text-ink">{accessToken}</p>
      <p className="mt-4 text-xs tracking-wide text-ink-muted">Decoded header</p>
      <pre className="mt-1 overflow-x-auto font-mono text-xs text-ink">
        {JSON.stringify(decoded?.header, null, 2)}
      </pre>
      <p className="mt-4 text-xs tracking-wide text-ink-muted">Decoded payload</p>
      <pre className="mt-1 overflow-x-auto font-mono text-xs text-ink">
        {JSON.stringify(decoded?.payload, null, 2)}
      </pre>
    </section>
  )
}

export function LogoutButton() {
  const { logout } = useSession()
  return (
    <button type="button" className={secondaryButtonClassName} onClick={() => void logout()}>
      Log out
    </button>
  )
}
