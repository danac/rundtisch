type StatusStateProps = {
  message: string
}

export function StatusState({ message }: StatusStateProps) {
  return (
    <div className="flex min-h-[50vh] items-center justify-center px-6">
      <p className="font-display text-sm font-light tracking-[0.28em] uppercase text-mist">
        {message}
      </p>
    </div>
  )
}
