<#
.SYNOPSIS
    Bulk Import Tool voor Poepende Bram bezoeken
.DESCRIPTION
    Importeert in Ã©Ã©n klap oudere poepbezoeken vanuit een CSV, tekstbestand, Excel kopie of interactieve invoer.
.EXAMPLE
    .\import-visits.ps1 -File "bezoeken.csv" -User "Bram"
    .\import-visits.ps1 -ServerUrl "http://node4.eu.codehost.me:2002"
#>
param(
    [string]$File = "",
    [string]$User = "Bram",
    [string]$ServerUrl = "http://node4.eu.codehost.me:2002"
)

Write-Host "`n========================================================" -ForegroundColor Yellow
Write-Host "   💩 POEPENDE BRAM -- BULK IMPORT TOOL" -ForegroundColor Yellow
Write-Host "========================================================`n" -ForegroundColor Yellow

$content = ""

if ($File -ne "" -and (Test-Path $File)) {
    Write-Host "Inlezen van bestand: $File" -ForegroundColor Cyan
    $content = [System.IO.File]::ReadAllText((Resolve-Path $File).Path)
} else {
    Write-Host "Voer de gegevens in van de bezoeken." -ForegroundColor White
    Write-Host "Je kunt regels plakken uit Excel, Kladblok of CSV." -ForegroundColor Gray
    Write-Host "Ondersteunde formaten per regel:" -ForegroundColor Gray
    Write-Host "  14-05-2023, Station Utrecht, 5 min, 4, Fijn toilet" -ForegroundColor DarkGray
    Write-Host "  2023-06-10 | Shell A2 Amsterdam | 8 min | 5" -ForegroundColor DarkGray
    Write-Host "  02/09/2023; CafÃ© De Witte Bal Breda; 3 min; 3; Geen wc-papier" -ForegroundColor DarkGray
    Write-Host "`nTyp of plak je regels hieronder en sluit af met twee lege Enters (of typ 'KLAAR'):" -ForegroundColor Green
    
    $lines = @()
    while ($true) {
        $l = Read-Host
        if ($l.Trim() -eq "KLAAR" -or ($l.Trim() -eq "" -and $lines.Count -gt 0)) {
            break
        }
        if ($l.Trim() -ne "") {
            $lines += $l
        }
    }
    $content = $lines -join "`n"
}

if ($content.Trim() -eq "") {
    Write-Host "Geen gegevens opgegeven. Geannuleerd." -ForegroundColor Red
    exit 0
}

Write-Host "`nGebruiker voor deze bezoeken: " -NoNewline -ForegroundColor White
Write-Host "$User" -ForegroundColor Cyan

$bodyObj = @{
    csv = $content
    userName = $User
    userAvatar = ($User.Substring(0, 1).ToUpper())
}
$jsonBody = $bodyObj | ConvertTo-Json

$targetUrl = "$ServerUrl/api/sessions/batch"
Write-Host "Versturen naar server: $targetUrl..." -ForegroundColor Cyan

try {
    $response = Invoke-RestMethod -Uri $targetUrl -Method Post -Body $jsonBody -ContentType "application/json" -TimeoutSec 15
    if ($response.success) {
        Write-Host "`n========================================================" -ForegroundColor Green
        Write-Host "  ✅ SUCCESS! $($response.imported) bezoeken succesvol geÃ¯mporteerd!" -ForegroundColor Green
        Write-Host "  Totaal aantal bezoeken op server: $($response.sessions.Count)" -ForegroundColor Green
        Write-Host "========================================================`n" -ForegroundColor Green
    } else {
        Write-Host "Server melding: $($response.error)" -ForegroundColor Red
    }
} catch {
    Write-Host "Fout bij verbinding met server ($ServerUrl): $_" -ForegroundColor Red
    Write-Host "Bezig met lokale database fallback (data/database.json)..." -ForegroundColor Yellow

    $dbFile = Join-Path $PSScriptRoot "data\database.json"
    if (-not (Test-Path "data")) { New-Item -ItemType Directory -Path "data" -Force | Out-Null }
    
    $db = @{ sessions = @(); profiles = @(); accounts = @() }
    if (Test-Path $dbFile) {
        $db = Get-Content $dbFile -Raw | ConvertFrom-Json
    }

    # Lokale parse
    $rawLines = $content -split "`r?`n" | Where-Object { $_.Trim() -ne "" }
    $added = 0
    foreach ($line in $rawLines) {
        if ($line -match "datum|locatie|date|location") { continue }
        $parts = $line -split "[,;\t|]" | ForEach-Object { $_.Trim() }
        if ($parts.Count -ge 2) {
            $d = $parts[0]
            $loc = $parts[1]
            $newSess = [PSCustomObject]@{
                id = "sess-local-$([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())-$added"
                userId = "user-$($User.ToLower())"
                userName = $User
                userAvatar = $User.Substring(0,1).ToUpper()
                locationName = $loc
                latitude = 52.1326 + ((Get-Random -Minimum -500 -Maximum 500) / 1000.0)
                longitude = 5.2913 + ((Get-Random -Minimum -500 -Maximum 500) / 1000.0)
                timestamp = "$d`T12:00:00.000Z"
                durationSeconds = 300
                rating = 4
                poopType = "De Vlotte Boodschap"
                notes = if ($parts.Count -ge 5) { $parts[4] } else { $null }
                tags = @()
                userEmail = $null
            }
            $db.sessions = @($newSess) + @($db.sessions)
            $added++
        }
    }
    $db | ConvertTo-Json -Depth 10 | Set-Content $dbFile -Encoding UTF8
    Write-Host "✅ $added bezoeken lokaal opgeslagen in data/database.json!" -ForegroundColor Green
}
