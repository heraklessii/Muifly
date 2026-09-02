//! Kare üretimi (Faz 4): iki gerçek kare arasına bir kare koyan boru hattı.
//!
//! Algoritma `hareket.rs`'te tanımlı ve orada test ediliyor; burası onun
//! D3D11 üzerindeki gerçek zamanlı yolu. Gölgelendirici `uretim.hlsl`.
//!
//! # Bedeli neden en başta yazıyor
//!
//! Ara kare, iki **gerçek** kare arasına konuyor. Yani ikinci gerçek kare
//! üretilmeden ara kare hesaplanamıyor ve ikinci kare, elde tutulup bir
//! sunum turu geç gösteriliyor. Bu, kaynak 60 kare/s ise en az bir kare —
//! yaklaşık 17 ms — gecikme demek ve bu gecikme algoritmanın hızıyla
//! **azalmıyor**; sonsuz hızlı bir GPU'da bile duruyor. Kare üretiminin
//! doğasında olan bedel bu.
//!
//! Bu yüzden özellik rekabetçi modda kapalı (profil şeması), varsayılan
//! kapalı ve arayüzde bedeli yazmadan açılamıyor. `gecikme.rs`'in ölçtüğü
//! sayılar burada bir süs değil, özelliğin açılma şartı (karar #35).
//!
//! # Ekran yenileme hızı
//!
//! Kare üretimi, ekranın kaynaktan belirgin olarak hızlı olmasını
//! gerektiriyor: 60 Hz ekranda 60 kare/s kaynağa ara kare eklemek, kaynak
//! karelerinin yarısını beklemeye almak demek — yani görüntüyü hızlandırmak
//! değil yavaşlatmak. Döngü bunu ölçüp söylüyor (`mod.rs`).

use crate::scaling::hareket;

/// Gölgelendirici kaynağı — ikiliye gömülü (`sunum.rs` ile aynı gerekçe).
pub const GOLGELENDIRICI: &str = include_str!("uretim.hlsl");

#[cfg(windows)]
pub use win::{derleme_denemesi, Uretici};

