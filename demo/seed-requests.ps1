param(
    [string]$ApiBaseUrl = 'http://127.0.0.1:4320',
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'
$fixturePath = Join-Path $PSScriptRoot 'requests.json'
$fixture = Get-Content -LiteralPath $fixturePath -Raw | ConvertFrom-Json
if (@($fixture.cases).Count -eq 0) {
    throw 'The demo fixture must contain at least one case.'
}
foreach ($demoCase in $fixture.cases) {
    if (-not $demoCase.scenario -or -not $demoCase.suggested_unit_id -or
        -not $demoCase.submission.agency_ids -or -not $demoCase.submission.requester_email -or
        -not $demoCase.submission.title -or -not $demoCase.submission.description -or
        -not $demoCase.submission.purpose -or $demoCase.submission.consent -ne 'yes') {
        throw 'A demo case is missing required submission or routing fields.'
    }
}
$baseUri = $null
if (-not [Uri]::TryCreate($ApiBaseUrl, [UriKind]::Absolute, [ref]$baseUri) -or
    $baseUri.Scheme -notin @('http', 'https') -or
    ($baseUri.Scheme -eq 'http' -and $baseUri.Host -notin @('127.0.0.1', 'localhost')) -or
    $baseUri.AbsolutePath -ne '/' -or
    $baseUri.Query -or $baseUri.Fragment -or $baseUri.UserInfo) {
    throw 'ApiBaseUrl must be an HTTPS origin or a loopback HTTP origin without a path.'
}

if (-not $Apply) {
    Write-Output "Dry run: $($fixture.cases.Count) synthetic cases. Add -Apply to create them."
    foreach ($demoCase in $fixture.cases) {
        Write-Output "$($demoCase.scenario) -> $($demoCase.suggested_unit_id)"
    }
    return
}

$endpoint = "$($baseUri.GetLeftPart([UriPartial]::Authority))/api/v1/submissions"
try {
    $apiRoot = Invoke-RestMethod -Uri $baseUri.GetLeftPart([UriPartial]::Authority) -Method Get -TimeoutSec 10
} catch {
    throw 'The selected origin does not expose the iTanongMo API root. No demo requests were created.'
}
if ($apiRoot.service -ne 'iTanongMo API') {
    throw 'The selected origin is not the iTanongMo API. No demo requests were created.'
}
foreach ($demoCase in $fixture.cases) {
    try {
        $body = $demoCase.submission | ConvertTo-Json -Depth 4 -Compress
        $result = Invoke-RestMethod -Uri $endpoint -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 30
        $references = @($result.data.requests | ForEach-Object { $_.reference }) -join ', '
        Write-Output "$($demoCase.scenario): $references"
    } catch {
        $status = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 'network error' }
        throw "Demo submission failed for '$($demoCase.scenario)' ($status). Earlier cases may already exist; inspect the API before retrying."
    }
}
