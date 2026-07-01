import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import { initI18n } from "./i18n";
import "./index.css";
import App from "./App";

async function bootstrap() {
  const rootEl = document.getElementById("root")!;
  rootEl.className =
    "flex min-h-dvh items-center justify-center bg-cream text-ink-soft";
  rootEl.textContent = "Loading…";

  await initI18n();

  createRoot(rootEl).render(
    <StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </StrictMode>,
  );
}

void bootstrap();
