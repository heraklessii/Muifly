/**
 * Satır içi SVG ikonlar.
 *
 * Bir ikon kütüphanesi eklenmedi: yirmi kadar ikon için bir paket bağımlılığı
 * (ve binary boyutu) taşımak, "hafif araç" vaadiyle çelişirdi. Hepsi
 * `currentColor` kullanıyor, boyut CSS'ten geliyor (`styles.css` → `svg`).
 *
 * Çizim kuralı: 24×24 kutu, 1.8 kalınlık, yuvarlak uç. Tek istisna marka
 * işareti — o dolgu da kullanıyor.
 */

type P = { className?: string };

const ortak = {
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.8,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

/** Marka: yükselen bir çizgi + hız işareti. */
export const IconMuifly = ({ className }: P) => (
  <svg viewBox="0 0 24 24" fill="none" className={className}>
    <path
      d="M3 17.5 9 11l3.2 3.2L20 6"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <path d="M20.5 5.5h-5l5 5z" fill="currentColor" />
    <path
      d="M3 21h18"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      opacity="0.35"
    />
  </svg>
);

/* --- Gezinme ------------------------------------------------------------ */

export const IconDurum = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M3 12h3.5l2.5 6.5L13 5l2.5 7H21" />
  </svg>
);

export const IconProfil = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <rect x="3" y="4" width="18" height="16" rx="3" />
    <path d="M3 9h18M7.5 13.5h6M7.5 16.5h3" />
  </svg>
);

export const IconAg = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3a15 15 0 0 1 0 18a15 15 0 0 1 0-18" />
  </svg>
);

export const IconGunluk = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <rect x="4" y="3.5" width="16" height="17" rx="3" />
    <path d="M8 9h8M8 13h8M8 17h4" />
  </svg>
);

export const IconAyarlar = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="3.2" />
    <path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1" />
  </svg>
);

/* --- Eylemler ----------------------------------------------------------- */

export const IconOynat = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M7 4.5 19 12 7 19.5z" />
  </svg>
);

export const IconGeriAl = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M3 9h11a5 5 0 0 1 0 10H9" />
    <path d="M7 5 3 9l4 4" />
  </svg>
);

export const IconArti = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 5v14M5 12h14" />
  </svg>
);

export const IconCop = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M4 7h16M9 7V5h6v2M6.5 7l1 13h9l1-13" />
  </svg>
);

export const IconKapat = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M6 6l12 12M18 6 6 18" />
  </svg>
);

export const IconDuzenle = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M4 20h4l10.5-10.5a2.1 2.1 0 0 0-3-3L5 17z" />
    <path d="M14.5 6.5l3 3" />
  </svg>
);

/** İçe aktar: dosyadan programa doğru ok. */
export const IconIceAktar = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 3v12" />
    <path d="m7.5 10.5 4.5 4.5 4.5-4.5" />
    <path d="M4 20h16" />
  </svg>
);

/** Dışa aktar: programdan dosyaya doğru ok. */
export const IconDisaAktar = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 15V3" />
    <path d="m7.5 7.5 4.5-4.5 4.5 4.5" />
    <path d="M4 20h16" />
  </svg>
);

export const IconYenile = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M20 11a8 8 0 1 0-2.3 6.3" />
    <path d="M20 5v6h-6" />
  </svg>
);

/** Oyun kütüphanesi: yan yana duran kapaklar. */
export const IconKutuphane = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <rect x="3" y="4" width="6" height="16" rx="1.5" />
    <rect x="11" y="4" width="6" height="16" rx="1.5" />
    <path d="M19.5 5.5l1.6 14" />
  </svg>
);

export const IconArama = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="11" cy="11" r="6.5" />
    <path d="m16 16 4.5 4.5" />
  </svg>
);

/* --- Durum bildirimi ---------------------------------------------------- */

export const IconUyari = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 4 2.5 20h19z" />
    <path d="M12 10v4M12 17h.01" />
  </svg>
);

export const IconBilgi = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5M12 8h.01" />
  </svg>
);

export const IconTik = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="m8 12.2 2.7 2.8L16 9.5" />
  </svg>
);

export const IconHata = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="m9 9 6 6M15 9l-6 6" />
  </svg>
);

export const IconYok = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="M6 6l12 12" />
  </svg>
);

export const IconKalkan = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 3l7 3v5c0 4.5-3 8.2-7 10-4-1.8-7-5.5-7-10V6z" />
  </svg>
);

/* --- Tema --------------------------------------------------------------- */

export const IconAy = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5" />
  </svg>
);

export const IconGunes = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.5 4.5l1.5 1.5M18 18l1.5 1.5M2 12h2M20 12h2M4.5 19.5 6 18M18 6l1.5-1.5" />
  </svg>
);

/* --- Yön (öncesi/sonrası karşılaştırması) -------------------------------
 *
 * Yönün rengi ve anlamı `lib/format.ts` → `yon` fonksiyonundan geliyor.
 * Burada yalnızca üç şekil var: aşağı, yukarı, yatay. Bir "iyileşme"
 * ikonu YOK — hangi yönün iyi olduğu ölçüme göre değişir.
 * ------------------------------------------------------------------------ */

export const IconAsagi = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 5v14M6.5 13.5 12 19l5.5-5.5" />
  </svg>
);

export const IconYukari = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M12 19V5M6.5 10.5 12 5l5.5 5.5" />
  </svg>
);

export const IconEsit = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M5 9.5h14M5 14.5h14" />
  </svg>
);

export const IconOkSag = ({ className }: P) => (
  <svg {...ortak} className={className}>
    <path d="M4 12h15M13.5 6.5 19 12l-5.5 5.5" />
  </svg>
);
