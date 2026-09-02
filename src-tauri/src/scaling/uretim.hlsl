// Muifly — kare üretimi gölgelendiricileri (Faz 4, karar #35).
//
// Gerçek zamanlı yol burası. `scaling/hareket.rs` aynı algoritmanın CPU
// referansı ve **sözleşmesi**: buradaki sabitler orada da aynı, çünkü
// doğruluğu ölçen testler orada. İkisi ayrışırsa test yeşil kalır ama ekran
// bozulur; `uretim.rs`'teki `sabitler_referansla_ayni` testi bunu bağlıyor.
//
// Boru hattı (bir üretilmiş kare için):
//
//   PS_Luma      renk → parlaklık            (yakalanan kare başına bir kez)
//   PS_Indirge   2× kutu indirgeme           (piramidin 1/2 ve 1/4 katı)
//   PS_Hareket   blok eşleme, seviye seviye  (kaba → ince, üç geçiş)
//   PS_Duzelt    3×3 ortanca                 (aykırı vektörleri atar)
//   PS_Ara       çift yönlü warp + karışım   (üretilen kare)
//
// Hiçbiri oyun sürecine dokunmuyor: girdi yalnızca masaüstü çoğaltmasından
// gelen iki doku. Tasarım ilkesi 3 burada da geçerli.

// --- Sabitler: hareket.rs ile AYNI -------------------------------------
static const int BLOK = 16;
static const int YARICAP_TAM = 2;
static const int YARICAP_YARI = 3;
static const int YARICAP_CEYREK = 6;
static const float ORTUSME_ESIGI = 0.18;

// Rec.709 parlaklık. Yakalanan biçim BGRA ama gölgelendiriciye
// `DXGI_FORMAT_B8G8R8A8_UNORM` olarak bağlandığı için `.rgb` zaten doğru
// sırada; kanalları burada elle çevirmek renkleri ters gösterirdi.
static const float3 LUMA = float3(0.2126, 0.7152, 0.0722);

Texture2D<float4> renk_onceki   : register(t0);
Texture2D<float4> renk_sonraki  : register(t1);
Texture2D<float>  luma_onceki   : register(t2);
Texture2D<float>  luma_sonraki  : register(t3);
Texture2D<float4> mv_giris      : register(t4);

SamplerState nokta    : register(s0);
SamplerState dogrusal : register(s1);

cbuffer UretimSabitleri : register(b0)
{
    /// İşlenen seviyenin parlaklık dokusunun boyutu (piksel).
    float2 luma_boyut;
    /// Hareket ızgarasının boyutu (blok).
    float2 izgara_boyut;
    /// Tam çözünürlük (piksel) — warp bu uzayda çalışıyor.
    float2 tam_boyut;
    /// 0 tam, 1 yarı, 2 çeyrek.
    int seviye;
    /// Ara karenin zaman konumu: 0 önceki, 1 sonraki.
    float zaman;
};

struct VSCikti
{
    float4 konum : SV_Position;
    float2 uv : TEXCOORD0;
};

