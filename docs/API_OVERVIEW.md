# SubJudge API Documentation

## Overview

SubJudge provides a REST API for managing programming contests. The API follows RESTful principles and returns JSON responses.

## Base URL

```
http://localhost:3000/api
```

## API Modules

The API is organized into three main modules:

### 1. Contest Management (`/api/contests`)

Endpoints for retrieving and modifying contest information.

- **GET** `/api/contests/{id}` - Get contest details
- **PATCH** `/api/contests/{id}` - Modify contest properties

### 2. Access Control (`/api/contests/{id}/access`)

Endpoint for determining what data and actions are available to the current client.

- **GET** `/api/contests/{id}/access` - Get access information

### 3. Data Synchronization (`/api/sync`)

Endpoints for bulk importing data from external sources.

- **POST** `/api/sync/teams` - Sync team data
- **POST** `/api/sync/groups` - Sync group data
- **POST** `/api/sync/contests` - Sync contest data
- **POST** `/api/sync/organizations` - Sync organization data

---

## Detailed Endpoint Documentation

### Contest Management

#### GET /api/contests/{id}

Retrieves detailed information about a specific contest.

**Path Parameters:**
- `id` (string, required) - Contest identifier

**Response:** `200 OK`
```json
{
  "id": "contest123",
  "name": "ICPC Regional",
  "formal_name": "ACM ICPC Regional Contest 2026",
  "start_time": "2026-06-10T09:00:00",
  "duration": "5:00:00",
  "scoreboard_type": "icpc",
  "scoreboard_freeze_duration": "1:00:00",
  "penalty_time": 20
}
```

**Error Responses:**
- `404 Not Found` - Contest doesn't exist
- `500 Internal Server Error` - Database error or validation failure

---

#### PATCH /api/contests/{id}

Modifies contest properties. The operation performed depends on the payload structure.

**Path Parameters:**
- `id` (string, required) - Contest identifier

**Operation 1: Modify Start Time**

Requires `contest_start` capability.

**Request Body:**
```json
{
  "id": "contest123",
  "start_time": "2026-06-10T09:00:00",
  "countdown_pause_time": null
}
```

**Fields:**
- `id` (string, required) - Must match path parameter
- `start_time` (datetime|null, required) - New start time or null to clear
- `countdown_pause_time` (string|null, optional) - Countdown pause in RELTIME format (e.g., "1:23:45")

**Constraints:**
- Cannot set both `start_time` and `countdown_pause_time` to non-null values
- Cannot modify if contest has started or starts within 30 seconds
- New start time cannot be in past or within 30 seconds of current time

**Response:** `204 No Content`

**Error Responses:**
- `400 Bad Request` - Invalid payload or ID mismatch
- `403 Forbidden` - Contest already started or imminent
- `404 Not Found` - Contest doesn't exist

---

**Operation 2: Modify Thaw Time**

Requires `contest_thaw` capability.

**Request Body:**
```json
{
  "id": "contest123",
  "scoreboard_thaw_time": "2026-06-10T15:00:00"
}
```

**Fields:**
- `id` (string, required) - Must match path parameter
- `scoreboard_thaw_time` (datetime, required) - When to unfreeze scoreboard

**Response:**
- `204 No Content` - Thaw scheduled for future time
- `200 OK` - Thaw time was in past, adjusted to current time (returns updated contest)

**Error Responses:**
- `400 Bad Request` - Invalid payload or ID mismatch
- `403 Forbidden` - Invalid contest state for thawing
- `404 Not Found` - Contest doesn't exist

---

### Access Control

#### GET /api/contests/{id}/access

Returns information about what endpoints and capabilities are available to the current client.

**Path Parameters:**
- `id` (string, required) - Contest identifier

**Response:** `200 OK`
```json
{
  "capabilities": [
    "team_submit",
    "clarification_request"
  ],
  "endpoints": [
    {
      "type": "contest",
      "properties": ["id", "name", "start_time", "duration"]
    },
    {
      "type": "problems",
      "properties": ["id", "label", "name", "ordinal"]
    }
  ]
}
```

**Access Levels:**

The response varies based on client role:

| Role | Capabilities | Visibility |
|------|-------------|------------|
| **Public** | None | Basic contest info, problem names, team names |
| **Team** | `team_submit`, `clarification_request` | Contest details, problems, own submissions/judgements |
| **Admin** | `contest_start`, `contest_stop`, `judge_submission`, `clarification_respond` | All data with full properties |

**Current Status:** Returns mock data for Public role. Authentication not yet implemented.

---

### Data Synchronization

All sync endpoints accept JSON arrays and perform upsert operations (insert or update on conflict).

#### POST /api/sync/organizations

Synchronizes organization data.

**Request Body:**
```json
[
  {
    "id": "org1",
    "name": "MIT",
    "formal_name": "Massachusetts Institute of Technology"
  }
]
```

**Response:** `200 OK`
```json
"Organizations sync completed"
```

**Updated Fields on Conflict:** `name`, `formal_name`

---

#### POST /api/sync/groups

Synchronizes group data.

**Request Body:**
```json
[
  {
    "id": "group1",
    "name": "Admins",
    "group_type": "system"
  }
]
```

**Response:** `200 OK`
```json
"Groups sync completed"
```

**Updated Fields on Conflict:** `name`, `group_type`

---

#### POST /api/sync/teams

Synchronizes team data and their group memberships. Uses database transactions for atomicity.

**Request Body:**
```json
[
  {
    "id": "team1",
    "name": "Alpha Team",
    "label": "alpha",
    "organization_id": "org1",
    "resources": {},
    "group_ids": ["group1", "group2"]
  }
]
```

