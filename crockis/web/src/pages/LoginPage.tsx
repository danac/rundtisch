import { Navigate, useLocation } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'
import { LoginForm } from '../LoginForm.tsx'

type LoginLocationState = {
  from?: string
}

export function LoginPage() {
  const { status, user } = useAuth()
  const location = useLocation()
  const from = (location.state as LoginLocationState | null)?.from ?? '/collections'

  if (user) {
    return <Navigate to={from} replace />
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
        <div className="relative w-full max-w-md">
          <p className="font-display text-center text-[12px] font-medium tracking-[0.32em] uppercase text-mist">
            Private library
          </p>
          {status === 'checking' ? (
            <p className="mt-10 text-center text-sm text-mist">Checking session…</p>
          ) : (
            <div className="mt-10">
              <LoginForm />
            </div>
          )}
        </div>
      </main>
    </div>
  )
}
