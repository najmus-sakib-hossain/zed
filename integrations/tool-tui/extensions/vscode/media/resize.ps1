Add-Type -AssemblyName System.Drawing

function Resize-Image {
    param(
        [string]$InputPath,
        [string]$OutputPath,
        [int]$Width,
        [int]$Height
    )
    
    $fullInputPath = Resolve-Path $InputPath
    $img = [System.Drawing.Image]::FromFile($fullInputPath)
    $newImg = New-Object System.Drawing.Bitmap($Width, $Height)
    $graphics = [System.Drawing.Graphics]::FromImage($newImg)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.DrawImage($img, 0, 0, $Width, $Height)
    
    $newImg.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    
    $graphics.Dispose()
    $newImg.Dispose()
    $img.Dispose()
    
    Write-Host "Resized: $InputPath -> $OutputPath"
}

# Resize dark icon
Resize-Image -InputPath "extension/media/file-extension-dark.png" -OutputPath "extension/media/dx-file-extension-dark.png" -Width 16 -Height 16

# Resize light icon
Resize-Image -InputPath "extension/media/file-extension-light.png" -OutputPath "extension/media/dx-file-extension-light.png" -Width 16 -Height 16

Write-Host "Done resizing icons to 16x16 pixels!"
