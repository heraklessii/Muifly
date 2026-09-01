# Faz 5 fizibilitesi — OCR test külliyatı üreteci.
#
# ATILACAK DENEME. Ürün kodu değil; ROADMAP'in Faz 5 için şart koştuğu
# "Windows.Media.Ocr gerçek oyun yazı tiplerini okuyor mu" sorusunu
# ölçülebilir hale getirmek için var.
#
# SINIRI: Bu görüntüler sentetik. Gerçek bir oyun ekran görüntüsü değiller.
# Oyun metninin bilinen zorluklarını (kontur, gölge, düşük kontrast, küçük
# punto, süslü font, kalabalık zemin) taklit ediyorlar; taklit edemedikleri
# şey oyunun kendi ölçekleme/keskinleştirme boru hattı ve sıkıştırma
# gürültüsü. Sonuçlar bu yüzden "en iyi durum" tarafında okunmalı.
#
# Kullanım:  powershell -ExecutionPolicy Bypass -File uret-ornekler.ps1

Add-Type -AssemblyName System.Drawing

$hedef = Join-Path $PSScriptRoot 'ornekler'
if (-not (Test-Path $hedef)) { New-Item -ItemType Directory -Path $hedef | Out-Null }

$rnd = New-Object System.Random 20260901

function Yeni-Zemin {
    param([int]$g, [int]$y, [string]$tur)

    $bmp = New-Object System.Drawing.Bitmap($g, $y)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.SmoothingMode = 'AntiAlias'

    switch ($tur) {
        'duz' {
            $gfx.Clear([System.Drawing.Color]::FromArgb(26, 26, 28))
        }
        'kalabalik' {
            # Fotoğrafik bir sahnenin yerine geçen renk gürültüsü: OCR'ın
            # metni zeminden ayırmak zorunda kaldığı durum.
            $gfx.Clear([System.Drawing.Color]::FromArgb(60, 70, 55))
            for ($i = 0; $i -lt 220; $i++) {
                $c = [System.Drawing.Color]::FromArgb(
                    $rnd.Next(40, 190), $rnd.Next(40, 190), $rnd.Next(30, 150))
                $f = New-Object System.Drawing.SolidBrush($c)
                $gfx.FillEllipse($f, $rnd.Next(-40, $g), $rnd.Next(-40, $y),
                    $rnd.Next(20, 160), $rnd.Next(20, 160))
                $f.Dispose()
            }
        }
        'parsomen' {
            $gfx.Clear([System.Drawing.Color]::FromArgb(222, 202, 162))
            for ($i = 0; $i -lt 900; $i++) {
                $t = $rnd.Next(-18, 18)
                $c = [System.Drawing.Color]::FromArgb(60, 150 + $t, 130 + $t, 95 + $t)
                $f = New-Object System.Drawing.SolidBrush($c)
                $gfx.FillEllipse($f, $rnd.Next(0, $g), $rnd.Next(0, $y), $rnd.Next(2, 9), $rnd.Next(2, 9))
                $f.Dispose()
            }
        }
        'orta' {
            # Yatay degrade: metnin bir ucu koyu bir ucu açık zemine denk gelir.
            $r = New-Object System.Drawing.Rectangle(0, 0, $g, $y)
            $b = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
                $r,
                [System.Drawing.Color]::FromArgb(120, 110, 100),
                [System.Drawing.Color]::FromArgb(35, 40, 60),
                0.0)
            $gfx.FillRectangle($b, $r)
            $b.Dispose()
        }
    }
    return @{ Bitmap = $bmp; Graphics = $gfx }
}

function Yaz-Konturlu {
    param($gfx, [string]$metin, $font, $ic, $kontur, [float]$kalinlik, [int]$x, [int]$y)

    $yol = New-Object System.Drawing.Drawing2D.GraphicsPath
    $yol.AddString($metin, $font.FontFamily, [int]$font.Style, $font.Size,
        (New-Object System.Drawing.PointF($x, $y)),
        [System.Drawing.StringFormat]::GenericTypographic)

    if ($kalinlik -gt 0) {
        $kalem = New-Object System.Drawing.Pen($kontur, $kalinlik)
        $kalem.LineJoin = 'Round'
        $gfx.DrawPath($kalem, $yol)
        $kalem.Dispose()
    }
    $firca = New-Object System.Drawing.SolidBrush($ic)
    $gfx.FillPath($firca, $yol)
    $firca.Dispose()
    $yol.Dispose()
}

