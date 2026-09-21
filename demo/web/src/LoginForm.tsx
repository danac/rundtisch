import type { FormEvent } from 'react'

export function LoginForm() {
  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
  }

  return (
    <form
      onSubmit={handleSubmit}
      className="mt-10 w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]"
      aria-labelledby="login-heading"
    >
      <h2 id="login-heading" className="text-sm font-semibold tracking-wide text-ink">
        Log in
      </h2>
      <div className="mt-4 space-y-4">
        <label className="block text-sm text-ink-muted">
          Email
          <input
            type="email"
            name="email"
            autoComplete="username"
            className="mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60"
          />
        </label>
        <label className="block text-sm text-ink-muted">
          Password
          <input
            type="password"
            name="password"
            autoComplete="current-password"
            className="mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60"
          />
        </label>
      </div>
      <button
        type="submit"
        className="mt-6 w-full rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        Log in
      </button>
    </form>
  )
}
