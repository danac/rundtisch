import { useState } from 'react'
import { useSession } from './session.tsx'

const panelClassName =
  'w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]'
const primaryButtonClassName =
  'rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'

type CliLink = {
  redirect: URL
  state: string
  os: string
  osVersion: string
}

function loopbackRedirect(value: string): URL | null {
  let url: URL
  try {
    url = new URL(value)
  } catch {
    return null
  }
  if (url.protocol !== 'http:') return null
  if (url.username || url.password) return null
  if (url.hostname !== '127.0.0.1' && url.hostname !== 'localhost' && url.hostname !== '::1') {
    return null
  }
  if (url.port === '') return null
  return url
}

function readCliLink(): CliLink | null {
  const params = new URLSearchParams(window.location.search)
  const redirect = loopbackRedirect(params.get('redirect_uri') ?? '')
  const state = params.get('state')?.trim() ?? ''
  const os = params.get('os')?.trim() ?? ''
  const osVersion = params.get('os_version')?.trim() ?? ''
  if (!redirect || state === '' || os === '' || osVersion === '') return null
  return { redirect, state, os, osVersion }
}

export function authorizeScreen(): 'authorize' | 'done' | null {
  const path = window.location.pathname.replace(/\/+$/, '') || '/'
  if (path === '/authorize/done') return 'done'
  if (path === '/authorize') return 'authorize'
  return null
}

export function CliAuthorized() {
  return (
    <section className={`${panelClassName} mt-10 max-w-md`} aria-labelledby="cli-done-heading">
      <h2 id="cli-done-heading" className="text-sm font-semibold tracking-wide text-ink">
        Command line tool signed in
      </h2>
      <p className="mt-4 text-sm text-ink-muted">You can close this tab.</p>
    </section>
  )
}

export function CliAuthorize() {
  const { api, user } = useSession()
  const link = readCliLink()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function authorize() {
    if (!link) return
    setBusy(true)
    setError(null)
    try {
      const response = await api('/api/auth/sessions', {
        method: 'POST',
        body: JSON.stringify({ os: link.os, os_version: link.osVersion }),
      })
      if (!response.ok) {
        setError('Could not create a session for this machine.')
        return
      }
      const body = (await response.json()) as { token?: string; expires_in?: number }
      if (!body.token) {
        setError('Could not create a session for this machine.')
        return
      }
      const target = new URL(link.redirect.href)
      target.searchParams.set('token', body.token)
      target.searchParams.set('state', link.state)
      if (body.expires_in !== undefined) {
        target.searchParams.set('expires_in', String(body.expires_in))
      }
      window.location.assign(target.toString())
    } catch {
      setError('Could not create a session for this machine.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={`${panelClassName} mt-10 max-w-md`} aria-labelledby="cli-heading">
      <h2 id="cli-heading" className="text-sm font-semibold tracking-wide text-ink">
        Authorize command line
      </h2>
      {link ? (
        <>
          <p className="mt-4 text-sm text-ink">
            Allow a command line tool on {link.os} {link.osVersion} to use {user?.email}.
          </p>
          <p className="mt-2 break-all text-sm text-ink-muted">{link.redirect.host}</p>
          {error ? (
            <p className="mt-3 text-sm text-ink" role="alert">
              {error}
            </p>
          ) : null}
          <button
            type="button"
            className={`mt-6 w-full ${primaryButtonClassName}`}
            disabled={busy}
            onClick={() => void authorize()}
          >
            Authorize
          </button>
        </>
      ) : (
        <p className="mt-4 text-sm text-ink" role="alert">
          This authorization link is missing a loopback redirect, state, or machine details.
        </p>
      )}
    </section>
  )
}
