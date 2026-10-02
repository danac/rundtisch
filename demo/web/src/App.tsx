import { AccountPanel, LoginForm } from './LoginForm.tsx'
import { CliAuthorize, CliAuthorized, authorizeScreen } from './CliAuthorize.tsx'
import { SessionProvider, useSession } from './session.tsx'
import { ThemeToggle } from './theme.tsx'

function SessionGate() {
  const { status, user } = useSession()
  const screen = authorizeScreen()
  if (screen === 'done') {
    return <CliAuthorized />
  }
  if (status === 'checking') {
    return <p className="mt-10 text-sm text-ink-muted">Checking session…</p>
  }
  if (!user) {
    return (
      <div className="mt-10 w-full max-w-md">
        <LoginForm />
      </div>
    )
  }
  if (screen === 'authorize') {
    return <CliAuthorize />
  }
  return (
    <div className="mt-10 flex w-full max-w-md flex-col gap-4">
      <AccountPanel />
    </div>
  )
}

function App() {
  return (
    <SessionProvider>
      <main className="relative flex min-h-full flex-col items-center justify-center overflow-hidden px-6 py-16">
        <div className="absolute right-6 top-6 z-20">
          <ThemeToggle />
        </div>

        {/* Soft paper grain */}
        <div aria-hidden className="app-grain pointer-events-none absolute inset-0 opacity-[0.35]" />

        {/* Concentric rings — round table motif */}
        <div
          aria-hidden
          className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
        >
          {[520, 400, 280, 160].map((size) => (
            <div
              key={size}
              className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full border border-ring/60"
              style={{ width: size, height: size }}
            />
          ))}
          <div
            className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-ring/25"
            style={{ width: 48, height: 48 }}
          />
        </div>

        <div className="relative z-10 flex w-full max-w-4xl flex-col items-center text-center">
          <h1 className="font-display text-5xl font-semibold tracking-tight text-ink sm:text-6xl md:text-7xl">
            <a href="/" className="text-inherit no-underline">
              rundtisch
            </a>
          </h1>
          <p className="mt-4 text-lg text-ink-muted sm:text-xl">
            a round table for building on the web
          </p>
          <SessionGate />
        </div>
      </main>
    </SessionProvider>
  )
}

export default App
