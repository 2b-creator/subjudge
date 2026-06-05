# Contest API - cURL Examples

## 1. Create/Update Contest (via Sync Endpoint)

### Basic Contest
```bash
curl -X POST http://localhost:3000/api/sync/contests \
  -H "Content-Type: application/json" \
  -d '[
    {
      "id": "icpc2026",
      "name": "ICPC Regional 2026",
      "formal_name": "ACM International Collegiate Programming Contest Regional 2026",
      "start_time": "2026-06-15T09:00:00",
      "duration": "5:00:00",
      "scoreboard_freeze_duration": "1:00:00",
      "scoreboard_type": "pass-fail",
      "penalty_time": "0:20:00",
      "main_scoreboard_group_id": null,
      "countdown_pause_time": null,
      "scoreboard_thaw_time": null,
      "banner": null,
      "logo": null,
      "location": null
    }
  ]'
```

### Multiple Contests
```bash
curl -X POST http://localhost:3000/api/sync/contests \
  -H "Content-Type: application/json" \
  -d '[
    {
      "id": "icpc2026",
      "name": "ICPC Regional 2026",
      "duration": "5:00:00",
      "scoreboard_type": "pass-fail"
    },
    {
      "id": "ioi2026",
      "name": "IOI 2026",
      "duration": "5:00:00",
      "scoreboard_type": "score"
    }
  ]'
```

## 2. Retrieve Contest

```bash
curl http://localhost:3000/api/contests/icpc2026
```

### With jq formatting
```bash
curl -s http://localhost:3000/api/contests/icpc2026 | jq .
```

## 3. Get Access Control Information

```bash
curl http://localhost:3000/api/contests/icpc2026/access
```

### Response (Public role)
```json
{
  "capabilities": [],
  "endpoints": [
    {
      "type": "contest",
      "properties": ["id", "name", "formal_name", "start_time", "duration"]
    },
    {
      "type": "problems",
      "properties": ["id", "label", "name", "ordinal"]
    },
    {
      "type": "teams",
      "properties": ["id", "name"]
    }
  ]
}
```

## 4. Update Contest Start Time

```bash
curl -X PATCH http://localhost:3000/api/contests/icpc2026 \
  -H "Content-Type: application/json" \
  -d '{
    "id": "icpc2026",
    "start_time": "2026-06-20T10:00:00"
  }'
```

## 5. Update Scoreboard Thaw Time

```bash
curl -X PATCH http://localhost:3000/api/contests/icpc2026 \
  -H "Content-Type: application/json" \
  -d '{
    "id": "icpc2026",
    "scoreboard_thaw_time": "2026-06-15T15:00:00"
  }'
```

## Field Reference

### Required Fields
- `id`: Contest identifier (string)
- `name`: Contest name (string)
- `duration`: Contest duration in RELTIME format (e.g., "5:00:00")
- `scoreboard_type`: Either "pass-fail" or "score"

### Optional Fields
- `formal_name`: Official contest name
- `start_time`: When the contest begins (ISO 8601 datetime)
- `countdown_pause_time`: Pause time in RELTIME format
- `scoreboard_freeze_duration`: Duration before end when scoreboard freezes
- `scoreboard_thaw_time`: When to unfreeze the scoreboard
- `main_scoreboard_group_id`: Group ID for main scoreboard
- `penalty_time`: Penalty for wrong submission (RELTIME)
- `banner`: Array of FILE objects (8:1 aspect ratio)
- `logo`: Array of FILE objects (1:1 aspect ratio)
- `location`: LOCATION object

## Access Control Roles

### Public (Unauthenticated)
- **Capabilities**: None (read-only)
- **Endpoints**: contest, problems, teams (limited properties)

### Team (Authenticated Participant)
- **Capabilities**: team_submit, clarification_request
- **Endpoints**: contest, problems, teams, submissions, judgements, clarifications

### Admin (Judge/Administrator)
- **Capabilities**: contest_start, contest_stop, judge_submission, clarification_respond
- **Endpoints**: All endpoints with full properties
