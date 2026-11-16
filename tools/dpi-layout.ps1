# Pixel-level assertions against an actual, unmodified GPUI window capture.
function Assert-DpiScreenshot([string]$Path, [bool]$Expected, [int]$Percent = -1, [switch]$ReturnLayout) {
    $bitmap = [Drawing.Bitmap]::new($Path)
    try {
        $scale = $bitmap.Width / 304.0
        $background = $bitmap.GetPixel([int](8*$scale), [int](40*$scale))
        $backgroundHex = '{0:x2}{1:x2}{2:x2}' -f $background.R,$background.G,$background.B
        $border = switch ($backgroundHex) {
            '191c21' { [Drawing.Color]::FromArgb(57,62,70) }
            'fafbfd' { [Drawing.Color]::FromArgb(213,219,227) }
            'c3bddb' { [Drawing.Color]::FromArgb(155,148,196) }
            default { throw "Unknown card palette: $backgroundHex" }
        }
        $left = [int](74*$scale); $right = [int](246*$scale)
        $top = [int](48*$scale); $bottom = [int](107*$scale)
        $edges = [Collections.Generic.List[int]]::new()
        for ($y=$top; $y -lt $bottom; $y++) {
            $run=0; $longest=0
            for ($x=$left; $x -lt $right; $x++) {
                if ($bitmap.GetPixel($x,$y).ToArgb() -eq $border.ToArgb()) { $run++; $longest=[math]::Max($run,$longest) } else { $run=0 }
            }
            if ($longest -ge 16*$scale) { $edges.Add($y) }
        }
        if (-not $Expected) {
            if ($edges.Count -ne 0) { throw 'A stale DPI badge remained visible without a valid reading.' }
            return
        }
        if ($edges.Count -lt 2) { throw 'The outlined DPI badge is missing.' }
        $badgeTop=$edges[0]; $badgeBottom=$edges[$edges.Count-1]
        $badgeLeft=$right; $badgeRight=$left
        for ($y=$badgeTop; $y -le $badgeBottom; $y++) {
            for ($x=$left; $x -lt $right; $x++) {
                if ($bitmap.GetPixel($x,$y).ToArgb() -eq $border.ToArgb()) {
                    $badgeLeft=[math]::Min($badgeLeft,$x); $badgeRight=[math]::Max($badgeRight,$x)
                }
            }
        }
        function Test-Ink([int]$X, [int]$Y) {
            $p=$bitmap.GetPixel($X,$Y)
            return [math]::Max([math]::Abs($p.R-$background.R),[math]::Max([math]::Abs($p.G-$background.G),[math]::Abs($p.B-$background.B))) -gt 70
        }
        function Get-InkBounds([int]$X0,[int]$Y0,[int]$X1,[int]$Y1) {
            $xMin=$X1; $xMax=-1; $yMin=$Y1; $yMax=-1
            for ($y=$Y0; $y -le $Y1; $y++) {
                for ($x=$X0; $x -le $X1; $x++) {
                    if (Test-Ink $x $y) {
                        $xMin=[math]::Min($xMin,$x); $xMax=[math]::Max($xMax,$x)
                        $yMin=[math]::Min($yMin,$y); $yMax=[math]::Max($yMax,$y)
                    }
                }
            }
            if ($xMax -lt 0) { throw 'Expected text/glyph pixels were not rendered.' }
            return [pscustomobject]@{ Left=$xMin; Right=$xMax; Top=$yMin; Bottom=$yMax }
        }
        $insideLeft=$badgeLeft+[int](2*$scale); $insideRight=$badgeRight-[int](2*$scale)
        $bands=[Collections.Generic.List[object]]::new()
        $start=-1
        for ($y=$badgeTop+1; $y -le $badgeBottom; $y++) {
            $hasInk=$false
            for ($x=$insideLeft; $x -le $insideRight; $x++) { if (Test-Ink $x $y) { $hasInk=$true; break } }
            if ($hasInk -and $start -lt 0) { $start=$y }
            if (-not $hasInk -and $start -ge 0) {
                $bands.Add((Get-InkBounds $insideLeft $start $insideRight ($y-1))); $start=-1
            }
        }
        if ($bands.Count -ne 2) { throw "DPI badge must contain two distinct text lines, got $($bands.Count)." }
        $label=$bands[0]; $value=$bands[1]
        if ($label.Bottom-$label.Top -le $value.Bottom-$value.Top) { throw 'DPI caption must be larger than its numeric value.' }
        if ($value.Left -lt $label.Left-$scale -or $value.Right -gt $label.Right+$scale) {
            throw 'Numeric DPI extends outside the visible DPI caption.'
        }
        if ([math]::Abs($value.Right-$label.Right) -gt 2*$scale) { throw 'Numeric DPI is not right-aligned to its caption.' }
        $paddingTop=$label.Top-$badgeTop-1
        $paddingLeft=$label.Left-$badgeLeft-1
        $paddingRight=$badgeRight-$label.Right-1
        if ([math]::Max([math]::Abs($paddingTop-$paddingLeft),[math]::Abs($paddingTop-$paddingRight)) -gt 2*$scale) {
            throw 'DPI caption has unbalanced optical padding.'
        }
        $charge=Get-InkBounds ([int](16*$scale)) $top ([int]([math]::Min($badgeLeft-6*$scale,130*$scale))) ([int](104*$scale))
        $power=Get-InkBounds ([int]($badgeRight+6*$scale)) $top ([int](288*$scale)) ([int](104*$scale))
        if ([math]::Abs($badgeTop-$charge.Top) -gt 2*$scale) { throw 'DPI badge is not aligned with the visible top of the charge.' }
        # Balance against the fixed power column, not right-aligned label ink.
        $powerRegionLeft=[int](184*$scale)
        $leftGap=$badgeLeft-$charge.Right-1; $rightGap=$powerRegionLeft-$badgeRight-1
        if ($power.Left -lt $powerRegionLeft-$scale -or [math]::Abs($power.Right-(288*$scale-1)) -gt 3*$scale) {
            throw 'Power status is not right-aligned inside its reserved column.'
        }
        if ([math]::Min($leftGap,$rightGap) -lt 6*$scale) {
            throw "Unbalanced/tight metric spacing: $leftGap / $rightGap pixels."
        }
        $rateLeft=$bitmap.Width; $rateRight=-1
        for($y=[int](18*$scale);$y -le [int](38*$scale);$y++) {
            for($x=[int](112*$scale);$x -le [int](210*$scale);$x++) {
                if($bitmap.GetPixel($x,$y).ToArgb() -eq $border.ToArgb()) {
                    $rateLeft=[math]::Min($rateLeft,$x); $rateRight=[math]::Max($rateRight,$x)
                }
            }
        }
        if($rateRight -ge 0 -and [math]::Abs(($badgeLeft+$badgeRight-$rateLeft-$rateRight)/2.0) -gt $scale) {
            throw 'DPI and polling-rate badges do not share a vertical center axis.'
        }
        Write-Host "DPI layout passed: caption/value widths $($label.Right-$label.Left+1)/$($value.Right-$value.Left+1), gaps $leftGap/$rightGap, padding top/left/right $($label.Top-$badgeTop-1)/$($label.Left-$badgeLeft-1)/$($badgeRight-$label.Right-1); $(Split-Path -Leaf $Path)."
        if ($ReturnLayout) { return [pscustomobject]@{ Left=$badgeLeft; Top=$badgeTop; Width=$badgeRight-$badgeLeft+1; Height=$badgeBottom-$badgeTop+1 } }
    } finally { $bitmap.Dispose() }
}

function Assert-DpiStablePosition([string[]]$Paths) {
    $reference=$null
    foreach ($path in $Paths) {
        $layout=Assert-DpiScreenshot $path $true -ReturnLayout
        if ($null -eq $reference) { $reference=$layout; continue }
        foreach ($key in @('Left','Top','Width','Height')) {
            if ($layout.$key -ne $reference.$key) { throw "DPI badge $key changed between values/statuses: $path" }
        }
    }
    Write-Host "Fixed DPI bounds passed for $($Paths.Count) frames: $($reference.Left),$($reference.Top),$($reference.Width),$($reference.Height)."
}
