import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'
import { AccountPanel } from '../LoginForm.tsx'

export function SettingsPage() {
  const { user } = useAuth()
  if (!user) return null

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
          <AccountPanel />
        </div>
      </main>
    </div>
  )
}
