// Muifly — ölçekleme gölgelendiricileri.
//
// Gerçek zamanlı yol burası. Yakalanan doku hiç CPU'ya inmeden, aynı D3D11
// cihazı üzerinde ölçeklenip sunuluyor (`decisions.md` #32).
//
// `scaling/algoritma.rs` aynı matematiğin CPU referansı. İkisi arasındaki
// bilinen ve bilinçli fark: xBR'de referans önce sabit iki kat üretip
// kalanını Lanczos'a bırakıyor, buradaki kural doğrudan hedef çözünürlükte
// değerlendiriliyor. Tam iki katta ikisi aynı sonucu veriyor.
//
// Sabitler iki tarafta aynı olmak zorunda; `sunum.rs`'teki
// `golgelendirici_sabitleri_referansla_ayni` testi bunu bağlıyor.

static const float LANCZOS_A = 3.0;
static const float XBR_GUCLU_KAT = 2.0;
static const float XBR_ZAYIF_KARISIM = 0.5;
static const float PI = 3.14159265;

Texture2D kaynak : register(t0);
SamplerState nokta : register(s0);
SamplerState dogrusal : register(s1);

// Yakalanan doku masaüstünün TAMAMI; ölçeklenen ise onun içindeki bir
// dikdörtgen (oyun penceresi). Kırpma burada yapılıyor, ayrı bir kopyayla
// değil: fazladan bir doku kopyası her karede bir GPU turu daha demekti.
cbuffer Sabitler : register(b0)
{
    /// Kırpılan alanın dokudaki sol üst köşesi (piksel).
    float2 kaynak_ofset;
    /// Kırpılan alanın boyutu (piksel). Ölçeklenen şey bu.
    float2 kaynak_boyut;
    /// Yakalanan dokunun tamamı (piksel) — örnekleme koordinatı için.
    float2 doku_boyut;
    float2 hedef_boyut;
};

/// Kırpılmış alandaki bir konumu doku koordinatına çevirir.
float2 dokuya(float2 piksel)
{
    return (kaynak_ofset + piksel) / doku_boyut;
}

struct VSCikti
{
    float4 konum : SV_Position;
    float2 uv : TEXCOORD0;
};

