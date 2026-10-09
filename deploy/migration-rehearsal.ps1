$ErrorActionPreference = 'Stop'
$env:ITANONGMO_BIND = '127.0.0.1:4390'
$env:ITANONGMO_DATABASE_URL = 'sqlite://var/live-migration-rehearsal.db?mode=rwc'
$env:ITANONGMO_DATA_DIR = 'var/migration-rehearsal'
$env:ITANONGMO_PUBLIC_ORIGIN = 'http://127.0.0.1:4390'
$env:ITANONGMO_REQUESTER_AI_ENABLED = 'false'
$env:ITANONGMO_DEMO_RESET_ON_START = 'false'

$process = Start-Process `
    -FilePath '.\target\debug\itanongmo.exe' `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput 'var\migration-rehearsal.stdout.log' `
    -RedirectStandardError 'var\migration-rehearsal.stderr.log'

try {
    $health = $null
    foreach ($attempt in 1..30) {
        try {
            $health = Invoke-RestMethod 'http://127.0.0.1:4390/health'
            break
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if ($null -eq $health) { throw 'Migration rehearsal health check timed out.' }

    $existing = Invoke-WebRequest 'http://127.0.0.1:4390/api/v1/requests/DOST-ECC8ABF76D9A' -UseBasicParsing
    if ($existing.StatusCode -ne 200) { throw 'Existing live request was not preserved.' }

    $batch = Invoke-WebRequest `
        'http://127.0.0.1:4390/api/v1/submissions' `
        -Method Post `
        -ContentType 'application/json' `
        -Body (@{
            agency_ids = 'agency-dost,agency-da'
            requester_name = 'Migration Rehearsal'
            requester_email = 'migration-rehearsal@example.test'
            title = 'Synthetic migration rehearsal request'
            description = 'Provide synthetic records for the copied database migration rehearsal only.'
            purpose = 'Migration verification'
            urgency_details = ''
            consent = 'yes'
        } | ConvertTo-Json) `
        -UseBasicParsing
    if ($batch.StatusCode -ne 201 -or $batch.Content -notmatch 'FOI-BATCH-') {
        throw 'New batch could not be created after migration.'
    }

    [pscustomobject]@{
        health = $health.status
        existing_request = 'preserved'
        new_multi_agency_batch = 'created'
    } | ConvertTo-Json -Compress
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id
        Wait-Process -Id $process.Id -ErrorAction SilentlyContinue
    }
}
