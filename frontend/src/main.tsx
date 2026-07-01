import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import { initI18n } from "./i18n";
import "./index.css";
import App from "./App";

async function bootstrap() {
  const root = createRoot(document.getElementById("root")!);

  root.render(
    <div className="flex min-h-dvh items-center justify-center bg-cream text-ink-soft">
      <p className="text-sm font-semibold">Loading…</p>
    </div>,
  );

  await initI18n();

  root.render(
    <StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </StrictMode>,
  );
}

void bootstrap();