// Tam ekran üçgeni: köşe tamponu yok, üç köşe kimlikten üretiliyor.
// Bir dikdörtgen için iki üçgen çizmek, köşegen boyunca pikselleri iki kez
// hesaplatırdı.
VSCikti VS(uint id : SV_VertexID)
{
    VSCikti o;
    o.uv = float2((id << 1) & 2, id & 2);
    o.konum = float4(o.uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    return o;
}

// --- Tam sayı katı / en yakın komşu ------------------------------------
// Yeni renk üretmeyen tek yol. Ekrana tam oturmayan kısım için görüntü
// alanı (viewport) daraltılıyor, kenarda kalan yer siyah temizleniyor;
// bu yüzden burada dolgu hesabı yok.
float4 PS_TamSayi(VSCikti g) : SV_Target
{
    return kaynak.Sample(nokta, dokuya(g.uv * kaynak_boyut));
}

// --- Bilinear ----------------------------------------------------------
float4 PS_Bilinear(VSCikti g) : SV_Target
{
    return kaynak.Sample(dogrusal, dokuya(g.uv * kaynak_boyut));
}

// --- Lanczos -----------------------------------------------------------
float lanczos_agirlik(float x)
{
    x = abs(x);
    if (x < 1e-5)
        return 1.0;
    if (x >= LANCZOS_A)
        return 0.0;
    float px = PI * x;
    return (LANCZOS_A * sin(px) * sin(px / LANCZOS_A)) / (px * px);
}

// 6x6 örnek: a=3 yarıçapının büyütmedeki tam desteği.
//
// Ağırlıklar toplamı burada da normalleştiriliyor. Normalleştirilmezse
// görüntünün parlaklığı ölçek oranına göre dalgalanır — sabit bir kayma
// değil, ekranda hareketli bir desen olarak görünürdü.
float4 PS_Lanczos(VSCikti g) : SV_Target
{
    float2 merkez = g.uv * kaynak_boyut - 0.5;
    float2 taban = floor(merkez);
    float2 f = merkez - taban;

    float3 toplam = 0.0;
    float agirlik_toplam = 0.0;
    [unroll]
    for (int j = -2; j <= 3; j++)
    {
        [unroll]
        for (int i = -2; i <= 3; i++)
        {
            float w = lanczos_agirlik(i - f.x) * lanczos_agirlik(j - f.y);
            float2 koord = dokuya(taban + float2(i, j) + 0.5);
            toplam += kaynak.SampleLevel(nokta, koord, 0).rgb * w;
            agirlik_toplam += w;
        }
    }
    return float4(toplam / max(agirlik_toplam, 1e-5), 1.0);
}

// --- xBR ---------------------------------------------------------------
// Ağırlıklı YUV uzaklığı: kenar tespitinin gözün ayırdığı farkı izlemesi
// için. Parlaklık baskın ağırlığı alıyor.
float uzaklik(float3 a, float3 b)
{
    float3 d = a - b;
    float dy = 0.299 * d.r + 0.587 * d.g + 0.114 * d.b;
    float du = -0.169 * d.r - 0.331 * d.g + 0.5 * d.b;
    float dv = 0.5 * d.r - 0.419 * d.g - 0.081 * d.b;
    return 48.0 * abs(dy) + 7.0 * abs(du) + 6.0 * abs(dv);
}

// Kural tek bir köşe için yazılı; dört köşe, komşuluğun döndürülmesiyle.
// CPU referansındaki `Komsuluk::p` ile aynı dönüş sırası.
float2 dondur(float2 d, int r)
{
    if (r == 1)
        return float2(-d.y, d.x);
    if (r == 2)
        return -d;
    if (r == 3)
        return float2(d.y, -d.x);
    return d;
}

float4 PS_Xbr(VSCikti g) : SV_Target
{
    float2 merkez = g.uv * kaynak_boyut;
    float2 taban = floor(merkez);
    float2 alt = merkez - taban;

    // Hangi köşedeyiz: sağ/aşağı yönü dönüş indeksini belirliyor.
    // 0 = sağ alt, 1 = sol alt, 2 = sol üst, 3 = sağ üst.
    int r;
    if (alt.x >= 0.5 && alt.y >= 0.5)
        r = 0;
    else if (alt.x < 0.5 && alt.y >= 0.5)
        r = 1;
    else if (alt.x < 0.5 && alt.y < 0.5)
        r = 2;
    else
        r = 3;

#define ORNEK(dx, dy) kaynak.SampleLevel(nokta, dokuya(taban + 0.5 + dondur(float2(dx, dy), r)), 0).rgb

    float3 e = ORNEK(0, 0);
    float3 f = ORNEK(1, 0);
    float3 h = ORNEK(0, 1);
    float3 i = ORNEK(1, 1);
    float3 b = ORNEK(0, -1);
    float3 c = ORNEK(1, -1);
    float3 d = ORNEK(-1, 0);
    float3 gg = ORNEK(-1, 1);
    float3 f4 = ORNEK(2, 0);
    float3 i4 = ORNEK(2, 1);
    float3 h5 = ORNEK(0, 2);
    float3 i5 = ORNEK(1, 2);

#undef ORNEK

    // Kenarın E-I köşegeninde olduğu varsayımının maliyeti.
    float e_maliyet = uzaklik(e, c) + uzaklik(e, gg) + uzaklik(i, h5) + uzaklik(i, f4)
                      + 4.0 * uzaklik(h, f);
    // Kenarın F-H köşegeninde olduğu varsayımının maliyeti.
    float i_maliyet = uzaklik(h, d) + uzaklik(h, i5) + uzaklik(f, i4) + uzaklik(f, b)
                      + 4.0 * uzaklik(e, i);

    if (e_maliyet >= i_maliyet)
        return float4(e, 1.0); // Kenar yok: düz alan bozulmuyor.

    float3 px = (uzaklik(e, f) <= uzaklik(e, h)) ? f : h;
    if (e_maliyet * XBR_GUCLU_KAT < i_maliyet)
        return float4(px, 1.0); // Güçlü kenar: köşe tamamen değişiyor.
    return float4(lerp(e, px, XBR_ZAYIF_KARISIM), 1.0);
}
