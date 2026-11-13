# Builds assets/streamboard.ico from assets/streamboard.png. Windows only.
Add-Type -AssemblyName System.Drawing

$srcPath = Join-Path $PSScriptRoot "streamboard.png"
$src = [System.Drawing.Image]::FromFile($srcPath)
$sizes = @(16, 24, 32, 48, 64, 256)
$images = New-Object System.Collections.Generic.List[byte[]]

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $graphics = [System.Drawing.Graphics]::FromImage($bmp)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.DrawImage($src, 0, 0, $size, $size)
    $graphics.Dispose()

    $stream = New-Object System.IO.MemoryStream
    $bmp.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $images.Add($stream.ToArray())

    if ($size -eq 32) {
        $rect = New-Object System.Drawing.Rectangle 0, 0, 32, 32
        $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $stride = [Math]::Abs($data.Stride)
        $raw = New-Object byte[] ($stride * 32)
        [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $raw, 0, $raw.Length)
        $bmp.UnlockBits($data)
        $rgba = New-Object byte[] (32 * 32 * 4)
        for ($y = 0; $y -lt 32; $y++) {
            for ($x = 0; $x -lt 32; $x++) {
                $srcIndex = $y * $stride + $x * 4
                $dstIndex = ($y * 32 + $x) * 4
                $rgba[$dstIndex] = $raw[$srcIndex + 2]
                $rgba[$dstIndex + 1] = $raw[$srcIndex + 1]
                $rgba[$dstIndex + 2] = $raw[$srcIndex]
                $rgba[$dstIndex + 3] = $raw[$srcIndex + 3]
            }
        }
        [System.IO.File]::WriteAllBytes((Join-Path $PSScriptRoot "streamboard-32.rgba"), $rgba)
    }
    $bmp.Dispose()
    $stream.Dispose()
}
$src.Dispose()

$count = $images.Count
$offset = 6 + (16 * $count)
$file = New-Object System.Collections.Generic.List[byte]
$file.AddRange([byte[]](0, 0, 1, 0, [byte]$count, 0))
for ($index = 0; $index -lt $count; $index++) {
    $image = $images[$index]
    $length = $image.Length
    $dimension = if ($sizes[$index] -ge 256) { [byte]0 } else { [byte]$sizes[$index] }
    $entry = New-Object byte[] 16
    $entry[0] = $dimension
    $entry[1] = $dimension
    $entry[4] = 1
    $entry[6] = 32
    $entry[8] = [byte]($length -band 255)
    $entry[9] = [byte](($length -shr 8) -band 255)
    $entry[10] = [byte](($length -shr 16) -band 255)
    $entry[11] = [byte](($length -shr 24) -band 255)
    $entry[12] = [byte]($offset -band 255)
    $entry[13] = [byte](($offset -shr 8) -band 255)
    $entry[14] = [byte](($offset -shr 16) -band 255)
    $entry[15] = [byte](($offset -shr 24) -band 255)
    $file.AddRange($entry)
    $offset += $length
}
foreach ($image in $images) {
    $file.AddRange([byte[]]$image)
}
[System.IO.File]::WriteAllBytes((Join-Path $PSScriptRoot "streamboard.ico"), $file.ToArray())
Write-Host "wrote streamboard.ico ($($file.Count) bytes)"