function Yaz-Duz {
    param($gfx, [string]$metin, $font, $renk, [int]$x, [int]$y, $golge)

    if ($golge) {
        $gf = New-Object System.Drawing.SolidBrush($golge)
        $gfx.DrawString($metin, $font, $gf, ($x + 2), ($y + 2))
        $gf.Dispose()
    }
    $f = New-Object System.Drawing.SolidBrush($renk)
    $gfx.DrawString($metin, $font, $f, $x, $y)
    $f.Dispose()
}

function Yaz-Aralikli {
    param($gfx, [string]$metin, $font, $renk, [int]$x, [int]$y, [float]$aralik)

    $f = New-Object System.Drawing.SolidBrush($renk)
    $imlec = [float]$x
    foreach ($ch in $metin.ToCharArray()) {
        $s = [string]$ch
        $gfx.DrawString($s, $font, $f, $imlec, $y)
        $w = $gfx.MeasureString($s, $font, 0, [System.Drawing.StringFormat]::GenericTypographic).Width
        $imlec += $w + $aralik
    }
    $f.Dispose()
}

function Kaydet {
    param($z, [string]$ad, [string]$dogru)
    $z.Graphics.Dispose()
    $png = Join-Path $hedef "$ad.png"
    $z.Bitmap.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
    $z.Bitmap.Dispose()
    # BOM YOK: BOM'lu yazildiginda gorunmez karakter dogru metnin ilk kelimesine
    # yapisip olcumu bozuyordu.
    $bomsuz = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText((Join-Path $hedef "$ad.txt"), $dogru, $bomsuz)
    "  uretildi: $ad.png"
}

$beyaz = [System.Drawing.Color]::White
$siyah = [System.Drawing.Color]::Black

"Kulliyat uretiliyor -> $hedef"

# --- 1. Kolay taban çizgisi: düz altyazı, yüksek kontrast --------------------
$m = 'You should speak with the innkeeper before nightfall.'
$z = Yeni-Zemin 1000 120 'duz'
$fnt = New-Object System.Drawing.Font('Segoe UI', 26, [System.Drawing.FontStyle]::Regular)
Yaz-Duz $z.Graphics $m $fnt $beyaz 30 40 $null
$fnt.Dispose()
Kaydet $z '01-altyazi-duz' $m

# --- 2. En yaygın oyun altyazısı: konturlu, kalabalık zemin ------------------
$m = 'The bridge collapsed. We need another way across the river.'
$z = Yeni-Zemin 1100 130 'kalabalik'
$fnt = New-Object System.Drawing.Font('Trebuchet MS', 28, [System.Drawing.FontStyle]::Bold)
Yaz-Konturlu $z.Graphics $m $fnt $beyaz $siyah 4.0 30 40
$fnt.Dispose()
Kaydet $z '02-altyazi-konturlu' $m

# --- 3. Gölgeli metin, orta tonlu degrade zemin ------------------------------
$m = 'Press F to pick up the ancient key.'
$z = Yeni-Zemin 800 120 'orta'
$fnt = New-Object System.Drawing.Font('Segoe UI', 24, [System.Drawing.FontStyle]::Bold)
Yaz-Duz $z.Graphics $m $fnt $beyaz 30 40 ([System.Drawing.Color]::FromArgb(180, 0, 0, 0))
$fnt.Dispose()
Kaydet $z '03-golgeli-degrade' $m

# --- 4. RPG süslü serif, parşömen zemin --------------------------------------
$m = 'My father left this blade to me, and now I leave it to you.'
$z = Yeni-Zemin 1050 130 'parsomen'
$fnt = New-Object System.Drawing.Font('Georgia', 24, [System.Drawing.FontStyle]::Italic)
Yaz-Duz $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(58, 38, 20)) 30 45 $null
$fnt.Dispose()
Kaydet $z '04-rpg-serif-parsomen' $m

