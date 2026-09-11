function App() {
  return (
    <main className="relative flex min-h-full items-center justify-center overflow-hidden px-6">
      {/* Soft paper grain */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 opacity-[0.35]"
        style={{
          backgroundImage:
            'radial-gradient(circle at 50% 45%, transparent 0%, transparent 40%, rgba(44, 36, 22, 0.04) 70%, rgba(44, 36, 22, 0.08) 100%)',
        }}
      />

      {/* Concentric rings — round table motif */}
      <div
        aria-hidden
        className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
      >
        {[520, 400, 280, 160].map((size) => (
          <div
            key={size}
            className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full border border-ring/60"
            style={{ width: size, height: size }}
          />
        ))}
        <div
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-ring/25"
          style={{ width: 48, height: 48 }}
        />
      </div>

      <div className="relative z-10 text-center">
        <h1 className="font-display text-6xl font-semibold tracking-tight text-ink sm:text-7xl md:text-8xl">
          rundtisch
        </h1>
        <p className="mt-4 text-lg text-ink-muted sm:text-xl">
          a round table for building on the web
        </p>
      </div>
    </main>
  )
}

export default App
