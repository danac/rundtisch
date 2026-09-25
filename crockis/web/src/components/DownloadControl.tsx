import type { KeyboardEvent, MouseEvent, ReactNode } from 'react'

type DownloadControlProps = {
  label: string
  busy?: boolean
  nested?: boolean
  onDownload: () => void
}

function Icon() {
  return (
    <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" aria-hidden>
      <path d="M12 4.5v9" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <path d="M8.5 11 12 14.5 15.5 11" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M6 18.5h12" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  )
}

export function DownloadControl({ label, busy, nested, onDownload }: DownloadControlProps) {
  function activate(event: MouseEvent | KeyboardEvent) {
    event.preventDefault()
    event.stopPropagation()
    if (!busy) onDownload()
  }

  const content: ReactNode = busy ? <span className="download-control__dot" /> : <Icon />

  if (nested) {
    return (
      <span
        role="button"
        tabIndex={0}
        aria-label={label}
        aria-busy={busy || undefined}
        title={label}
        className="download-control"
        onClick={activate}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') activate(event)
        }}
      >
        {content}
      </span>
    )
  }

  return (
    <button
      type="button"
      aria-label={label}
      aria-busy={busy || undefined}
      title={label}
      className="download-control"
      onClick={activate}
    >
      {content}
    </button>
  )
}
