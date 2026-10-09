$ErrorActionPreference = 'Stop'
$env:ITANONGMO_BIND = '127.0.0.1:4387'
$env:ITANONGMO_DATABASE_URL = 'sqlite://var/smoke.db?mode=rwc'
$env:ITANONGMO_DATA_DIR = 'var/smoke'
$env:ITANONGMO_PUBLIC_ORIGIN = 'http://127.0.0.1:4387'
$env:ITANONGMO_FRONTEND_ORIGIN = 'https://example.github.io'
$env:ITANONGMO_DEMO_RESET_ON_START = 'true'

$process = Start-Process `
    -FilePath '.\target\debug\itanongmo.exe' `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput 'var\smoke.stdout.log' `
    -RedirectStandardError 'var\smoke.stderr.log'

try {
    $health = $null
    foreach ($attempt in 1..30) {
        try {
            $health = Invoke-RestMethod 'http://127.0.0.1:4387/health'
            break
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if ($null -eq $health) { throw 'Health check timed out.' }

    foreach ($path in '/', '/api/v1/agencies', '/api/v1/government/personas') {
        $response = Invoke-WebRequest "http://127.0.0.1:4387$path" -UseBasicParsing
        if ($response.StatusCode -ne 200) { throw "Request failed: $path" }
    }

    foreach ($path in '/agencies', '/government/personas', '/requests') {
        try {
            $legacyStatus = (Invoke-WebRequest "http://127.0.0.1:4387$path" -UseBasicParsing).StatusCode
        }
        catch {
            $legacyStatus = [int]$_.Exception.Response.StatusCode
        }
        if ($legacyStatus -ne 404) { throw "Legacy HTML route remains registered: $path" }
    }

    $search = Invoke-RestMethod 'http://127.0.0.1:4387/api/v1/agencies/search?q=of%20department%20AgRiCultUre'
    if ($search.data.Count -ne 1 -or $search.data[0].short_name -ne 'DA') {
        throw 'Duplicate-aware agency search smoke check failed.'
    }

    $apiBatch = Invoke-WebRequest `
        'http://127.0.0.1:4387/api/v1/submissions' `
        -Method Post `
        -Headers @{ Origin = 'https://example.github.io' } `
        -ContentType 'application/json' `
        -Body (@{
            agency_ids = 'agency-dost,agency-da'
            requester_name = 'GitHub Pages Smoke Requester'
            requester_email = 'pages-smoke@example.test'
            title = 'Static frontend API smoke request'
            description = 'Provide the synthetic project registry used only for static frontend verification.'
            purpose = 'Static frontend verification'
            urgency_details = ''
            consent = 'yes'
        } | ConvertTo-Json) `
        -UseBasicParsing
    $apiBatchBody = $apiBatch.Content | ConvertFrom-Json
    if ($apiBatch.StatusCode -ne 201 -or $apiBatchBody.data.requests.Count -ne 2) {
        throw 'GitHub Pages submission API smoke check failed.'
    }
    if ($apiBatch.Headers['Access-Control-Allow-Origin'] -ne 'https://example.github.io') {
        throw 'Exact-origin CORS smoke check failed.'
    }
    $trackedReference = $apiBatchBody.data.requests[0].reference
    $requesterToken = $apiBatchBody.data.requester_access_token
    if ($requesterToken.Length -ne 64) { throw 'Requester capability token smoke check failed.' }
    $reply = Invoke-WebRequest `
        "http://127.0.0.1:4387/api/v1/requests/$trackedReference/messages" `
        -Method Post `
        -Headers @{ Origin = 'https://example.github.io'; Authorization = "Bearer $requesterToken" } `
        -ContentType 'application/json' `
        -Body (@{ body = 'Synthetic requester follow-up for capability verification.' } | ConvertTo-Json) `
        -UseBasicParsing
    if ($reply.StatusCode -ne 201) { throw 'Requester reply capability smoke check failed.' }
    $tracked = Invoke-RestMethod "http://127.0.0.1:4387/api/v1/requests/$trackedReference"
    if ($tracked.data.reference -ne $trackedReference -or $tracked.data.messages.Count -ne 1) {
        throw 'Static frontend tracking API smoke check failed.'
    }

    try {
        $disabledAiStatus = (Invoke-WebRequest `
            'http://127.0.0.1:4387/api/v1/requester-ai/draft' `
            -Method Post `
            -ContentType 'application/json' `
            -Body (@{
                provider = 'openai'
                model = 'gpt-4o-mini'
                what_to_ask = 'Synthetic AI feature gate verification details.'
                manual_title = 'Synthetic feature gate verification'
            } | ConvertTo-Json) `
            -UseBasicParsing).StatusCode
    }
    catch {
        $disabledAiStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($disabledAiStatus -ne 503) { throw 'Requester AI feature gate smoke check failed.' }

    $session = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    $selectedPersona = Invoke-WebRequest `
        'http://127.0.0.1:4387/api/v1/government/session' `
        -Method Post `
        -Headers @{ Origin = 'https://example.github.io' } `
        -ContentType 'application/json' `
        -Body (@{ persona_id = 'dost-fro' } | ConvertTo-Json) `
        -WebSession $session `
        -UseBasicParsing
    $dashboard = Invoke-WebRequest `
        'http://127.0.0.1:4387/api/v1/government/board' `
        -Headers @{ Origin = 'https://example.github.io' } `
        -WebSession $session `
        -UseBasicParsing
    if ($selectedPersona.StatusCode -ne 200 -or $dashboard.StatusCode -ne 200) {
        throw 'Government JSON workspace smoke check failed.'
    }

    [pscustomobject]@{
        health = $health.status
        api_only_routes = 'ok'
        agency_search = 'ok'
        multi_agency_batch = 'ok'
        github_pages_api = 'ok'
        exact_origin_cors = 'ok'
        requester_reply = 'ok'
        requester_ai_gate = 'disabled'
        government_dashboard = 'ok'
    } | ConvertTo-Json -Compress
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id
        Wait-Process -Id $process.Id -ErrorAction SilentlyContinue
    }
}