// Tam ekran üçgeni — `olcekleme.hlsl`dekiyle aynı gerekçe.
VSCikti VS(uint id : SV_VertexID)
{
    VSCikti o;
    o.uv = float2((id << 1) & 2, id & 2);
    o.konum = float4(o.uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    return o;
}

// --- Parlaklık ---------------------------------------------------------
// Hareket tahmini renk kullanmıyor: maliyeti üçe katlar, karşılığında
// yalnızca eşit parlaklıkta farklı renkli yüzeyleri ayırırdı.
float PS_Luma(VSCikti g) : SV_Target
{
    return dot(renk_sonraki.Sample(nokta, g.uv).rgb, LUMA);
}

// --- İndirgeme ---------------------------------------------------------
// Kutu ortalaması, tek örnek almaya tercih edildi: tek örnek alsaydık ince
// desenli bir yüzeyde (çim, tuğla) örtüşme oluşur ve kaba arama gerçek
// olmayan bir hareket bulurdu.
//
// `luma_boyut` burada ÇIKTININ boyutu.
float PS_Indirge(VSCikti g) : SV_Target
{
    int2 hedef = int2(g.uv * luma_boyut);
    int2 k = hedef * 2;
    float t = luma_sonraki.Load(int3(k, 0))
            + luma_sonraki.Load(int3(k + int2(1, 0), 0))
            + luma_sonraki.Load(int3(k + int2(0, 1), 0))
            + luma_sonraki.Load(int3(k + int2(1, 1), 0));
    return t * 0.25;
}

// --- Blok eşleme -------------------------------------------------------
// Çıktının her pikseli bir bloğun hareket vektörü. Izgara boyunda bir
// hedefe çizildiği için tam çözünürlükte 1920×1080 ekran 120×68 piksele
// düşüyor — arama maliyetinin katlanabilir olmasının tek sebebi bu.
//
// `mv_giris` bir üst (daha kaba) seviyenin sonucu; en kaba seviyede
// kullanılmıyor.
float2 kirp(float2 p)
{
    return clamp(p, float2(0.0, 0.0), luma_boyut - 1.0);
}

float4 PS_Hareket(VSCikti g) : SV_Target
{
    int2 hucre = int2(g.uv * izgara_boyut);
    int kat = 1 << seviye;
    int blok = max(BLOK / kat, 1);
    int2 taban = hucre * blok;

    int yaricap = YARICAP_TAM;
    if (seviye == 2)
        yaricap = YARICAP_CEYREK;
    else if (seviye == 1)
        yaricap = YARICAP_YARI;

    // Üst seviyenin vektörü bu seviyenin birimine taşınıyor.
    int2 merkez = int2(0, 0);
    if (seviye != 2)
        merkez = int2(round(mv_giris.Load(int3(hucre, 0)).xy)) * 2;

    int2 eniyi = merkez;
    float eniyi_fark = 1e30;

    [loop] for (int dy = -yaricap; dy <= yaricap; ++dy)
    {
        [loop] for (int dx = -yaricap; dx <= yaricap; ++dx)
        {
            int2 aday = merkez + int2(dx, dy);
            float toplam = 0.0;
            [loop] for (int by = 0; by < blok; ++by)
            {
                [loop] for (int bx = 0; bx < blok; ++bx)
                {
                    float2 p = float2(taban + int2(bx, by));
                    float a = luma_onceki.Load(int3(int2(kirp(p)), 0));
                    float b = luma_sonraki.Load(int3(int2(kirp(p + float2(aday))), 0));
                    toplam += abs(a - b);
                }
            }
            toplam /= float(blok * blok);
            // Eşitlikte merkeze yakın olan kazanıyor: düz bir yüzeyde bütün
            // konumlar aynı farkı verir ve "hareket yok" cevabı, rastgele
            // bir yöne kaymaktan doğrudur.
            if (toplam < eniyi_fark - 1e-6)
            {
                eniyi_fark = toplam;
                eniyi = aday;
            }
        }
    }

    // Güven kalibre edilmemiş bir sıralama ölçüsü, olasılık değil.
    float guven = saturate(1.0 - min(eniyi_fark / ORTUSME_ESIGI, 1.0));
    return float4(float(eniyi.x), float(eniyi.y), guven, 0.0);
}

// --- Ortanca düzeltme --------------------------------------------------
// Blok eşleme yerel bir karar veriyor ve düz yüzeylerde yanılıyor. Tek
// başına yanlış duran bir vektör, ara karede bir blok kadar bölgenin
// yanlış yere kaymasına yol açıyor. Ortanca seçiliyor, ortalama değil:
// ortalama bir yanlış vektörü komşularına bulaştırırdı.
float ortanca9(float v[9])
{
    // Yalnızca 5. eleman gerekli: seçmeli sıralamanın ilk beş adımı yeter.
    [unroll] for (int i = 0; i < 5; ++i)
    {
        int m = i;
        [unroll] for (int j = i + 1; j < 9; ++j)
        {
            if (v[j] < v[m])
                m = j;
        }
        float t = v[i];
        v[i] = v[m];
        v[m] = t;
    }
    return v[4];
}

float4 PS_Duzelt(VSCikti g) : SV_Target
{
    int2 hucre = int2(g.uv * izgara_boyut);
    float xs[9];
    float ys[9];
    float guven = 0.0;
    int n = 0;
    [unroll] for (int dy = -1; dy <= 1; ++dy)
    {
        [unroll] for (int dx = -1; dx <= 1; ++dx)
        {
            int2 k = clamp(hucre + int2(dx, dy), int2(0, 0), int2(izgara_boyut) - 1);
            float4 m = mv_giris.Load(int3(k, 0));
            xs[n] = m.x;
            ys[n] = m.y;
            guven += m.z;
            ++n;
        }
    }
    return float4(ortanca9(xs), ortanca9(ys), guven / 9.0, 0.0);
}

// --- Ara kare ----------------------------------------------------------
// Çift yönlü warp: önceki kare ileri, sonraki kare geri taşınıyor. Tek
// yönlü warp daha ucuz olurdu ama nesnenin ARKASINDA açılan boşluğu
// doldurabilecek bir kaynağı olmazdı; o boşluk her karede esneyen bir iz
// bırakır.
//
// Izgara doğrusal örnekleniyor: doğrudan okunsaydı blok sınırlarında
// vektör bir anda değişir ve 16 pikselde bir görünür bir basamak oluşurdu.
float2 izgara_ornekle(float2 piksel)
{
    float2 gc = piksel / float(BLOK);
    return mv_giris.SampleLevel(dogrusal, gc / izgara_boyut, 0).xy;
}

float4 PS_Ara(VSCikti g) : SV_Target
{
    float2 p = g.uv * tam_boyut;
    float2 v = izgara_ornekle(p);

    float2 uv_a = (p - v * zaman) / tam_boyut;
    float2 uv_b = (p + v * (1.0 - zaman)) / tam_boyut;
    float4 a = renk_onceki.Sample(dogrusal, uv_a);
    float4 b = renk_sonraki.Sample(dogrusal, uv_b);

    // İki yön birbirini tutmuyorsa orada bir örtüşme var: bir nesnenin
    // arkasından çıkan piksel önceki karede YOK. Ortalamasını almak orada
    // hayalet bir görüntü üretir; bunun yerine zaman olarak yakın olan
    // kare seçiliyor. Sonuç "yanlış" değil, "üretilmemiş".
    float ayrisma = abs(dot(a.rgb, LUMA) - dot(b.rgb, LUMA));
    if (ayrisma > ORTUSME_ESIGI)
        return (zaman < 0.5) ? a : b;

    return lerp(a, b, zaman);
}
