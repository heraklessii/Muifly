//! Sunum penceresi: ölçeklenmiş kareyi ekrana çizen katman.
//!
//! # Pencere neden böyle
//!
//! Ölçekleme penceresi oyunun **üstünde** duruyor ama oyunun yerini
//! almıyor. Dört üslup bayrağı bunu sağlıyor ve dördü de bilinçli:
//!
//! - `WS_EX_NOACTIVATE` — pencere odağı ALMIYOR. Alsaydı oyun arka plana
//!   düşer, bazı oyunlar kendini duraklatır, bazıları fare yakalamasını
//!   bırakırdı.
//! - `WS_EX_TRANSPARENT` — fare tıklamaları pencereden geçip oyuna
//!   gidiyor. Bu olmasaydı ölçekleme açıkken oyun oynanamazdı.
//! - `WS_EX_TOOLWINDOW` — Alt+Tab listesinde görünmüyor. Kullanıcının
//!   sekmeleri arasında bizim penceremizin durması gürültü olurdu.
//! - `WS_EX_TOPMOST` — üstte kalıyor; ölçekleyicinin varlık sebebi bu.
//!
//! `WS_EX_LAYERED` **kullanılmıyor**: katmanlı pencereler DXGI'nin çevirme
//! (flip) modeliyle çalışmıyor ve çevirme modeli olmadan her karede fazladan
//! bir kopya oluşurdu — yani ölçmeye çalıştığımız gecikmenin kendisi artardı.
//!
//! # Oyun sürecine dokunulmuyor
//!
//! Bu dosyada oyun süreciyle ilgili tek bir çağrı yok: pencere açılıyor,
//! yakalanan doku çiziliyor, bitiyor. Tasarım ilkesi 3'ün sunum tarafındaki
//! karşılığı.

use crate::scaling::algoritma::Algoritma;

/// Gölgelendirici kaynağı — ikiliye gömülü.
///
/// Dışarıdan okunan bir dosya değil: kullanıcının makinesinde
/// değiştirilebilen bir gölgelendirici, "ne çalıştığını biliyoruz" iddiasını
/// bozardı. Derleme çalışma zamanında yapılıyor (`D3DCompile`) çünkü
/// önceden derlenmiş bayt kodu sürücü/özellik seviyesine bağlı.
pub const GOLGELENDIRICI: &str = include_str!("olcekleme.hlsl");

/// Bir algoritmanın gölgelendiricideki giriş noktası.
pub fn giris_noktasi(a: Algoritma) -> &'static str {
    match a {
        Algoritma::TamSayi => "PS_TamSayi",
        Algoritma::Bilinear => "PS_Bilinear",
        Algoritma::Lanczos => "PS_Lanczos",
        Algoritma::Xbr => "PS_Xbr",
    }
}

#[cfg(windows)]
pub use win::SunumPenceresi;

#[cfg(windows)]
mod win {
    use super::*;
    use crate::scaling::algoritma::{en_boy_koru, tam_kat, Alan};
    use crate::scaling::yakalama::Engel;
    use windows::core::{Interface, PCSTR, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Direct3D::Fxc::{
        D3DCompile, D3DCOMPILE_ENABLE_STRICTNESS, D3DCOMPILE_OPTIMIZATION_LEVEL3,
    };
    use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST};
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    const SINIF_ADI: PCWSTR = windows::core::w!("MuiflyOlceklemePenceresi");

