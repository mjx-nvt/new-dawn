import React from "react";
import ReactDOM from "react-dom/client";

// Typeface — Be Vietnam Pro is the single typeface for every workspace (see CLAUDE.md "Stack").
import "@fontsource/be-vietnam-pro/400.css";
import "@fontsource/be-vietnam-pro/400-italic.css";
import "@fontsource/be-vietnam-pro/500.css";
import "@fontsource/be-vietnam-pro/600.css";
import "@fontsource/be-vietnam-pro/700.css";

// Design tokens first, then base styles.
import "./styles/tokens.css";
import "./styles/base.css";

import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