#[cfg(windows)]
mod win {
    use super::*;
    use crate::scaling::yakalama::Engel;
    use windows::core::PCSTR;
    use windows::Win32::Graphics::Direct3D::Fxc::{
        D3DCompile, D3DCOMPILE_ENABLE_STRICTNESS, D3DCOMPILE_OPTIMIZATION_LEVEL3,
    };
    use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST};
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;

    fn sistem(e: windows::core::Error) -> Engel {
        Engel::Sistem(e.code().0)
    }

    /// Gölgelendiriciye giden sabitler. 16 bayta hizalı olmak zorunda.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Sabitler {
        luma_boyut: [f32; 2],
        izgara_boyut: [f32; 2],
        tam_boyut: [f32; 2],
        seviye: i32,
        zaman: f32,
    }

    /// Çizilebilir ve okunabilir bir doku.
    struct Hedef {
        _doku: ID3D11Texture2D,
        gorunum: ID3D11ShaderResourceView,
        cizim: ID3D11RenderTargetView,
        genislik: u32,
        yukseklik: u32,
    }

    impl Hedef {
        fn yeni(
            cihaz: &ID3D11Device,
            genislik: u32,
            yukseklik: u32,
            bicim: DXGI_FORMAT,
        ) -> Result<Self, Engel> {
            let tanim = D3D11_TEXTURE2D_DESC {
                Width: genislik.max(1),
                Height: yukseklik.max(1),
                MipLevels: 1,
                ArraySize: 1,
                Format: bicim,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                ..Default::default()
            };
            unsafe {
                let mut doku: Option<ID3D11Texture2D> = None;
                cihaz
                    .CreateTexture2D(&tanim, None, Some(&mut doku))
                    .map_err(sistem)?;
                let doku = doku.ok_or(Engel::Desteklenmeyen)?;
                let mut gorunum: Option<ID3D11ShaderResourceView> = None;
                cihaz
                    .CreateShaderResourceView(&doku, None, Some(&mut gorunum))
                    .map_err(sistem)?;
                let mut cizim: Option<ID3D11RenderTargetView> = None;
                cihaz
                    .CreateRenderTargetView(&doku, None, Some(&mut cizim))
                    .map_err(sistem)?;
                Ok(Self {
                    _doku: doku,
                    gorunum: gorunum.ok_or(Engel::Desteklenmeyen)?,
                    cizim: cizim.ok_or(Engel::Desteklenmeyen)?,
                    genislik: tanim.Width,
                    yukseklik: tanim.Height,
                })
            }
        }
    }

    /// Yalnızca kopyalanan ve okunan doku (çizim hedefi değil).
    struct Kopya {
        doku: ID3D11Texture2D,
        gorunum: ID3D11ShaderResourceView,
    }

    impl Kopya {
        fn yeni(cihaz: &ID3D11Device, genislik: u32, yukseklik: u32) -> Result<Self, Engel> {
            let tanim = D3D11_TEXTURE2D_DESC {
                Width: genislik.max(1),
                Height: yukseklik.max(1),
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                ..Default::default()
            };
            unsafe {
                let mut doku: Option<ID3D11Texture2D> = None;
                cihaz
                    .CreateTexture2D(&tanim, None, Some(&mut doku))
                    .map_err(sistem)?;
                let doku = doku.ok_or(Engel::Desteklenmeyen)?;
                let mut gorunum: Option<ID3D11ShaderResourceView> = None;
                cihaz
                    .CreateShaderResourceView(&doku, None, Some(&mut gorunum))
                    .map_err(sistem)?;
                Ok(Self {
                    doku,
                    gorunum: gorunum.ok_or(Engel::Desteklenmeyen)?,
                })
            }
        }
    }

    fn derle(giris: &str, hedef: &str) -> Result<ID3DBlob, Engel> {
        unsafe {
            let giris_c = std::ffi::CString::new(giris).unwrap();
            let hedef_c = std::ffi::CString::new(hedef).unwrap();
            let ad_c = std::ffi::CString::new("uretim.hlsl").unwrap();
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
                if let Some(h) = hatalar {
                    let metin = std::slice::from_raw_parts(
                        h.GetBufferPointer() as *const u8,
                        h.GetBufferSize(),
                    );
                    log::error!(
                        "üretim gölgelendiricisi derlenemedi ({giris}): {}",
                        String::from_utf8_lossy(metin)
                    );
                }
                return Err(sistem(e));
            }
            kod.ok_or(Engel::Desteklenmeyen)
        }
    }

    /// Bütün giriş noktalarını derler; hiçbir GPU nesnesi oluşturmaz.
    ///
    /// `D3DCompile` bir cihaz istemiyor — bu yüzden gölgelendiricinin
    /// derlendiği **normal bir birim testiyle** doğrulanabiliyor
    /// (`testler::golgelendirici_derleniyor`). Bu önemli: HLSL hatası
    /// yoksa çalışma zamanına kadar görünmez ve kullanıcı özelliği
    /// açtığında ortaya çıkardı.
    pub fn derleme_denemesi() -> Result<(), Engel> {
        derle("VS", "vs_5_0")?;
        for giris in ["PS_Luma", "PS_Indirge", "PS_Hareket", "PS_Duzelt", "PS_Ara"] {
            derle(giris, "ps_5_0")?;
        }
        Ok(())
    }

    fn bayt_kodu(blob: &ID3DBlob) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
        }
    }

    fn piksel(cihaz: &ID3D11Device, giris: &str) -> Result<ID3D11PixelShader, Engel> {
        let blob = derle(giris, "ps_5_0")?;
        unsafe {
            let mut g: Option<ID3D11PixelShader> = None;
            cihaz
                .CreatePixelShader(bayt_kodu(&blob), None, Some(&mut g))
                .map_err(sistem)?;
            g.ok_or(Engel::Desteklenmeyen)
        }
    }

    /// Kare üretimi boru hattı.
    ///
    /// Bütün dokular açılışta bir kez ayrılıyor. Kare başına ayırma
    /// yapılsaydı sürücü her karede bellek yönetimine girer ve ölçtüğümüz
    /// sürelere kendi gürültüsünü katardı.
    pub struct Uretici {
        baglam: ID3D11DeviceContext,
        koseler: ID3D11VertexShader,
        ps_luma: ID3D11PixelShader,
        ps_indirge: ID3D11PixelShader,
        ps_hareket: ID3D11PixelShader,
        ps_duzelt: ID3D11PixelShader,
        ps_ara: ID3D11PixelShader,
        sabitler: ID3D11Buffer,
        ornekleyici_nokta: ID3D11SamplerState,
        ornekleyici_dogrusal: ID3D11SamplerState,

        /// `luma[yuva][seviye]` — yuva 0/1 arasında gidip geliyor.
        luma: [[Hedef; hareket::SEVIYE]; 2],
        /// Hareket ızgarası, geçişler arasında gidip gelen iki hedef.
        mv: [Hedef; 2],
        mv_duz: Hedef,
        /// Bir önceki karenin rengi. Yakalayıcının dokusu her karede
        /// üstüne yazıldığı için saklanması şart.
        renk_onceki: Kopya,
        /// Üretilen kare.
        ara: Hedef,

        yuva: usize,
        /// En az iki kare görüldü mü? İlk karede üretilecek bir ara yok.
        pub hazir: bool,
        genislik: u32,
        yukseklik: u32,
    }

    impl Uretici {
        pub fn yeni(
            cihaz: &ID3D11Device,
            baglam: &ID3D11DeviceContext,
            genislik: u32,
            yukseklik: u32,
        ) -> Result<Self, Engel> {
            let blob = derle("VS", "vs_5_0")?;
            let koseler = unsafe {
                let mut g: Option<ID3D11VertexShader> = None;
                cihaz
                    .CreateVertexShader(bayt_kodu(&blob), None, Some(&mut g))
                    .map_err(sistem)?;
                g.ok_or(Engel::Desteklenmeyen)?
            };

            let izgara_g = genislik.div_ceil(hareket::BLOK).max(1);
            let izgara_y = yukseklik.div_ceil(hareket::BLOK).max(1);

            let luma_yuvasi = |c: &ID3D11Device| -> Result<[Hedef; hareket::SEVIYE], Engel> {
                Ok([
                    Hedef::yeni(c, genislik, yukseklik, DXGI_FORMAT_R16_FLOAT)?,
                    Hedef::yeni(c, genislik / 2, yukseklik / 2, DXGI_FORMAT_R16_FLOAT)?,
                    Hedef::yeni(c, genislik / 4, yukseklik / 4, DXGI_FORMAT_R16_FLOAT)?,
                ])
            };

            let sabit_tanim = D3D11_BUFFER_DESC {
                ByteWidth: std::mem::size_of::<Sabitler>() as u32,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let sabitler = unsafe {
                let mut b: Option<ID3D11Buffer> = None;
                cihaz
                    .CreateBuffer(&sabit_tanim, None, Some(&mut b))
                    .map_err(sistem)?;
                b.ok_or(Engel::Desteklenmeyen)?
            };

            Ok(Self {
                baglam: baglam.clone(),
                koseler,
                ps_luma: piksel(cihaz, "PS_Luma")?,
                ps_indirge: piksel(cihaz, "PS_Indirge")?,
                ps_hareket: piksel(cihaz, "PS_Hareket")?,
                ps_duzelt: piksel(cihaz, "PS_Duzelt")?,
                ps_ara: piksel(cihaz, "PS_Ara")?,
                sabitler,
                ornekleyici_nokta: ornekleyici(cihaz, D3D11_FILTER_MIN_MAG_MIP_POINT)?,
                ornekleyici_dogrusal: ornekleyici(cihaz, D3D11_FILTER_MIN_MAG_MIP_LINEAR)?,
                luma: [luma_yuvasi(cihaz)?, luma_yuvasi(cihaz)?],
                mv: [
                    Hedef::yeni(cihaz, izgara_g, izgara_y, DXGI_FORMAT_R16G16B16A16_FLOAT)?,
                    Hedef::yeni(cihaz, izgara_g, izgara_y, DXGI_FORMAT_R16G16B16A16_FLOAT)?,
                ],
                mv_duz: Hedef::yeni(cihaz, izgara_g, izgara_y, DXGI_FORMAT_R16G16B16A16_FLOAT)?,
                renk_onceki: Kopya::yeni(cihaz, genislik, yukseklik)?,
                ara: Hedef::yeni(cihaz, genislik, yukseklik, DXGI_FORMAT_B8G8R8A8_UNORM)?,
                yuva: 0,
                hazir: false,
                genislik,
                yukseklik,
            })
        }

        /// Ortak geçiş: hedefi bağla, gölgelendiriciyi çalıştır, çöz.
        ///
        /// Kaynak görünümleri her geçiş sonunda **çözülüyor**. Çözülmezse
        /// bir sonraki geçişte aynı doku hem okunacak hem yazılacak duruma
        /// düşüyor; sürücü bunu sessizce yok sayıp okumayı boşa çıkarıyor.
        #[allow(clippy::too_many_arguments)]
        fn gecis(
            &self,
            hedef: &Hedef,
            golgelendirici: &ID3D11PixelShader,
            kaynaklar: [Option<ID3D11ShaderResourceView>; 5],
            sabit: Sabitler,
        ) {
            unsafe {
                self.baglam.UpdateSubresource(
                    &self.sabitler,
                    0,
                    None,
                    &sabit as *const _ as *const _,
                    0,
                    0,
                );
                self.baglam
                    .OMSetRenderTargets(Some(&[Some(hedef.cizim.clone())]), None);
                let alan = D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: hedef.genislik as f32,
                    Height: hedef.yukseklik as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                self.baglam.RSSetViewports(Some(&[alan]));
                self.baglam
                    .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                self.baglam.VSSetShader(&self.koseler, None);
                self.baglam.PSSetShader(golgelendirici, None);
                self.baglam.PSSetShaderResources(0, Some(&kaynaklar));
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
                self.baglam
                    .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
                self.baglam.OMSetRenderTargets(None, None);
            }
        }

        /// Yeni yakalanan karenin parlaklık piramidini kurar.
        ///
        /// Yuva burada değişiyor: bundan sonra `luma[1 - yuva]` önceki
        /// karenin parlaklığı.
        pub fn luma_kur(&mut self, kaynak: &ID3D11ShaderResourceView) {
            self.yuva ^= 1;
            let yuva = self.yuva;

            // Tam çözünürlük.
            let tam = Sabitler {
                luma_boyut: [self.genislik as f32, self.yukseklik as f32],
                izgara_boyut: [0.0, 0.0],
                tam_boyut: [self.genislik as f32, self.yukseklik as f32],
                seviye: 0,
                zaman: 0.0,
            };
            self.gecis(
                &self.luma[yuva][0],
                &self.ps_luma,
                [None, Some(kaynak.clone()), None, None, None],
                tam,
            );

            // 1/2 ve 1/4: her biri bir üstünden.
            for seviye in 1..hareket::SEVIYE {
                let hedef = &self.luma[yuva][seviye];
                let sabit = Sabitler {
                    luma_boyut: [hedef.genislik as f32, hedef.yukseklik as f32],
                    izgara_boyut: [0.0, 0.0],
                    tam_boyut: [self.genislik as f32, self.yukseklik as f32],
                    seviye: seviye as i32,
                    zaman: 0.0,
                };
                self.gecis(
                    hedef,
                    &self.ps_indirge,
                    [
                        None,
                        None,
                        None,
                        Some(self.luma[yuva][seviye - 1].gorunum.clone()),
                        None,
                    ],
                    sabit,
                );
            }
        }

        /// Bu karenin rengini "önceki kare" olarak saklar.
        ///
        /// Yakalayıcının dokusu bir sonraki karede üstüne yazılıyor; warp
        /// için önceki karenin renginin ayrı bir kopyada durması şart.
        pub fn renk_sakla(&mut self, kaynak: &ID3D11Texture2D) {
            unsafe {
                self.baglam.CopyResource(&self.renk_onceki.doku, kaynak);
            }
            // İki kare birikti: bundan sonra ara kare üretilebilir.
            self.hazir = true;
        }

        /// İki gerçek kare arasındaki ara kareyi üretir.
        ///
        /// `zaman` 0 önceki, 1 sonraki. Şu an tek çarpan olduğu için hep
        /// 0.5 çağrılıyor; parametre olarak durması, 3x/4x çarpanların
        /// eklenmesi halinde bu fonksiyonun değişmemesi için
        /// (`hareket::Carpan`).
        ///
        /// Dönen görünüm bir sonraki `ara_kare` çağrısına kadar geçerli.
        pub fn ara_kare(
            &mut self,
            sonraki: &ID3D11ShaderResourceView,
            zaman: f32,
        ) -> Option<ID3D11ShaderResourceView> {
            if !self.hazir {
                return None;
            }
            let onceki_yuva = 1 - self.yuva;
            let izgara = [self.mv[0].genislik as f32, self.mv[0].yukseklik as f32];
            let tam = [self.genislik as f32, self.yukseklik as f32];

            // --- Hareket: kabadan inceye, hedefler arasında gidip gelerek.
            for (adim, seviye) in (0..hareket::SEVIYE).rev().enumerate() {
                let hedef = adim % 2;
                let kaynak_mv = if seviye == hareket::SEVIYE - 1 {
                    None
                } else {
                    Some(self.mv[1 - hedef].gorunum.clone())
                };
                let l = &self.luma[0][seviye];
                let sabit = Sabitler {
                    luma_boyut: [l.genislik as f32, l.yukseklik as f32],
                    izgara_boyut: izgara,
                    tam_boyut: tam,
                    seviye: seviye as i32,
                    zaman: 0.0,
                };
                self.gecis(
                    &self.mv[hedef],
                    &self.ps_hareket,
                    [
                        None,
                        None,
                        Some(self.luma[onceki_yuva][seviye].gorunum.clone()),
                        Some(self.luma[self.yuva][seviye].gorunum.clone()),
                        kaynak_mv,
                    ],
                    sabit,
                );
            }
            // Üç geçiş: son yazılan hedef 0 (0 → 1 → 0).
            let son = (hareket::SEVIYE - 1) % 2;

            // --- Ortanca düzeltme.
            let sabit = Sabitler {
                luma_boyut: izgara,
                izgara_boyut: izgara,
                tam_boyut: tam,
                seviye: 0,
                zaman: 0.0,
            };
            self.gecis(
                &self.mv_duz,
                &self.ps_duzelt,
                [None, None, None, None, Some(self.mv[son].gorunum.clone())],
                sabit,
            );

            // --- Warp + karışım.
            let sabit = Sabitler {
                luma_boyut: tam,
                izgara_boyut: izgara,
                tam_boyut: tam,
                seviye: 0,
                zaman,
            };
            self.gecis(
                &self.ara,
                &self.ps_ara,
                [
                    Some(self.renk_onceki.gorunum.clone()),
                    Some(sonraki.clone()),
                    None,
                    None,
                    Some(self.mv_duz.gorunum.clone()),
                ],
                sabit,
            );

            Some(self.ara.gorunum.clone())
        }
    }

    fn ornekleyici(
        cihaz: &ID3D11Device,
        filtre: D3D11_FILTER,
    ) -> Result<ID3D11SamplerState, Engel> {
        let tanim = D3D11_SAMPLER_DESC {
            Filter: filtre,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        unsafe {
            let mut s: Option<ID3D11SamplerState> = None;
            cihaz
                .CreateSamplerState(&tanim, Some(&mut s))
                .map_err(sistem)?;
            s.ok_or(Engel::Desteklenmeyen)
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Gölgelendirici sabitleri CPU referansıyla aynı mı?
    ///
    /// `hareket.rs` doğruluğu ölçüyor, `uretim.hlsl` işi yapıyor. İkisi
    /// ayrışırsa testler yeşil kalır ama ekran bozulur — bu test o
    /// sessiz ayrışmayı kapatıyor. `sunum.rs`'teki eşdeğerinin aynısı.
    #[test]
    fn sabitler_referansla_ayni() {
        let bekle = |ad: &str, deger: String| {
            let satir = format!("{ad} = {deger}");
            assert!(
                GOLGELENDIRICI.contains(&satir),
                "gölgelendiricide '{satir}' yok — CPU referansıyla ayrıştı"
            );
        };
        bekle("BLOK", format!("{};", hareket::BLOK));
        bekle("YARICAP_TAM", format!("{};", hareket::YARICAP[0]));
        bekle("YARICAP_YARI", format!("{};", hareket::YARICAP[1]));
        bekle("YARICAP_CEYREK", format!("{};", hareket::YARICAP[2]));
        bekle("ORTUSME_ESIGI", format!("{:.2};", hareket::ORTUSME_ESIGI));
    }

    /// Gölgelendirici gerçekten derleniyor mu?
    ///
    /// `D3DCompile` bir GPU cihazı istemiyor, dolayısıyla bu test ekransız
    /// bir makinede de koşuyor. Kazandırdığı şey büyük: bir HLSL hatası
    /// aksi halde ancak kullanıcı özelliği açtığında görünürdü.
    #[cfg(windows)]
    #[test]
    fn golgelendirici_derleniyor() {
        if let Err(e) = derleme_denemesi() {
            panic!("üretim gölgelendiricisi derlenmedi: {e}");
        }
    }

    /// Beş giriş noktasının hepsi gölgelendiricide var.
    ///
    /// Eksik bir giriş noktası ancak çalışma zamanında, kullanıcı özelliği
    /// açtığında patlardı.
    #[test]
    fn giris_noktalari_var() {
        for giris in ["VS", "PS_Luma", "PS_Indirge", "PS_Hareket", "PS_Duzelt", "PS_Ara"] {
            assert!(
                GOLGELENDIRICI.contains(giris),
                "gölgelendiricide {giris} giriş noktası yok"
            );
        }
    }
}
