/**
 * Giriş noktası.
 *
 * Tek pencere, tek kök: yardımcı pencereler (çeviri overlay'i ve alan
 * seçici) kaldırıldığında `?pencere=` yönlendirmesinin de dayanağı kalmadı.
 */

import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
