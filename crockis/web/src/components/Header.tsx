import { Link, NavLink } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'
import { Logo } from './Logo'

type HeaderProps = {
  title?: string
}

const navClass = ({ isActive }: { isActive: boolean }) =>
  [
    'tracking-[0.22em] uppercase transition-colors hover:text-ink',
    isActive ? 'text-ink' : 'text-mist',
  ].join(' ')

export function Header({ title }: HeaderProps) {
  const { user, logout } = useAuth()

  return (
    <header className="grid h-[72px] grid-cols-[1fr_auto_1fr] items-center px-5 md:px-8">
      <Link
        to={user ? '/collections' : '/login'}
        className="flex items-center gap-3 justify-self-start text-ink"
      >
        <Logo className="h-8 w-8" />
        <span className="font-display text-[15px] font-light tracking-[0.38em] uppercase">
          Crockis
        </span>
      </Link>

      <p className="hidden font-display text-[13px] font-light tracking-[0.42em] uppercase text-mist sm:block">
        {title}
      </p>

      <nav className="flex items-center justify-end gap-7 font-display text-[12px] font-light">
        {user ? (
          <>
            <NavLink to="/collections" className={navClass}>
              Collections
            </NavLink>
            <button
              type="button"
              onClick={() => {
                void logout()
              }}
              className="tracking-[0.22em] uppercase text-mist transition-colors hover:text-ink"
            >
              Sign out
            </button>
          </>
        ) : (
          <NavLink to="/login" className={navClass}>
            Sign in
          </NavLink>
        )}
      </nav>
    </header>
  )
}
