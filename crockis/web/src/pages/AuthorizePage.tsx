import { useAuth } from '../auth/useAuth'
import { CliAuthorize, CliAuthorized } from '../CliAuthorize.tsx'
import { Header } from '../components/Header'
import { LoginForm } from '../LoginForm.tsx'

export function AuthorizePage() {
  const { status, user } = useAuth()

  return (
    <div className="flex min-h-dvh flex-col">
      <Header />
      <main className="flex flex-1 items-start justify-center px-6 pb-24 pt-16">
        {status === 'checking' ? (
          <p className="text-sm text-mist">Checking session…</p>
        ) : user ? (
          <CliAuthorize />
        ) : (
          <div className="w-full max-w-md">
            <LoginForm />
          </div>
        )}
      </main>
    </div>
  )
}

export function AuthorizeDonePage() {
  return (
    <div className="flex min-h-dvh flex-col">
      <Header />
      <main className="flex flex-1 items-start justify-center px-6 pb-24 pt-16">
        <CliAuthorized />
      </main>
    </div>
  )
}
