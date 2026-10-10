# Synthetic FOI demo cases

`requests.json` contains five fictional submissions for the existing `/api/v1/submissions` API. Four create DOST cases for the government board; one creates separate DOST and DA requests in a single batch. Each case has a `suggested_unit_id` for a manual routing demonstration. That field is presentation metadata and is not sent to the API.

Preview the cases without writing to the backend:

```powershell
./demo/seed-requests.ps1
```

Create them in a local backend:

```powershell
./demo/seed-requests.ps1 -ApiBaseUrl http://127.0.0.1:4320 -Apply
```

The script accepts an HTTPS API origin for an explicitly chosen remote demo backend. Each run creates new requests; it does not reset or deduplicate data. Guest submissions return public references and do not issue requester access tokens. Edit a case's `submission` fields to change its intake content or `suggested_unit_id` to change the walkthrough destination. Route cases through the authorized government UI after submission.

The fixtures contain synthetic names, addresses, agencies, and scenarios only. They do not represent real FOI submissions or real incidents.
