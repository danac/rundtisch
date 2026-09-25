import { Link, NavLink } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'
import { Logo } from './Logo'

type HeaderProps = {
  title?: string
  selecting?: boolean
  onToggleSelect?: () => void
}

const navClass = ({ isActive }: { isActive: boolean }) =>
  [
    'tracking-[0.14em] uppercase transition-colors hover:text-ink sm:tracking-[0.22em]',
    isActive ? 'text-ink' : 'text-mist',
  ].join(' ')

export function Header({ title, selecting, onToggleSelect }: HeaderProps) {
  const { user, logout } = useAuth()

  return (
    <header className="grid h-16 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-3 sm:h-[72px] sm:gap-6 sm:px-5 md:px-8">
      <Link
        to={user ? '/collections' : '/login'}
        className="flex items-center gap-2 justify-self-start text-ink sm:gap-3"
      >
        <Logo className="h-7 w-7 shrink-0 sm:h-8 sm:w-8" />
        <span className="font-display text-[13px] font-medium tracking-[0.16em] uppercase sm:text-[15px] sm:tracking-[0.28em]">
          Crockis
        </span>
      </Link>

      <div className="flex min-w-0 justify-center">
        {title ? (
          <p className="hidden truncate font-display text-[13px] font-medium tracking-[0.32em] uppercase text-mist sm:block">
            {title}
          </p>
        ) : null}
        {user ? (
          <NavLink
            to="/collections"
            className={({ isActive }) =>
              `font-display text-[12px] font-medium sm:hidden ${navClass({ isActive })}`
            }
          >
            Collections
          </NavLink>
        ) : null}
      </div>

      <nav className="flex items-center justify-end gap-3 font-display text-[12px] font-medium sm:gap-7">
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
            <NavLink to="/collections" className={`hidden sm:inline ${navClass({ isActive: false })}`}>
              Collections
            </NavLink>
            <button
              type="button"
              onClick={() => {
                void logout()
              }}
              className="tracking-[0.14em] uppercase text-mist transition-colors hover:text-ink sm:tracking-[0.22em]"
            >
              Sign out
            </button>
          </>
        ) : (
          <NavLink to="/login" className={`font-display text-[12px] font-medium ${navClass({ isActive: false })}`}>
            Sign in
          </NavLink>
        )}
      </nav>
    </header>
  )
}
