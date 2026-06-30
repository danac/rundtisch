import { useCallback, useEffect, useState } from "react";

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
}

/**
 * Generic data-fetching hook with loading/error state and an abortable
 * fetcher. Pages use this to render skeletons and error states while the
 * service layer resolves (mock data today, REST later).
 *
 * `fetcher` should be stable (wrap in `useCallback` at the call site or
 * pass a module-level function).
 */
export function useAsyncData<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
): AsyncState<T> & { reload: () => void } {
  const [state, setState] = useState<AsyncState<T>>({
    data: null,
    loading: true,
    error: null,
  });
  const [nonce, setNonce] = useState(0);

  const reload = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    const controller = new AbortController();
    let active = true;

    setState((prev) => ({ ...prev, loading: true, error: null }));

    fetcher(controller.signal)
      .then((data) => {
        if (active) setState({ data, loading: false, error: null });
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted || !active) return;
        const error = err instanceof Error ? err : new Error(String(err));
        setState({ data: null, loading: false, error });
      });

    return () => {
      active = false;
      controller.abort();
    };
  }, [fetcher, nonce]);

  return { ...state, reload };
}
