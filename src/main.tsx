/**
 * Giriş noktası ve pencere yönlendirmesi.
 *
 * Üç Tauri penceresi de aynı `index.html`i yüklüyor; hangisinin ne
 * göstereceğini adres satırındaki `?pencere=` belirliyor. Ayrı HTML
 * dosyaları üretmek yerine bu seçildi: üçü de aynı tasarım jetonlarını,
 * aynı `api` köprüsünü ve aynı tipleri kullanıyor, yani ayırmanın
 * kazandıracağı bir şey yok — bakılacak ikinci bir yapı dışında.
 */

import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { AlanSecici } from "./components/AlanSecici";
import { CeviriOverlay } from "./components/CeviriOverlay";
import "./styles.css";

const parametreler = new URLSearchParams(window.location.search);
const pencere = parametreler.get("pencere");

/** Bu pencerenin göstereceği şey. Tanınmayan bir değer ana uygulamaya düşüyor. */
function Kok() {
  switch (pencere) {
    case "ceviri-overlay":
      return <CeviriOverlay />;
    case "ceviri-alan":
      return <AlanSecici kimlik={parametreler.get("kimlik") ?? ""} />;
    default:
      return <App />;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Kok />
  </React.StrictMode>,
);
