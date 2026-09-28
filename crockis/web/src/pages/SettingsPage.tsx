import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import type { AuthMethod } from '../api'
import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'

export function SettingsPage() {
  const { user } = useAuth()
  if (!user) return null

  const password = method(user.authMethods, 'password')
  const passkey = method(user.authMethods, 'passkey')

  return (
    <div className="flex min-h-dvh flex-col">
      <Header
        crumbs={[
          { label: 'Collections', to: '/collections' },
          { label: 'Settings' },
        ]}
      />
      <main className="flex flex-1 justify-center px-6 pb-24 pt-8 sm:pt-12">
        <div className="w-full max-w-md">
          <p className="font-display text-[12px] font-medium tracking-[0.32em] uppercase text-mist">
            Account
          </p>
          <h1 className="mt-3 font-display text-2xl font-medium tracking-[0.18em] uppercase">
            Settings
          </h1>

          <section className="mt-12">
            <h2 className="font-display text-[11px] font-medium tracking-[0.2em] uppercase text-mist">
              Profile
            </h2>
            <dl>
              <div className="account-row">
                <dt>Email</dt>
                <dd>{user.email}</dd>
              </div>
              {user.name ? (
                <div className="account-row">
                  <dt>Name</dt>
                  <dd>{user.name}</dd>
                </div>
              ) : null}
            </dl>
          </section>

          <section className="mt-12">
            <h2 className="font-display text-[11px] font-medium tracking-[0.2em] uppercase text-mist">
              Sign-in methods
            </h2>
            <ul>
              <AuthMethodRow
                method={password}
                action={
                  password.enrolled ? (
                    <Link to="/settings/password" className="account-link">
                      Reset password
                    </Link>
                  ) : null
                }
              />
              <AuthMethodRow method={passkey} />
            </ul>
          </section>
        </div>
      </main>
    </div>
  )
}

function method(methods: AuthMethod[], kind: AuthMethod['kind']): AuthMethod {
  return (
    methods.find((item) => item.kind === kind) ?? {
      id: kind,
      kind,
      label: kind === 'password' ? 'Password' : 'Passkey',
      enrolled: kind === 'password',
    }
  )
}

function AuthMethodRow({
  method: item,
  action,
}: {
  method: AuthMethod
  action?: ReactNode
}) {
  return (
    <li className="account-row">
      <div className="min-w-0">
        <p className="text-ink">{item.label}</p>
        {action}
      </div>
      <p className={item.enrolled ? 'text-ink' : 'text-mist'}>
        {item.enrolled ? 'Active' : item.kind === 'passkey' ? 'Coming later' : 'Not added'}
      </p>
    </li>
  )
}
