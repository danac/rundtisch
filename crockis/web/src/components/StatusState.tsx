type StatusStateProps = {
  message: string
}

export function StatusState({ message }: StatusStateProps) {
  return (
    <div className="flex min-h-[50vh] items-center justify-center px-6">
      <p className="font-display text-sm font-medium tracking-[0.22em] uppercase text-mist">
        {message}
      </p>
    </div>
  )
}
