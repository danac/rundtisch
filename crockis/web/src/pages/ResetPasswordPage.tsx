import { useState, type FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { api, ApiError } from '../api'
import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'

export function ResetPasswordPage() {
  const { token } = useAuth()
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const [done, setDone] = useState(false)

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    if (password !== confirm) {
      setError('The two passwords do not match.')
      return
    }
    if (!token) {
      setError('Sign in to reset your password.')
      return
    }
    setPending(true)
    try {
      await api.resetPassword(token, { password })
      setDone(true)
    } catch (cause) {
      setError(cause instanceof ApiError ? cause.message : 'Unable to reset the password.')
    } finally {
      setPending(false)
    }
  }

  return (
    <div className="flex min-h-dvh flex-col">
      <Header
        crumbs={[
          { label: 'Collections', to: '/collections' },
          { label: 'Settings', to: '/settings' },
          { label: 'Password' },
        ]}
      />
      <main className="flex flex-1 justify-center px-6 pb-24 pt-8 sm:pt-12">
        <div className="w-full max-w-sm">
          <p className="font-display text-[12px] font-medium tracking-[0.32em] uppercase text-mist">
            Account
          </p>
          <h1 className="mt-3 font-display text-2xl font-medium tracking-[0.18em] uppercase">
            Reset password
          </h1>

          {done ? (
            <div className="mt-12">
              <p className="text-sm leading-relaxed text-mist">Your password has been updated.</p>
              <p className="mt-8">
                <Link to="/settings" className="account-link">
                  Back to settings
                </Link>
              </p>
            </div>
          ) : (
            <form
              onSubmit={(event) => {
                void onSubmit(event)
              }}
              className="mt-12"
            >
              <label className="block font-display text-[11px] font-medium tracking-[0.2em] uppercase text-mist">
                New password
                <input
                  type="password"
                  name="password"
                  autoComplete="new-password"
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  className="auth-field"
                />
              </label>
              <label className="mt-8 block font-display text-[11px] font-medium tracking-[0.2em] uppercase text-mist">
                Confirm password
                <input
                  type="password"
                  name="confirm"
                  autoComplete="new-password"
                  required
                  value={confirm}
                  onChange={(event) => setConfirm(event.target.value)}
                  className="auth-field"
                />
              </label>
              {error ? (
                <p className="mt-6 text-sm text-red-300/90" role="alert">
                  {error}
                </p>
              ) : null}
              <button
                type="submit"
                disabled={pending}
                className="mt-12 w-full border border-ink/70 px-6 py-3.5 font-display text-[12px] font-medium tracking-[0.28em] uppercase transition-colors hover:bg-ink hover:text-void disabled:cursor-wait disabled:opacity-60"
              >
                {pending ? 'Saving…' : 'Save password'}
              </button>
            </form>
          )}
        </div>
      </main>
    </div>
  )
}
