import React from "react";
import ReactDOM from "react-dom/client";

// Typeface — DM Sans (Majoris) is the single UI typeface, bundled (see CLAUDE.md "Stack").
import "@fontsource-variable/dm-sans/opsz.css";
import "@fontsource-variable/dm-sans/opsz-italic.css";

// Design tokens first, then base styles.
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/components.css";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import CrashScreen from "./kernel/shell/CrashScreen";
import { router } from "./router";

const queryClient = new QueryClient();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <CrashScreen>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </CrashScreen>
  </React.StrictMode>,
);
