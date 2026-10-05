import { Link } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'
import { Logo } from './Logo'
import { UserMenu } from './UserMenu'

export type Crumb = {
  label: string
  to?: string
}

type HeaderProps = {
  crumbs?: Crumb[]
  selecting?: boolean
  onToggleSelect?: () => void
}

export function Header({ crumbs, selecting, onToggleSelect }: HeaderProps) {
  const { user } = useAuth()

  return (
    <header className="flex h-16 items-center justify-between gap-3 px-3 sm:h-[72px] sm:gap-6 sm:px-5 md:px-8">
      <div className="flex min-w-0 items-center gap-2 sm:gap-3">
        <Link
          to={user ? '/collections' : '/login'}
          className="flex shrink-0 items-center text-ink"
          aria-label="Crockis"
        >
          <Logo className="h-7 w-7 sm:h-8 sm:w-8" />
        </Link>
        {crumbs && crumbs.length > 0 ? (
          <Breadcrumb crumbs={crumbs} />
        ) : (
          <span className="font-display text-[13px] font-medium tracking-[0.16em] uppercase sm:text-[15px] sm:tracking-[0.28em]">
            Crockis
          </span>
        )}
      </div>

      <nav className="flex shrink-0 items-center justify-end gap-3 font-display text-[12px] font-medium sm:gap-6">
        {user ? (
          <>
            {onToggleSelect ? (
              <button
                type="button"
                aria-pressed={selecting}
                onClick={onToggleSelect}
                className="tracking-[0.14em] uppercase text-mist transition-colors hover:text-ink sm:tracking-[0.22em]"
              >
                {selecting ? 'Cancel' : 'Select'}
              </button>
            ) : null}
            <UserMenu />
          </>
        ) : (
          <Link
            to="/login"
            className="tracking-[0.14em] uppercase text-mist transition-colors hover:text-ink sm:tracking-[0.22em]"
          >
            Sign in
          </Link>
        )}
      </nav>
    </header>
  )
}

function Breadcrumb({ crumbs }: { crumbs: Crumb[] }) {
  return (
    <nav aria-label="Breadcrumb" className="min-w-0">
      <ol className="flex min-w-0 items-center gap-2 font-display text-[12px] font-medium tracking-[0.12em] uppercase sm:gap-2.5 sm:text-[13px] sm:tracking-[0.18em]">
        {crumbs.map((crumb, index) => {
          const current = !crumb.to
          return (
            <li key={`${crumb.label}-${index}`} className="flex min-w-0 items-center gap-2 sm:gap-2.5">
              {index > 0 ? (
                <span className="shrink-0 text-mist/50" aria-hidden>
                  /
                </span>
              ) : null}
              {current ? (
                <span className="truncate text-ink">{crumb.label}</span>
              ) : crumb.to ? (
                <Link to={crumb.to} className="shrink-0 text-mist transition-colors hover:text-ink">
                  {crumb.label}
                </Link>
              ) : null}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
