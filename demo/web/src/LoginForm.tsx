import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react'
import { useSession, type AuthUser } from './session.tsx'
import {
  conditionalMediationAvailable,
  createPasskey,
  getPasskey,
  passkeySupported,
} from './webauthn.ts'

type ErrorResponse = { error: string }
type CeremonyStart = { flow_id: string; options: unknown }
type SessionRow = {
  public_id: string
  created_at: string
  last_used_at: string
  expires_at: string
  user_agent: string | null
  current: boolean
}

function sessionLabel(userAgent: string | null): string {
  if (!userAgent) return 'This browser'
  if (userAgent.startsWith('cli ')) return userAgent.slice(4)
  return userAgent
}

type PasskeyRow = {
  public_id: string
  label: string | null
  aaguid: string | null
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
  'w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[var(--shadow-panel)] backdrop-blur-[2px]'

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
  const path = window.location.pathname.replace(/\/+$/, '') || '/'
  if (path === '/authorize') return
  window.history.replaceState({}, '', window.location.pathname)
}

function messageFrom(err: unknown): string {
  if (err instanceof DOMException && err.name === 'NotAllowedError') return 'Passkey cancelled'
  if (err instanceof Error) return err.message
  return 'Network error'
}

function readStepUp(slot: { current: { token: string; expiresAt: number } | null }): string | null {
  const held = slot.current
  if (!held || held.expiresAt <= Date.now()) {
    slot.current = null
    return null
  }
  return held.token
}

function storeStepUp(
  slot: { current: { token: string; expiresAt: number } | null },
  token: string,
  expiresIn: number,
) {
  slot.current = { token, expiresAt: Date.now() + Math.max(expiresIn - 2, 1) * 1000 }
}

function accountError(code: string): string {
  if (code === 'invalid_credentials') return 'That credential was not accepted.'
  if (code === 'last_credential') {
    return 'Keep a password or a passkey so this account can still sign in.'
  }
  if (code === 'invalid_password') {
    return 'Use at least 15 characters, and not a common password.'
  }
  return code
}

function NewPasswordField({
  value,
  onChange,
  disabled,
  autoFocus = false,
}: {
  value: string
  onChange: (value: string) => void
  disabled: boolean
  autoFocus?: boolean
}) {
  return (
    <label className="block text-sm text-ink-muted">
      Password
      <input
        type="password"
        name="password"
        autoComplete="new-password"
        autoFocus={autoFocus}
        required
        minLength={15}
        maxLength={256}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={inputClassName}
        disabled={disabled}
      />
    </label>
  )
}

function ignoredPasskeyError(err: unknown): boolean {
  return (
    err instanceof DOMException && (err.name === 'AbortError' || err.name === 'NotAllowedError')
  )
}

function StepSummary({
  label,
  onChange,
  disabled,
}: {
  label: string
  onChange: () => void
  disabled: boolean
}) {
  return (
    <button
      type="button"
      onClick={onChange}
      disabled={disabled}
      className="flex w-full items-center justify-between gap-3 rounded-lg border border-ring/80 bg-paper px-3 py-2 text-left text-sm transition hover:bg-ring/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50"
    >
      <span className="min-w-0 truncate text-ink">{label}</span>
      <span className="shrink-0 font-semibold tracking-wide text-ink-muted">Change</span>
    </button>
  )
}

function OptionalPasskeyName({
  value,
  onChange,
  disabled,
}: {
  value: string
  onChange: (value: string) => void
  disabled: boolean
}) {
  return (
    <label className="block text-sm text-ink-muted">
      Name <span className="font-normal">(optional)</span>
      <input
        type="text"
        maxLength={64}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="MacBook Touch ID"
        className={inputClassName}
        disabled={disabled}
      />
    </label>
  )
}

function PasskeyCreateStep({
  busy,
  canPasskey,
  label,
  onLabelChange,
  onBack,
  backLabel = 'Back',
  onCreate,
  children,
}: {
  busy: boolean
  canPasskey: boolean
  label: string
  onLabelChange: (value: string) => void
  onBack?: () => void
  backLabel?: string
  onCreate: () => void
  children?: ReactNode
}) {
  return (
    <form
      className="mt-4"
      onSubmit={(event) => {
        event.preventDefault()
        onCreate()
      }}
    >
      <div className="space-y-4">
        {children}
        <OptionalPasskeyName value={label} onChange={onLabelChange} disabled={busy} />
      </div>
      <button
        type="submit"
        className={`mt-6 w-full ${primaryButtonClassName}`}
        disabled={busy || !canPasskey}
      >
        {canPasskey ? 'Create passkey' : 'Passkeys are not available in this browser'}
      </button>
      {onBack ? (
        <button
          type="button"
          className={`mt-3 w-full ${secondaryButtonClassName}`}
          onClick={onBack}
          disabled={busy}
        >
          {backLabel}
        </button>
      ) : null}
    </form>
  )
}

