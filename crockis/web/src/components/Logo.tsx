export function Logo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 48 48" className={className} fill="none" aria-hidden>
      <path
        d="M24 7c1.8 7.2 1.8 12.6 0 17.2C22.2 19.6 22.2 14.2 24 7Z"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinejoin="round"
      />
      <path
        d="M24 11c4.6 4.4 7.6 8.4 8.2 12.6-4.8-1.6-8.4-2.4-8.2-12.6Z"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinejoin="round"
      />
      <path
        d="M24 11c-4.6 4.4-7.6 8.4-8.2 12.6 4.8-1.6 8.4-2.4 8.2-12.6Z"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinejoin="round"
      />
      <path
        d="M16.2 27.8C19.4 30 21.8 31.2 24 31.2c2.2 0 4.6-1.2 7.8-3.4"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinecap="round"
      />
      <path
        d="M15 24.4c-3.2 2.2-5 4.6-5.2 7.2 3.6.2 6.8-1.4 10.2-5.2"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinejoin="round"
      />
      <path
        d="M33 24.4c3.2 2.2 5 4.6 5.2 7.2-3.6.2-6.8-1.4-10.2-5.2"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinejoin="round"
      />
    </svg>
  )
}