# --- 5. Küçük punto tooltip, yarı saydam panel üstünde ----------------------
$m = 'Iron Longsword - Damage 42, Weight 6.5, Value 120 gold'
$z = Yeni-Zemin 700 90 'kalabalik'
$panel = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(190, 12, 14, 20))
$z.Graphics.FillRectangle($panel, 15, 20, 670, 50)
$panel.Dispose()
$fnt = New-Object System.Drawing.Font('Segoe UI', 11, [System.Drawing.FontStyle]::Regular)
Yaz-Duz $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(210, 210, 215)) 28 35 $null
$fnt.Dispose()
Kaydet $z '05-tooltip-kucuk' $m

# --- 6. Menü: büyük harf + harf aralığı --------------------------------------
$m = 'LOAD GAME    SETTINGS    QUIT TO DESKTOP'
$z = Yeni-Zemin 1100 110 'duz'
$fnt = New-Object System.Drawing.Font('Bahnschrift', 24, [System.Drawing.FontStyle]::Regular)
Yaz-Aralikli $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(235, 225, 200)) 30 40 3.0
$fnt.Dispose()
Kaydet $z '06-menu-aralikli' $m

# --- 7. Düşük kontrast: gri üstüne gri --------------------------------------
$m = 'Autosaving. Do not turn off your console.'
$z = Yeni-Zemin 800 110 'duz'
$gri = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(106, 106, 106))
$z.Graphics.FillRectangle($gri, 0, 0, 800, 110)
$gri.Dispose()
$fnt = New-Object System.Drawing.Font('Corbel', 22, [System.Drawing.FontStyle]::Regular)
Yaz-Duz $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(154, 154, 154)) 30 40 $null
$fnt.Dispose()
Kaydet $z '07-dusuk-kontrast' $m

# --- 8. Stilize başlık: Impact, sarı, kalın kontur, kalabalık zemin ----------
$m = 'MISSION FAILED - RETURN TO CHECKPOINT'
$z = Yeni-Zemin 1100 140 'kalabalik'
$fnt = New-Object System.Drawing.Font('Impact', 32, [System.Drawing.FontStyle]::Regular)
Yaz-Konturlu $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(240, 200, 60)) $siyah 5.0 30 45
$fnt.Dispose()
Kaydet $z '08-stilize-impact' $m

# --- 9. Çok satırlı diyalog kutusu -------------------------------------------
$m = "We have been walking for three days without water.`nIf the well is dry, we turn back at dawn."
$z = Yeni-Zemin 900 180 'kalabalik'
$panel = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(205, 8, 10, 16))
$z.Graphics.FillRectangle($panel, 20, 20, 860, 140)
$panel.Dispose()
$fnt = New-Object System.Drawing.Font('Constantia', 20, [System.Drawing.FontStyle]::Regular)
Yaz-Duz $z.Graphics $m $fnt ([System.Drawing.Color]::FromArgb(230, 228, 220)) 40 45 $null
$fnt.Dispose()
Kaydet $z '09-diyalog-cok-satir' $m

# --- 10. Tam kare 1920x1080: yakalama alani buyudugunde maliyet ne oluyor? --
# Karar #22 kullanicinin bir ALAN secmesini ongoruyor, yani gercek kullanimda
# tum kare taranmayacak. Yine de maliyet egrisinin ust ucunu bilmek gerekiyor.
$m = 'I have never seen the northern gate left open like this.'
$z = Yeni-Zemin 1920 1080 'kalabalik'
$fnt = New-Object System.Drawing.Font('Trebuchet MS', 30, [System.Drawing.FontStyle]::Bold)
Yaz-Konturlu $z.Graphics $m $fnt $beyaz $siyah 4.0 380 940
$fnt.Dispose()
Kaydet $z '10-tam-kare-1080p' $m

"Bitti."
