import { useState, type FormEvent } from 'react'

type AuthUser = {
  public_id: string
  email: string
  alias: string
  role: string
}

type ErrorResponse = { error: string }

const inputClassName =
  'mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60'
const primaryButtonClassName =
  'rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const secondaryButtonClassName =
  'rounded-full border border-ring/80 bg-paper px-3 py-1.5 text-sm font-semibold tracking-wide text-ink transition hover:bg-ring/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'

async function readError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as ErrorResponse
    if (body.error) return body.error
  } catch {
    // ignore parse errors
  }
  return `Request failed (${response.status})`
}

async function api(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers)
  if (init.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }
  return fetch(path, { credentials: 'include', ...init, headers })
}

export function LoginForm() {
  const [mode, setMode] = useState<'login' | 'register'>('register')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [alias, setAlias] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [status, setStatus] = useState<string | null>(null)
  const [activationToken, setActivationToken] = useState<string | null>(null)
  const [accessToken, setAccessToken] = useState<string | null>(null)
  const [user, setUser] = useState<AuthUser | null>(null)

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
      setAccessToken(body.access_token)
      setUser(body.user)
      setPassword('')
      setStatus('Signed in.')
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleMe() {
    if (!accessToken) return
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/me', {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as { result: AuthUser }
      setUser(body.result)
      setStatus(`Authenticated as ${body.result.email}`)
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleRefresh() {
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/refresh', { method: 'POST' })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as {
        access_token: string
        user: AuthUser
      }
      setAccessToken(body.access_token)
      setUser(body.user)
      setStatus('Session refreshed.')
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleLogout() {
    setBusy(true)
    setError(null)
    setStatus(null)
    try {
      const response = await api('/api/auth/logout', { method: 'POST' })
      if (!response.ok && response.status !== 204) {
        setError(await readError(response))
        return
      }
      setAccessToken(null)
      setUser(null)
      setStatus('Signed out.')
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section
      className="w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]"
      aria-labelledby="login-heading"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 id="login-heading" className="text-sm font-semibold tracking-wide text-ink">
          {user ? 'Session' : mode === 'register' ? 'Register' : 'Log in'}
        </h2>
        {user ? null : (
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
        )}
      </div>

      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}
      {status ? <p className="mt-3 text-sm text-ink-muted">{status}</p> : null}

      {user ? (
        <div className="mt-4 space-y-3">
          <p className="text-sm text-ink">
            {user.alias} <span className="text-ink-muted">({user.email})</span>
          </p>
          <p className="truncate font-mono text-xs text-ink-muted" title={user.public_id}>
            {user.public_id}
          </p>
          <div className="flex flex-wrap gap-2">
            <button type="button" className={secondaryButtonClassName} onClick={() => void handleMe()} disabled={busy}>
              /me
            </button>
            <button
              type="button"
              className={secondaryButtonClassName}
              onClick={() => void handleRefresh()}
              disabled={busy}
            >
              Refresh
            </button>
            <button
              type="button"
              className={secondaryButtonClassName}
              onClick={() => void handleLogout()}
              disabled={busy}
            >
              Log out
            </button>
          </div>
        </div>
      ) : (
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
      )}

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