export function LoginForm() {
  const { api, establish } = useSession()
  const recovering = recoverFromQuery.length > 0
  const [mode, setMode] = useState<
    'login' | 'register' | 'request_recovery' | 'recovery_sent'
  >(inviteFromQuery ? 'register' : 'login')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [alias, setAlias] = useState('')
  const [passkeyLabel, setPasskeyLabel] = useState('')
  const [inviteToken, setInviteToken] = useState(inviteFromQuery)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [recoveryNotice, setRecoveryNotice] = useState<string | null>(null)
  const [loginPhase, setLoginPhase] = useState<'email' | 'password'>('email')
  const [registerPhase, setRegisterPhase] = useState<
    'checking' | 'token' | 'invalid' | 'alias' | 'method' | 'password' | 'passkey'
  >(inviteFromQuery ? 'checking' : 'token')
  const [recoverPhase, setRecoverPhase] = useState<'method' | 'password' | 'passkey'>('method')
  const canPasskey = passkeySupported()
  const conditionalAbort = useRef<AbortController | null>(null)
  const passwordInputRef = useRef<HTMLInputElement>(null)

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

  const previewGeneration = useRef(0)

  function showLogin() {
    previewGeneration.current += 1
    setMode('login')
    setLoginPhase('email')
    setRegisterPhase('token')
    setInviteToken('')
    setAlias('')
    setRecoverPhase('method')
    setPassword('')
    setPasskeyLabel('')
    setError(null)
    setRecoveryNotice(null)
    clearAuthQuery()
  }

  const previewApi = useRef(api)
  previewApi.current = api

  useEffect(() => {
    if (!inviteFromQuery) return
    const generation = previewGeneration.current + 1
    previewGeneration.current = generation
    setBusy(true)
    void (async () => {
      try {
        const response = await previewApi.current('/api/auth/register/invitation', {
          method: 'POST',
          body: JSON.stringify({ token: inviteFromQuery }),
        })
        if (previewGeneration.current !== generation) return
        setRegisterPhase(response.ok ? 'alias' : 'invalid')
      } catch {
        if (previewGeneration.current === generation) setRegisterPhase('invalid')
      } finally {
        if (previewGeneration.current === generation) setBusy(false)
      }
    })()
  }, [])

  function showRequestRecovery() {
    setMode('request_recovery')
    setRecoverPhase('method')
    setPassword('')
    setPasskeyLabel('')
    setError(null)
    setRecoveryNotice(null)
  }

  async function handleRequestRecovery(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    begin()
    setRecoveryNotice(null)
    try {
      const recoveryEmail =
        String(new FormData(event.currentTarget).get('email') ?? '').trim() || email.trim()
      setEmail(recoveryEmail)
      const response = await api('/api/auth/request_reset', {
        method: 'POST',
        body: JSON.stringify({ email: recoveryEmail }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as { message?: string }
      setRecoveryNotice(
        body.message ?? 'If that account exists, a recovery link has been sent.',
      )
      setMode('recovery_sent')
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  useEffect(() => {
    if (mode !== 'login' || recovering || !canPasskey || loginPhase !== 'email') return
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
        if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
      }
    })()
    return () => {
      cancelled = true
      controller.abort()
      if (conditionalAbort.current === controller) conditionalAbort.current = null
    }
  }, [mode, recovering, canPasskey, loginPhase])

  useEffect(() => {
    if (mode === 'login' && loginPhase === 'password') {
      passwordInputRef.current?.focus()
    }
  }, [mode, loginPhase])

  function showEmailStep() {
    setLoginPhase('email')
    setPassword('')
    setError(null)
  }

  function handleEmailNext(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const loginEmail =
      String(new FormData(event.currentTarget).get('email') ?? '').trim() || email.trim()
    setEmail(loginEmail)
    setPassword('')
    setError(null)
    setLoginPhase('password')
  }

  function startPasskeyLogin() {
    conditionalAbort.current?.abort()
    void runCeremony('/api/auth/passkeys/login/options', '/api/auth/passkeys/login', {}, 'get')
  }

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
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function handleInvitation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    begin()
    try {
      const response = await api('/api/auth/register/invitation', {
        method: 'POST',
        body: JSON.stringify({ token: inviteToken.trim() }),
      })
      if (!response.ok) {
        const message = await readError(response)
        if (response.status === 401 || message === 'invalid_token') {
          setRegisterPhase('invalid')
          return
        }
        setError(message)
        return
      }
      setRegisterPhase('alias')
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  function handleAlias(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const next = alias.trim()
    if (!next || next.length > 128) {
      setError('Enter an alias of at most 128 characters.')
      return
    }
    setAlias(next)
    setError(null)
    setRegisterPhase('method')
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
          alias: alias.trim(),
        }),
      })
      await finishSession(response)
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
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
      if (err instanceof Error && err.message === 'cancelled') return
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
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  const heading = recovering
    ? 'Set a new credential'
    : mode === 'register'
      ? registerPhase === 'token' || registerPhase === 'checking' || registerPhase === 'invalid'
        ? 'Invitation'
        : registerPhase === 'alias'
          ? 'Alias'
          : 'Register'
      : mode === 'request_recovery' || mode === 'recovery_sent'
        ? 'Recover account'
        : 'Log in'

  return (
    <section className={panelClassName} aria-labelledby="login-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="login-heading" className="text-sm font-semibold tracking-wide text-ink">
          {heading}
        </h2>
        {recovering ? null : mode === 'request_recovery' || mode === 'recovery_sent' ? (
          <button
            type="button"
            className={secondaryButtonClassName}
            onClick={showLogin}
            disabled={busy}
          >
            Back to login
          </button>
        ) : (
          <button
            type="button"
            className={secondaryButtonClassName}
            onClick={() => {
              if (mode === 'register') {
                showLogin()
                return
              }
              setMode('register')
              setLoginPhase('email')
              setRegisterPhase('token')
              setRecoverPhase('method')
              setPassword('')
              setPasskeyLabel('')
              setError(null)
              setRecoveryNotice(null)
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
        recoverPhase === 'method' ? (
          <div className="mt-4">
            <p className="text-sm text-ink-muted">
              Choose a new password or a passkey for this account.
            </p>
            <button
              type="button"
              className={`mt-6 w-full ${primaryButtonClassName}`}
              disabled={busy}
              onClick={() => {
                setError(null)
                setRecoverPhase('password')
              }}
            >
              Set a password
            </button>
            <button
              type="button"
              className={`mt-3 w-full ${secondaryButtonClassName}`}
              disabled={busy || !canPasskey}
              onClick={() => {
                setError(null)
                setRecoverPhase('passkey')
              }}
            >
              {canPasskey ? 'Create a passkey' : 'Passkeys are not available in this browser'}
            </button>
          </div>
        ) : recoverPhase === 'password' ? (
          <form
            key="recover-password"
            onSubmit={(event) => void handleRecoverPassword(event)}
            className="mt-4"
          >
            <StepSummary
              label="Password"
              disabled={busy}
              onChange={() => {
                setPassword('')
                setError(null)
                setRecoverPhase('method')
              }}
            />
            <div className="mt-4">
              <NewPasswordField
                value={password}
                onChange={setPassword}
                disabled={busy}
                autoFocus
              />
            </div>
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              Save password
            </button>
          </form>
        ) : (
          <PasskeyCreateStep
            busy={busy}
            canPasskey={canPasskey}
            label={passkeyLabel}
            onLabelChange={setPasskeyLabel}
            onCreate={() =>
              void runCeremony(
                '/api/auth/reset/passkey/options',
                '/api/auth/reset/passkey',
                { token: recoverFromQuery, label: passkeyLabel.trim() || undefined },
                'create',
              )
            }
          >
            <StepSummary
              label="Passkey"
              disabled={busy}
              onChange={() => {
                setPasskeyLabel('')
                setError(null)
                setRecoverPhase('method')
              }}
            />
          </PasskeyCreateStep>
        )
      ) : mode === 'register' ? (
        registerPhase === 'checking' ? (
          <p className="mt-4 text-sm text-ink-muted">Checking invitation…</p>
        ) : registerPhase === 'invalid' ? (
          <div className="mt-4">
            <p className="text-sm text-ink" role="alert">
              This invitation is not valid.
            </p>
            <a href={window.location.pathname} className="mt-6 inline-block text-sm font-semibold text-ink underline underline-offset-4">
              Back to login
            </a>
          </div>
        ) : registerPhase === 'token' ? (
          <form onSubmit={(event) => void handleInvitation(event)} className="mt-4">
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
                autoFocus
              />
            </label>
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              Continue
            </button>
          </form>
        ) : registerPhase === 'alias' ? (
          <form onSubmit={handleAlias} className="mt-4">
            <label className="block text-sm text-ink-muted">
              Alias
              <input
                type="text"
                name="alias"
                autoComplete="nickname"
                required
                maxLength={128}
                value={alias}
                onChange={(event) => setAlias(event.target.value)}
                className={inputClassName}
                disabled={busy}
                autoFocus
              />
            </label>
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              Continue
            </button>
          </form>
        ) : registerPhase === 'method' ? (
          <div className="mt-4">
            <p className="text-sm text-ink-muted">Choose a password or a passkey for this account.</p>
            <button
              type="button"
              className={`mt-6 w-full ${primaryButtonClassName}`}
              disabled={busy}
              onClick={() => {
                setError(null)
                setRegisterPhase('password')
              }}
            >
              Create a password
            </button>
            <button
              type="button"
              className={`mt-3 w-full ${secondaryButtonClassName}`}
              disabled={busy || !canPasskey}
              onClick={() => {
                setError(null)
                setRegisterPhase('passkey')
              }}
            >
              {canPasskey ? 'Create a passkey' : 'Passkeys are not available in this browser'}
            </button>
          </div>
        ) : registerPhase === 'password' ? (
          <form onSubmit={handleRegister} className="mt-4">
            <StepSummary
              label="Password"
              disabled={busy}
              onChange={() => {
                setPassword('')
                setError(null)
                setRegisterPhase('method')
              }}
            />
            <div className="mt-4">
              <NewPasswordField value={password} onChange={setPassword} disabled={busy} autoFocus />
            </div>
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              Register
            </button>
          </form>
        ) : (
          <PasskeyCreateStep
            busy={busy}
            canPasskey={canPasskey}
            label={passkeyLabel}
            onLabelChange={setPasskeyLabel}
            onCreate={() =>
              void runCeremony(
                '/api/auth/register/passkey/options',
                '/api/auth/register/passkey',
                {
                  token: inviteToken.trim(),
                  alias: alias.trim(),
                  label: passkeyLabel.trim() || undefined,
                },
                'create',
              )
            }
          >
            <StepSummary
              label="Passkey"
              disabled={busy}
              onChange={() => {
                setPasskeyLabel('')
                setError(null)
                setRegisterPhase('method')
              }}
            />
          </PasskeyCreateStep>
        )
      ) : mode === 'recovery_sent' ? (
        <p className="mt-4 text-sm text-ink" role="status">
          {recoveryNotice ?? 'If that account exists, a recovery link has been sent.'}
        </p>
      ) : mode === 'request_recovery' ? (
        <form
          key="request-recovery"
          onSubmit={(event) => void handleRequestRecovery(event)}
          className="mt-4"
        >
          <p className="text-sm text-ink-muted">
            Enter the email for your account. If it exists, we will send a recovery link.
          </p>
          <label className="mt-4 block text-sm text-ink-muted">
            Email
            <input
              type="email"
              name="email"
              autoComplete="username"
              autoFocus
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className={inputClassName}
              disabled={busy}
            />
          </label>
          <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
            Send recovery link
          </button>
        </form>
      ) : loginPhase === 'email' ? (
        <div className="mt-4">
          <form key="login-email" onSubmit={handleEmailNext}>
            <label className="block text-sm text-ink-muted">
              Email
              <input
                type="email"
                name="email"
                autoComplete="username webauthn"
                autoFocus
                required
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                className={inputClassName}
                disabled={busy}
              />
            </label>
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              Next
            </button>
            <button
              type="button"
              className={`mt-3 w-full ${secondaryButtonClassName}`}
              disabled={busy || !canPasskey}
              onClick={startPasskeyLogin}
            >
              {canPasskey ? 'Log in with a passkey' : 'Passkeys are not available in this browser'}
            </button>
          </form>
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={busy}
            onClick={showRequestRecovery}
          >
            Recover account
          </button>
        </div>
      ) : (
        <div className="mt-4">
          <form key="login-password" onSubmit={handleLogin}>
            <StepSummary label={email} disabled={busy} onChange={showEmailStep} />
            <label className="mt-4 block text-sm text-ink-muted">
              Password
              <input
                ref={passwordInputRef}
                type="password"
                name="password"
                autoComplete="current-password"
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
              Log in
            </button>
          </form>
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={busy}
            onClick={showRequestRecovery}
          >
            Recover account
          </button>
        </div>
      )}
    </section>
  )
}

export function AccountPanel() {
  const { api, user, establish, logout } = useSession()
  const [passkeys, setPasskeys] = useState<PasskeyRow[]>([])
  const [sessions, setSessions] = useState<SessionRow[]>([])
  const [passkeysLoaded, setPasskeysLoaded] = useState(false)
  const [passkeyLabel, setPasskeyLabel] = useState('')
  const [addingPasskey, setAddingPasskey] = useState(false)
  const [password, setPassword] = useState('')
  const [settingPassword, setSettingPassword] = useState(false)
  const [aliasDraft, setAliasDraft] = useState('')
  const [settingAlias, setSettingAlias] = useState(false)
  const [stepUpPassword, setStepUpPassword] = useState('')
  const [stepUpError, setStepUpError] = useState<string | null>(null)
  const [stepUpBusy, setStepUpBusy] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const canPasskey = passkeySupported()
  const passwordSet = user?.has_password ?? false
  const canRemovePassword = passkeysLoaded && passwordSet && passkeys.length > 0
  const solePasskey = !passwordSet && passkeys.length <= 1
  const currentSession = sessions.find((session) => session.current) ?? null
  const otherSessions = sessions.filter((session) => !session.current)
  const stepUpToken = useRef<{ token: string; expiresAt: number } | null>(null)
  const stepUpWait = useRef<{
    resolve: (token: string) => void
    reject: (err: Error) => void
  } | null>(null)
  const [stepUpOpen, setStepUpOpen] = useState(false)

  function askStepUp(): Promise<string> {
    const existing = readStepUp(stepUpToken)
    if (existing) return Promise.resolve(existing)
    return new Promise((resolve, reject) => {
      stepUpWait.current = { resolve, reject }
      setStepUpPassword('')
      setStepUpError(null)
      setStepUpOpen(true)
    })
  }

  function cancelStepUp() {
    stepUpWait.current?.reject(new Error('cancelled'))
    stepUpWait.current = null
    setStepUpOpen(false)
    setStepUpPassword('')
    setStepUpError(null)
  }

  function finishStepUp(token: string, expiresIn: number) {
    storeStepUp(stepUpToken, token, expiresIn)
    const waiting = stepUpWait.current
    stepUpWait.current = null
    setStepUpOpen(false)
    setStepUpPassword('')
    setStepUpError(null)
    waiting?.resolve(token)
  }

  async function withStepUp(run: (token: string) => Promise<Response>): Promise<Response> {
    let token = await askStepUp()
    let response = await run(token)
    if (response.status !== 403) return response
    const code = await readError(response.clone())
    if (code !== 'step_up_required' && code !== 'step_up_invalid') return response
    stepUpToken.current = null
    token = await askStepUp()
    return run(token)
  }

  async function submitStepUpPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!user) return
    setStepUpBusy(true)
    setStepUpError(null)
    try {
      const response = await api('/api/auth/step-up/login', {
        method: 'POST',
        body: JSON.stringify({ email: user.email, password: stepUpPassword }),
      })
      if (!response.ok) {
        setStepUpError(accountError(await readError(response)))
        return
      }
      const token = response.headers.get('x-step-up-token')
      const body = (await response.json()) as { expires_in?: number }
      if (!token) {
        setStepUpError('Re-authentication did not return a token')
        return
      }
      finishStepUp(token, body.expires_in ?? 300)
    } catch (err) {
      setStepUpError(messageFrom(err))
    } finally {
      setStepUpBusy(false)
    }
  }

  async function submitStepUpPasskey() {
    setStepUpBusy(true)
    setStepUpError(null)
    try {
      const started = await api('/api/auth/step-up/passkeys/login/options', {
        method: 'POST',
        body: '{}',
      })
      if (!started.ok) {
        setStepUpError(accountError(await readError(started)))
        return
      }
      const ceremony = (await started.json()) as CeremonyStart
      const credential = await getPasskey(ceremony.options)
      const finished = await api('/api/auth/step-up/passkeys/login', {
        method: 'POST',
        body: JSON.stringify({ flow_id: ceremony.flow_id, credential }),
      })
      if (!finished.ok) {
        setStepUpError(accountError(await readError(finished)))
        return
      }
      const token = finished.headers.get('x-step-up-token')
      const body = (await finished.json()) as { expires_in?: number }
      if (!token) {
        setStepUpError('Re-authentication did not return a token')
        return
      }
      finishStepUp(token, body.expires_in ?? 300)
    } catch (err) {
      setStepUpError(messageFrom(err))
    } finally {
      setStepUpBusy(false)
    }
  }

  async function loadPasskeys() {
    const response = await api('/api/auth/passkeys')
    if (!response.ok) {
      setError(await readError(response))
      return
    }
    const body = (await response.json()) as { passkeys: PasskeyRow[] }
    setPasskeys(body.passkeys)
    setPasskeysLoaded(true)
  }

  async function loadSessions() {
    const response = await api('/api/auth/sessions')
    if (!response.ok) return
    const body = (await response.json()) as { sessions: SessionRow[] }
    setSessions(body.sessions)
  }

  useEffect(() => {
    void loadPasskeys()
    void loadSessions()
  }, [])

  async function revokeSession(session: SessionRow) {
    if (session.current) return
    setBusy(true)
    setError(null)
    try {
      const response = await api(`/api/auth/sessions/${session.public_id}`, { method: 'DELETE' })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      await loadSessions()
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function revokeOtherSessions() {
    setBusy(true)
    setError(null)
    try {
      const listed = await api('/api/auth/sessions')
      if (!listed.ok) {
        setError(await readError(listed))
        return
      }
      const body = (await listed.json()) as { sessions: SessionRow[] }
      for (const session of body.sessions) {
        if (session.current) continue
        const response = await api(`/api/auth/sessions/${session.public_id}`, { method: 'DELETE' })
        if (!response.ok) {
          setError(await readError(response))
          break
        }
      }
      await loadSessions()
    } catch (err) {
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function addPasskey() {
    setBusy(true)
    setError(null)
    try {
      const started = await withStepUp((token) =>
        api('/api/auth/passkeys/register/options', {
          method: 'POST',
          headers: { 'X-Step-Up-Token': token },
          body: JSON.stringify({ label: passkeyLabel.trim() || undefined }),
        }),
      )
      if (!started.ok) {
        setError(await readError(started))
        return
      }
      const ceremony = (await started.json()) as CeremonyStart
      const credential = await createPasskey(ceremony.options)
      const finished = await withStepUp((token) =>
        api('/api/auth/passkeys/register', {
          method: 'POST',
          headers: { 'X-Step-Up-Token': token },
          body: JSON.stringify({ flow_id: ceremony.flow_id, credential }),
        }),
      )
      if (!finished.ok) {
        setError(await readError(finished))
        return
      }
      setPasskeyLabel('')
      setAddingPasskey(false)
      await loadPasskeys()
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function removePasskey(publicId: string) {
    setBusy(true)
    setError(null)
    try {
      const response = await withStepUp((token) =>
        api(`/api/auth/passkeys/${publicId}`, {
          method: 'DELETE',
          headers: { 'X-Step-Up-Token': token },
        }),
      )
      if (!response.ok) {
        setError(accountError(await readError(response)))
        return
      }
      await loadPasskeys()
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function savePassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    try {
      const response = await withStepUp((token) =>
        api('/api/auth/password', {
          method: 'PUT',
          headers: { 'X-Step-Up-Token': token },
          body: JSON.stringify({ password }),
        }),
      )
      if (!response.ok) {
        setError(accountError(await readError(response)))
        return
      }
      establish((await response.json()) as AuthUser)
      setPassword('')
      setSettingPassword(false)
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function removePassword() {
    setBusy(true)
    setError(null)
    try {
      const response = await withStepUp((token) =>
        api('/api/auth/password', {
          method: 'DELETE',
          headers: { 'X-Step-Up-Token': token },
        }),
      )
      if (!response.ok) {
        setError(accountError(await readError(response)))
        return
      }
      establish((await response.json()) as AuthUser)
      setPassword('')
      setSettingPassword(false)
    } catch (err) {
      if (err instanceof Error && err.message === 'cancelled') return
      setError(messageFrom(err))
    } finally {
      setBusy(false)
    }
  }

  async function saveAlias(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    try {
      const response = await api('/api/auth/alias', {
        method: 'PUT',
        body: JSON.stringify({ alias: aliasDraft.trim() }),
      })
      if (!response.ok) {
        setError(accountError(await readError(response)))
        return
      }
      establish((await response.json()) as AuthUser)
      setAliasDraft('')
      setSettingAlias(false)
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
        <button type="button" className={secondaryButtonClassName} onClick={() => void logout()}>
          Log out
        </button>
      </div>
      {settingAlias ? (
        <form onSubmit={(event) => void saveAlias(event)} className="mt-4">
          <label className="block text-sm text-ink-muted">
            Alias
            <input
              type="text"
              name="alias"
              autoComplete="nickname"
              required
              maxLength={128}
              autoFocus
              value={aliasDraft}
              onChange={(event) => setAliasDraft(event.target.value)}
              className={inputClassName}
              disabled={busy}
            />
          </label>
          <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
            Save alias
          </button>
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={busy}
            onClick={() => {
              setAliasDraft('')
              setError(null)
              setSettingAlias(false)
            }}
          >
            Cancel
          </button>
        </form>
      ) : (
        <div className="mt-4 flex items-center justify-between gap-3">
          <p className="min-w-0 truncate text-sm text-ink">{user?.alias}</p>
          <button
            type="button"
            className={secondaryButtonClassName}
            disabled={busy}
            onClick={() => {
              setError(null)
              setPassword('')
              setSettingPassword(false)
              setAddingPasskey(false)
              setAliasDraft(user?.alias ?? '')
              setSettingAlias(true)
            }}
          >
            Change
          </button>
        </div>
      )}
      <p className="mt-1 text-sm text-ink-muted">{user?.email}</p>
      <p className="mt-1 break-all font-mono text-xs text-ink-muted">{user?.public_id}</p>
      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}
      {stepUpOpen ? (
        <div className="mt-5 border-t border-ring/50 pt-5">
          <h3 className="text-sm font-semibold tracking-wide text-ink">Confirm it's you</h3>
          <p className="mt-3 text-sm text-ink">{user?.email}</p>
          {stepUpError ? (
            <p className="mt-3 text-sm text-ink" role="alert">
              {stepUpError}
            </p>
          ) : null}
          {passwordSet ? (
            <form onSubmit={(event) => void submitStepUpPassword(event)} className="mt-4">
              <label className="block text-sm text-ink-muted">
                Password
                <input
                  type="password"
                  autoComplete="current-password"
                  required
                  minLength={15}
                  maxLength={256}
                  value={stepUpPassword}
                  onChange={(event) => setStepUpPassword(event.target.value)}
                  className={inputClassName}
                  disabled={stepUpBusy}
                  autoFocus
                />
              </label>
              <button
                type="submit"
                className={`mt-6 w-full ${primaryButtonClassName}`}
                disabled={stepUpBusy}
              >
                Confirm password
              </button>
            </form>
          ) : null}
          {passkeys.length > 0 && canPasskey ? (
            <button
              type="button"
              className={`mt-3 w-full ${passwordSet ? secondaryButtonClassName : primaryButtonClassName}`}
              disabled={stepUpBusy}
              onClick={() => void submitStepUpPasskey()}
            >
              Use a passkey
            </button>
          ) : null}
          <button
            type="button"
            className={`mt-3 w-full ${secondaryButtonClassName}`}
            disabled={stepUpBusy}
            onClick={cancelStepUp}
          >
            Cancel
          </button>
        </div>
      ) : null}
      <div className="mt-5 border-t border-ring/50 pt-5">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold tracking-wide text-ink">Password</h3>
          {settingPassword ? null : (
            <div className="flex gap-2">
              <button
                type="button"
                className={secondaryButtonClassName}
                disabled={busy || !passkeysLoaded}
                onClick={() => {
                  setError(null)
                  setPassword('')
                  setAddingPasskey(false)
                  setSettingAlias(false)
                  setSettingPassword(true)
                }}
              >
                {passwordSet ? 'Change' : 'Set'}
              </button>
              {passwordSet ? (
                <button
                  type="button"
                  className={secondaryButtonClassName}
                  disabled={busy || !canRemovePassword}
                  onClick={() => void removePassword()}
                >
                  Remove
                </button>
              ) : null}
            </div>
          )}
        </div>
        {settingPassword ? (
          <form onSubmit={(event) => void savePassword(event)} className="mt-4">
            <NewPasswordField value={password} onChange={setPassword} disabled={busy} autoFocus />
            <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={busy}>
              {passwordSet ? 'Save password' : 'Set password'}
            </button>
            <button
              type="button"
              className={`mt-3 w-full ${secondaryButtonClassName}`}
              disabled={busy}
              onClick={() => {
                setPassword('')
                setError(null)
                setSettingPassword(false)
              }}
            >
              Cancel
            </button>
          </form>
        ) : (
          <p className="mt-3 text-sm text-ink-muted">
            {passwordSet ? 'Password is set.' : 'No password is set.'}
            {passwordSet && passkeysLoaded && passkeys.length === 0
              ? ' Add a passkey before removing it.'
              : ''}
          </p>
        )}
      </div>
      <div className="mt-5 border-t border-ring/50 pt-5">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold tracking-wide text-ink">Passkeys</h3>
          {addingPasskey ? null : (
            <button
              type="button"
              className={secondaryButtonClassName}
              disabled={busy || !canPasskey || !passkeysLoaded}
              onClick={() => {
                setError(null)
                setPasskeyLabel('')
                setSettingPassword(false)
                setSettingAlias(false)
                setAddingPasskey(true)
              }}
            >
              Add
            </button>
          )}
        </div>
        {addingPasskey ? (
          <PasskeyCreateStep
            busy={busy}
            canPasskey={canPasskey}
            label={passkeyLabel}
            onLabelChange={setPasskeyLabel}
            backLabel="Cancel"
            onBack={() => {
              setPasskeyLabel('')
              setError(null)
              setAddingPasskey(false)
            }}
            onCreate={() => void addPasskey()}
          />
        ) : passkeys.length === 0 ? (
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
                  {passkey.aaguid ? (
                    <span className="mt-0.5 block break-all font-mono text-xs text-ink-muted">
                      AAGUID {passkey.aaguid}
                    </span>
                  ) : null}
                </span>
                <button
                  type="button"
                  className={secondaryButtonClassName}
                  disabled={busy || solePasskey}
                  onClick={() => void removePasskey(passkey.public_id)}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="mt-5 border-t border-ring/50 pt-5">
        <h3 className="text-sm font-semibold tracking-wide text-ink">Sessions</h3>
        <div className="mt-4">
          <h4 className="text-sm font-semibold tracking-wide text-ink">Current session</h4>
          {currentSession ? (
            <ul className="mt-3 space-y-2">
              <li className="text-sm">
                <span className="block text-ink">{sessionLabel(currentSession.user_agent)}</span>
                <span className="block text-xs text-ink-muted">
                  Last used {currentSession.last_used_at.slice(0, 10)} · Expires{' '}
                  {currentSession.expires_at.slice(0, 10)}
                </span>
              </li>
            </ul>
          ) : (
            <p className="mt-3 text-sm text-ink-muted">No sessions.</p>
          )}
        </div>
        <div className="mt-5">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h4 className="text-sm font-semibold tracking-wide text-ink">Other active sessions</h4>
            <button
              type="button"
              className={secondaryButtonClassName}
              disabled={busy || otherSessions.length === 0}
              onClick={() => void revokeOtherSessions()}
            >
              Revoke all
            </button>
          </div>
          {otherSessions.length === 0 ? (
            <p className="mt-3 text-sm text-ink-muted">No other active sessions.</p>
          ) : (
            <ul className="mt-3 space-y-2">
              {otherSessions.map((session) => (
                <li
                  key={session.public_id}
                  className="flex items-center justify-between gap-3 text-sm"
                >
                  <span>
                    <span className="block text-ink">{sessionLabel(session.user_agent)}</span>
                    <span className="block text-xs text-ink-muted">
                      Last used {session.last_used_at.slice(0, 10)} · Expires{' '}
                      {session.expires_at.slice(0, 10)}
                    </span>
                  </span>
                  <button
                    type="button"
                    className={secondaryButtonClassName}
                    disabled={busy}
                    onClick={() => void revokeSession(session)}
                  >
                    Revoke
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  )
}