    /// Gölgelendiriciye giden sabitler. 16 bayta hizalı olmak zorunda.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Sabitler {
        kaynak_ofset: [f32; 2],
        kaynak_boyut: [f32; 2],
        doku_boyut: [f32; 2],
        hedef_boyut: [f32; 2],
    }

    fn sistem(e: windows::core::Error) -> Engel {
        Engel::Sistem(e.code().0)
    }

    /// Pencere yordamı.
    ///
    /// `WM_NCHITTEST` açıkça `HTTRANSPARENT` dönüyor: `WS_EX_TRANSPARENT`
    /// çoğu durumda yetiyor ama bazı sürücü/uyumluluk yollarında pencere
    /// yine de fareyi yakalayabiliyor. Oyun oynanamaz hale gelmesindense
    /// iki kez söylemek ucuz.
    unsafe extern "system" fn pencere_proc(
        pencere: HWND,
        mesaj: u32,
        w: WPARAM,
        l: LPARAM,
    ) -> LRESULT {
        match mesaj {
            WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            // Ekran koruyucu / güç durumu isteklerini engellemiyoruz:
            // ölçekleme açık diye makinenin uyumasını yasaklamak,
            // kullanıcının ayarını sessizce ezmek olurdu.
            _ => DefWindowProcW(pencere, mesaj, w, l),
        }
    }

    fn sinifi_kaydet() -> Result<HINSTANCE, Engel> {
        static BIR_KEZ: std::sync::Once = std::sync::Once::new();
        let ornek: HINSTANCE = unsafe { GetModuleHandleW(None).map_err(sistem)? }.into();
        BIR_KEZ.call_once(|| unsafe {
            let sinif = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(pencere_proc),
                hInstance: ornek,
                lpszClassName: SINIF_ADI,
                ..Default::default()
            };
            RegisterClassExW(&sinif);
        });
        Ok(ornek)
    }

    /// Açık bir sunum penceresi ve boru hattı.
    pub struct SunumPenceresi {
        pencere: HWND,
        /// Pencere yakalamanın dışına çıkarılabildi mi?
        ///
        /// `false` ise ölçekleme kendi çıktısını yakalıyor demektir ve
        /// kullanıcıya bunun söylenmesi gerekiyor.
        pub yakalamadan_gizli: bool,
        zincir: IDXGISwapChain1,
        hedef: Option<ID3D11RenderTargetView>,
        koseler: ID3D11VertexShader,
        pikseller: [ID3D11PixelShader; 4],
        ornekleyici_nokta: ID3D11SamplerState,
        ornekleyici_dogrusal: ID3D11SamplerState,
        sabitler: ID3D11Buffer,
        baglam: ID3D11DeviceContext,
        pub genislik: u32,
        pub yukseklik: u32,
    }

    impl SunumPenceresi {
        /// Verilen ekran dikdörtgeninde pencereyi açar.
        ///
        /// Cihaz **yakalamanın cihazı**: iki ayrı cihaz her karede paylaşımlı
        /// doku aktarımı demek olurdu.
        pub fn ac(
            cihaz: &ID3D11Device,
            baglam: &ID3D11DeviceContext,
            x: i32,
            y: i32,
            genislik: u32,
            yukseklik: u32,
        ) -> Result<Self, Engel> {
            let ornek = sinifi_kaydet()?;
            unsafe {
                let pencere = CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                    SINIF_ADI,
                    windows::core::w!("Muifly ölçekleme"),
                    WS_POPUP,
                    x,
                    y,
                    genislik as i32,
                    yukseklik as i32,
                    None,
                    None,
                    Some(ornek),
                    None,
                )
                .map_err(sistem)?;

                let dxgi_cihaz: IDXGIDevice = cihaz.cast().map_err(sistem)?;
                let adaptor = dxgi_cihaz.GetAdapter().map_err(sistem)?;
                let fabrika: IDXGIFactory2 = adaptor.GetParent().map_err(sistem)?;

                let tanim = DXGI_SWAP_CHAIN_DESC1 {
                    Width: genislik,
                    Height: yukseklik,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    // Çevirme modeli: sunumda fazladan kopya yok.
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    Scaling: DXGI_SCALING_NONE,
                    AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                    ..Default::default()
                };
                let zincir = fabrika
                    .CreateSwapChainForHwnd(cihaz, pencere, &tanim, None, None)
                    .map_err(sistem)?;

                // Alt+Enter'ı DXGI'ye bırakmıyoruz: kullanıcı oyunda tam
                // ekrana geçmek isterken bizim penceremizin mod
                // değiştirmesi, yakalamayı da düşürürdü.
                let _ = fabrika.MakeWindowAssociation(pencere, DXGI_MWA_NO_ALT_ENTER);

                let koseler = kose_golgelendirici(cihaz)?;
                let pikseller = [
                    piksel_golgelendirici(cihaz, Algoritma::TamSayi)?,
                    piksel_golgelendirici(cihaz, Algoritma::Bilinear)?,
                    piksel_golgelendirici(cihaz, Algoritma::Lanczos)?,
                    piksel_golgelendirici(cihaz, Algoritma::Xbr)?,
                ];

                let ornekleyici_nokta = ornekleyici(cihaz, D3D11_FILTER_MIN_MAG_MIP_POINT)?;
                let ornekleyici_dogrusal = ornekleyici(cihaz, D3D11_FILTER_MIN_MAG_MIP_LINEAR)?;

                let sabit_tanim = D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<Sabitler>() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                };
                let mut sabitler: Option<ID3D11Buffer> = None;
                cihaz
                    .CreateBuffer(&sabit_tanim, None, Some(&mut sabitler))
                    .map_err(sistem)?;
                let sabitler = sabitler.ok_or(Engel::Desteklenmeyen)?;

                // Kendi penceremizi yakalamanın dışına çıkarıyoruz.
                //
                // Bu satır olmadan modül ÇALIŞMAZ: masaüstü çoğaltması
                // bileşiklenmiş masaüstünü veriyor ve bizim pencere de onun
                // bir parçası. Kendi çıktımızı yakalayıp yeniden çizerdik —
                // ekranda birbirinin içine giren bir tünel görünürdü.
                //
                // `WDA_EXCLUDEFROMCAPTURE` Windows 10 2004 ile geldi. Daha
                // eskisinde çağrı başarısız oluyor; hata yutulmuyor,
                // `yakalamadan_gizli` ile dışarı veriliyor ve çağıran
                // kullanıcıya söylüyor. Sessizce devam etmek, kullanıcıya
                // bozuk bir görüntü gösterip sebebini söylememek olurdu.
                let yakalamadan_gizli =
                    SetWindowDisplayAffinity(pencere, WDA_EXCLUDEFROMCAPTURE).is_ok();
                if !yakalamadan_gizli {
                    log::warn!(
                        "pencere yakalamadan gizlenemedi; bu Windows sürümünde \
                         ölçekleme kendi çıktısını yakalayabilir"
                    );
                }

                let mut p = Self {
                    pencere,
                    yakalamadan_gizli,
                    zincir,
                    hedef: None,
                    koseler,
                    pikseller,
                    ornekleyici_nokta,
                    ornekleyici_dogrusal,
                    sabitler,
                    baglam: baglam.clone(),
                    genislik,
                    yukseklik,
                };
                p.hedefi_kur()?;
                // SW_SHOWNA: göster ama odak verme.
                let _ = ShowWindow(pencere, SW_SHOWNA);
                Ok(p)
            }
        }

        fn hedefi_kur(&mut self) -> Result<(), Engel> {
            unsafe {
                let arka: ID3D11Texture2D = self.zincir.GetBuffer(0).map_err(sistem)?;
                let cihaz = self.baglam.GetDevice().map_err(sistem)?;
                let mut gorunum: Option<ID3D11RenderTargetView> = None;
                cihaz
                    .CreateRenderTargetView(&arka, None, Some(&mut gorunum))
                    .map_err(sistem)?;
                self.hedef = gorunum;
                Ok(())
            }
        }

        /// Bekleyen pencere mesajlarını işler.
        ///
        /// Mesaj kuyruğu boşaltılmazsa Windows pencereyi "yanıt vermiyor"
        /// sayıp gri bir kopyayla değiştiriyor — ekranın ortasında donmuş
        /// bir görüntü demek.
        pub fn mesajlari_isle(&self) -> bool {
            unsafe {
                let mut mesaj = MSG::default();
                while PeekMessageW(&mut mesaj, None, 0, 0, PM_REMOVE).as_bool() {
                    if mesaj.message == WM_QUIT {
                        return false;
                    }
                    let _ = TranslateMessage(&mesaj);
                    DispatchMessageW(&mesaj);
                }
                true
            }
        }

        /// Yakalanan dokuyu ölçekleyip sunar.
        ///
        /// `kaynak_g/kaynak_y` yakalanan görüntünün boyutu; en/boy oranı
        /// korunarak ortalanıyor, kalan yer siyah temizleniyor.
        pub fn ciz(
            &mut self,
            gorunum: &ID3D11ShaderResourceView,
            alan: Alan,
            doku_g: u32,
            doku_y: u32,
            algo: Algoritma,
        ) -> Result<(), Engel> {
            let Some(hedef) = self.hedef.clone() else {
                return Err(Engel::Desteklenmeyen);
            };
            let (kaynak_g, kaynak_y) = (alan.genislik, alan.yukseklik);
            // Tam sayı ölçekleme dışındakiler ekrana sığan en büyük
            // dikdörtgeni alıyor; tam sayı, kata yuvarlanmış olanı.
            let yerlesim = match algo {
                Algoritma::TamSayi => {
                    let kat = tam_kat(kaynak_g, kaynak_y, self.genislik, self.yukseklik);
                    en_boy_koru(
                        kaynak_g,
                        kaynak_y,
                        (kaynak_g * kat).min(self.genislik),
                        (kaynak_y * kat).min(self.yukseklik),
                    )
                }
                _ => en_boy_koru(kaynak_g, kaynak_y, self.genislik, self.yukseklik),
            };
            let x_ofset = (self.genislik.saturating_sub(yerlesim.genislik)) / 2;
            let y_ofset = (self.yukseklik.saturating_sub(yerlesim.yukseklik)) / 2;

            unsafe {
                let sabit = Sabitler {
                    kaynak_ofset: [alan.x as f32, alan.y as f32],
                    kaynak_boyut: [kaynak_g as f32, kaynak_y as f32],
                    doku_boyut: [doku_g as f32, doku_y as f32],
                    hedef_boyut: [yerlesim.genislik as f32, yerlesim.yukseklik as f32],
                };
                self.baglam.UpdateSubresource(
                    &self.sabitler,
                    0,
                    None,
                    &sabit as *const _ as *const _,
                    0,
                    0,
                );

                self.baglam
                    .OMSetRenderTargets(Some(&[Some(hedef.clone())]), None);
                self.baglam
                    .ClearRenderTargetView(&hedef, &[0.0, 0.0, 0.0, 1.0]);

                let alan = D3D11_VIEWPORT {
                    TopLeftX: x_ofset as f32,
                    TopLeftY: y_ofset as f32,
                    Width: yerlesim.genislik as f32,
                    Height: yerlesim.yukseklik as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                self.baglam.RSSetViewports(Some(&[alan]));
                self.baglam
                    .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                self.baglam.VSSetShader(&self.koseler, None);
                self.baglam
                    .PSSetShader(&self.pikseller[algo as usize], None);
                self.baglam
                    .PSSetShaderResources(0, Some(&[Some(gorunum.clone())]));
                self.baglam.PSSetSamplers(
                    0,
                    Some(&[
                        Some(self.ornekleyici_nokta.clone()),
                        Some(self.ornekleyici_dogrusal.clone()),
                    ]),
                );
                self.baglam
                    .PSSetConstantBuffers(0, Some(&[Some(self.sabitler.clone())]));
                self.baglam.Draw(3, 0);

                // Dokuyu çözüyoruz: bir sonraki yakalama aynı dokuya
                // kopyalayacak ve bağlıyken kopyalamak sürücüde sessiz bir
                // çözme + uyarı üretirdi.
                self.baglam.PSSetShaderResources(0, Some(&[None]));

                // Dikey eşitlemeyle sunum: yırtılma (tearing) ölçekleme
                // penceresinde oyununkinden daha rahatsız edici, çünkü
                // görüntü zaten bir kare gecikmiş oluyor.
                self.zincir.Present(1, DXGI_PRESENT(0)).ok().map_err(sistem)
            }
        }
    }

    impl Drop for SunumPenceresi {
        fn drop(&mut self) {
            // Pencere kapatılmazsa ekranda üstte duran boş bir siyah
            // dikdörtgen kalırdı — geri alınabilirlik ilkesinin bu
            // modüldeki en görünür hali.
            unsafe {
                let _ = DestroyWindow(self.pencere);
            }
        }
    }

    fn ornekleyici(
        cihaz: &ID3D11Device,
        filtre: D3D11_FILTER,
    ) -> Result<ID3D11SamplerState, Engel> {
        let tanim = D3D11_SAMPLER_DESC {
            Filter: filtre,
            // Kenarda kırpma: sarma (wrap) olsaydı görüntünün sağ kenarında
            // sol kenarın renkleri belirirdi.
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        unsafe {
            let mut o: Option<ID3D11SamplerState> = None;
            cihaz
                .CreateSamplerState(&tanim, Some(&mut o))
                .map_err(sistem)?;
            o.ok_or(Engel::Desteklenmeyen)
        }
    }

    fn derle(giris: &str, hedef: &str) -> Result<ID3DBlob, Engel> {
        unsafe {
            let giris_c = std::ffi::CString::new(giris).unwrap();
            let hedef_c = std::ffi::CString::new(hedef).unwrap();
            let ad_c = std::ffi::CString::new("olcekleme.hlsl").unwrap();
            let mut kod: Option<ID3DBlob> = None;
            let mut hatalar: Option<ID3DBlob> = None;
            let sonuc = D3DCompile(
                GOLGELENDIRICI.as_ptr() as *const _,
                GOLGELENDIRICI.len(),
                PCSTR(ad_c.as_ptr() as *const u8),
                None,
                None,
                PCSTR(giris_c.as_ptr() as *const u8),
                PCSTR(hedef_c.as_ptr() as *const u8),
                D3DCOMPILE_ENABLE_STRICTNESS | D3DCOMPILE_OPTIMIZATION_LEVEL3,
                0,
                &mut kod,
                Some(&mut hatalar),
            );
            if let Err(e) = sonuc {
                // Derleyici metnini günlüğe yazıyoruz: bu bir geliştirme
                // hatası ve kullanıcıya gösterilecek bir cümlesi yok, ama
                // sessizce yutulursa hiç bulunamaz.
                if let Some(h) = hatalar {
                    let metin = std::slice::from_raw_parts(
                        h.GetBufferPointer() as *const u8,
                        h.GetBufferSize(),
                    );
                    log::error!(
                        "gölgelendirici derlenemedi ({giris}): {}",
                        String::from_utf8_lossy(metin)
                    );
                }
                return Err(sistem(e));
            }
            kod.ok_or(Engel::Desteklenmeyen)
        }
    }

    fn bayt_kodu(blob: &ID3DBlob) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
        }
    }

    fn kose_golgelendirici(cihaz: &ID3D11Device) -> Result<ID3D11VertexShader, Engel> {
        let blob = derle("VS", "vs_5_0")?;
        unsafe {
            let mut g: Option<ID3D11VertexShader> = None;
            cihaz
                .CreateVertexShader(bayt_kodu(&blob), None, Some(&mut g))
                .map_err(sistem)?;
            g.ok_or(Engel::Desteklenmeyen)
        }
    }

    fn piksel_golgelendirici(
        cihaz: &ID3D11Device,
        algo: Algoritma,
    ) -> Result<ID3D11PixelShader, Engel> {
        let blob = derle(giris_noktasi(algo), "ps_5_0")?;
        unsafe {
            let mut g: Option<ID3D11PixelShader> = None;
            cihaz
                .CreatePixelShader(bayt_kodu(&blob), None, Some(&mut g))
                .map_err(sistem)?;
            g.ok_or(Engel::Desteklenmeyen)
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::scaling::algoritma;

    #[test]
    fn her_algoritmanin_giris_noktasi_var() {
        for a in Algoritma::hepsi() {
            let giris = giris_noktasi(a);
            assert!(
                GOLGELENDIRICI.contains(&format!("{giris}(VSCikti")),
                "{giris} gölgelendiricide tanımlı değil"
            );
        }
    }

    /// CPU referansı ile gölgelendiricinin sabitleri aynı olmalı.
    ///
    /// Aynı matematiğin iki yerde durması `decisions.md` #32'de kabul edilen
    /// bir borç. Bu test o borcun en kolay kaçan kısmını — sayıların
    /// birbirinden sessizce ayrılmasını — tutuyor. Satır satır eşitlik
    /// KANITLAMIYOR; kanıtlayan tek şey gerçek bir ekranda karşılaştırma
    /// olurdu (`tasks.md` → Faz 3 elle doğrulama).
    #[test]
    fn golgelendirici_sabitleri_referansla_ayni() {
        for (ad, deger) in [
            ("LANCZOS_A", format!("{:.1}", algoritma::LANCZOS_A as f32)),
            ("XBR_GUCLU_KAT", format!("{:.1}", algoritma::XBR_GUCLU_KAT)),
            (
                "XBR_ZAYIF_KARISIM",
                format!("{:.1}", algoritma::XBR_ZAYIF_KARISIM),
            ),
        ] {
            let beklenen = format!("static const float {ad} = {deger};");
            assert!(
                GOLGELENDIRICI.contains(&beklenen),
                "gölgelendiricide beklenen satır yok: {beklenen}"
            );
        }
    }

    #[test]
    fn golgelendiricide_yuv_agirliklari_referansla_ayni() {
        // Kenar tespitinin ağırlıkları da ikisinde aynı olmak zorunda;
        // farklı olsalar xBR iki tarafta farklı kenar bulurdu.
        for parca in ["48.0 * abs(dy)", "7.0 * abs(du)", "6.0 * abs(dv)"] {
            assert!(GOLGELENDIRICI.contains(parca), "eksik: {parca}");
        }
    }

    #[test]
    fn giris_noktalari_ayri() {
        let mut adlar: Vec<&str> = Algoritma::hepsi()
            .iter()
            .map(|a| giris_noktasi(*a))
            .collect();
        adlar.sort_unstable();
        adlar.dedup();
        assert_eq!(adlar.len(), 4);
    }
}
