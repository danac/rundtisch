import { useState, type FormEvent } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { ApiError } from '../api'
import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'

type LoginLocationState = {
  from?: string
}

export function LoginPage() {
  const { user, login } = useAuth()
  const location = useLocation()
  const from = (location.state as LoginLocationState | null)?.from ?? '/collections'
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  if (user) {
    return <Navigate to={from} replace />
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    setPending(true)
    try {
      await login(email.trim(), password)
    } catch (cause) {
      setError(cause instanceof ApiError ? cause.message : 'Unable to sign in.')
    } finally {
      setPending(false)
    }
  }

  return (
    <div className="flex min-h-dvh flex-col">
      <Header />
      <main className="relative flex flex-1 items-center justify-center px-6 pb-24">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-40"
          style={{
            backgroundImage:
              'radial-gradient(ellipse at 50% 20%, rgba(255,255,255,0.06), transparent 55%)',
          }}
        />
        <form
          onSubmit={(event) => {
            void onSubmit(event)
          }}
          className="relative w-full max-w-sm"
        >
          <p className="font-display text-center text-[12px] font-light tracking-[0.42em] uppercase text-mist">
            Private library
          </p>
          <h1 className="mt-4 text-center font-display text-3xl font-light tracking-[0.28em] uppercase">
            Sign in
          </h1>
          <p className="mt-4 text-center text-sm font-light text-mist">
            Any email and password will open the placeholder library until the
            rundtisch API is connected.
          </p>

          <label className="mt-12 block font-display text-[11px] tracking-[0.24em] uppercase text-mist">
            Email
            <input
              type="email"
              name="email"
              autoComplete="email"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className="mt-3 block w-full border-0 border-b border-line bg-transparent px-0 py-3 font-body text-base font-light text-ink outline-none transition-colors focus:border-ink"
            />
          </label>

          <label className="mt-8 block font-display text-[11px] tracking-[0.24em] uppercase text-mist">
            Password
            <input
              type="password"
              name="password"
              autoComplete="current-password"
              required
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className="mt-3 block w-full border-0 border-b border-line bg-transparent px-0 py-3 font-body text-base font-light text-ink outline-none transition-colors focus:border-ink"
            />
          </label>

          {error ? (
            <p className="mt-6 text-center text-sm font-light text-red-300/90" role="alert">
              {error}
            </p>
          ) : null}

          <button
            type="submit"
            disabled={pending}
            className="mt-12 w-full border border-ink/70 px-6 py-3 font-display text-[12px] tracking-[0.32em] uppercase transition-colors hover:bg-ink hover:text-void disabled:cursor-wait disabled:opacity-60"
          >
            {pending ? 'Opening…' : 'Enter'}
          </button>
        </form>
      </main>
    </div>
  )
}