**Special Fields:**
- `group_ids` (array[string], optional) - Group memberships (many-to-many relationship)

**Process:**
1. Extract `group_ids` from payload
2. Upsert team data
3. Delete existing team-group relationships
4. Insert new team-group relationships

**Response:** `200 OK`
```json
"Teams and relations sync completed"
```

**Updated Fields on Conflict:** `name`, `label`, `organization_id`, `resources`

**Transaction Safety:** All operations within a transaction—rollback on any failure.

---

#### POST /api/sync/contests

Synchronizes contest data.

**Request Body:**
```json
[
  {
    "id": "contest1",
    "name": "ICPC Regional",
    "formal_name": "ACM ICPC Regional Contest 2026",
    "start_time": "2026-06-10T09:00:00",
    "duration": "5:00:00",
    "scoreboard_type": "icpc"
  }
]
```

**Response:** `200 OK`
```json
"Contests sync completed"
```

**Updated Fields on Conflict:** `name`, `formal_name`, `start_time`, `duration`, `scoreboard_type`

---

## Common Patterns

### Error Handling

All endpoints return errors in a consistent format:

```json
"Error message describing what went wrong"
```

Common error types:
- `"Invalid JSON object"` - Malformed JSON
- `"Data format error: {details}"` - JSON doesn't match expected schema
- `"DB Error: {details}"` - Database operation failed

### Upsert Strategy

Sync endpoints use upsert (insert or update on conflict):
- If record with given ID doesn't exist → **insert**
- If record with given ID exists → **update** specific fields

This ensures idempotency—repeated sync operations produce the same result.

### Data Dependencies

When syncing related data, follow this order:

1. **Organizations** (no dependencies)
2. **Groups** (no dependencies)
3. **Teams** (depends on Organizations and Groups)
4. **Contests** (no dependencies)

---

## Authentication & Authorization

**Current Status:** Not yet implemented.

**Planned:**
- JWT token-based authentication
- Role-based access control (Public, Team, Admin)
- Capability-based permissions for actions

**Current Behavior:**
- All sync endpoints are open
- Access endpoint returns Public role permissions
- Contest management endpoints have basic validation only

---

## Data Types

### DateTime Format

All datetime fields use ISO 8601 format without timezone:
```
2026-06-10T09:00:00
```

**Note:** Times are stored as naive datetime (no timezone info).

### Duration Format

Two formats are supported:

**RELTIME** (relative time):
```
"5:00:00"  // 5 hours
"1:23:45"  // 1 hour, 23 minutes, 45 seconds
```

**ISO 8601 Duration**:
```
"PT5H"     // 5 hours
"PT1H23M45S" // 1 hour, 23 minutes, 45 seconds
```

---

## Rate Limiting

**Current Status:** Not implemented.

---

## Versioning

**Current Version:** v1 (implicit)

The API does not currently include version numbers in URLs. Breaking changes will be communicated before deployment.

---

## Examples

### Complete Sync Workflow

```bash
# 1. Sync organizations first
curl -X POST http://localhost:3000/api/sync/organizations \
  -H "Content-Type: application/json" \
  -d '[
    {"id":"org1","name":"MIT","formal_name":"Massachusetts Institute of Technology"}
  ]'

# 2. Sync groups
curl -X POST http://localhost:3000/api/sync/groups \
  -H "Content-Type: application/json" \
  -d '[
    {"id":"group1","name":"Participants","group_type":"team"}
  ]'

# 3. Sync teams (referencing organizations and groups)
curl -X POST http://localhost:3000/api/sync/teams \
  -H "Content-Type: application/json" \
  -d '[
    {
      "id":"team1",
      "name":"Alpha",
      "label":"alpha",
      "organization_id":"org1",
      "resources":{},
      "group_ids":["group1"]
    }
  ]'

# 4. Sync contests
curl -X POST http://localhost:3000/api/sync/contests \
  -H "Content-Type: application/json" \
  -d '[
    {
      "id":"contest1",
      "name":"ICPC Regional",
      "formal_name":"ACM ICPC Regional Contest 2026",
      "start_time":"2026-06-10T09:00:00",
      "duration":"5:00:00",
      "scoreboard_type":"icpc"
    }
  ]'
```

### Contest Management Workflow

```bash
# Get contest details
curl http://localhost:3000/api/contests/contest1

# Check access permissions
curl http://localhost:3000/api/contests/contest1/access

# Modify contest start time
curl -X PATCH http://localhost:3000/api/contests/contest1 \
  -H "Content-Type: application/json" \
  -d '{
    "id":"contest1",
    "start_time":"2026-06-10T10:00:00"
  }'

# Set scoreboard thaw time
curl -X PATCH http://localhost:3000/api/contests/contest1 \
  -H "Content-Type: application/json" \
  -d '{
    "id":"contest1",
    "scoreboard_thaw_time":"2026-06-10T15:30:00"
  }'
```

---

## Future Enhancements

1. **Authentication & Authorization**
   - JWT token authentication
   - Role-based access control
   - API key support for sync operations

2. **Pagination**
   - List endpoints for contests, teams, etc.
   - Cursor-based pagination

3. **Filtering & Sorting**
   - Query parameters for filtering results
   - Sortable responses

4. **Webhooks**
   - Event notifications for contest state changes
   - Submission result callbacks

5. **Rate Limiting**
   - Per-client request limits
   - Burst allowances

6. **Audit Logging**
   - Track all data modifications
   - Admin action history

---

## Support

For detailed implementation information, see:
- [Sync API Documentation](./API_SYNC.md)
- Rust API docs: `cargo doc --open`
