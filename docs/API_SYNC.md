# Sync API Documentation

## Overview

The Sync API module (`src/api/sync.rs`) provides REST endpoints for synchronizing external data into the SubJudge database. These endpoints are designed for idempotent data synchronization, meaning they can be safely called multiple times with the same data without causing duplicates or errors.

## Architecture Pattern

All sync endpoints follow the **upsert pattern** (insert or update on conflict):
- If a record with the given ID doesn't exist, it's inserted
- If a record with the given ID already exists, specific fields are updated
- This ensures data consistency during repeated synchronization operations

## Endpoints

### 1. POST `/api/sync/teams`

Synchronizes team data along with their group memberships.

**Special Handling**: This endpoint manages a many-to-many relationship between teams and groups through the `team_group` junction table.

**Transaction Safety**: Uses database transactions to ensure atomicity—if any step fails, all changes are rolled back.

**Request Body**:
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

**Process Flow**:
1. Begin database transaction
2. For each team:
   - Extract `group_ids` field (removed before model conversion)
   - Convert remaining fields to Team ActiveModel
   - Upsert team (updates: name, label, organization_id, resources)
   - Delete existing team-group relationships
   - Insert new team-group relationships
3. Commit transaction

**Response**: `"Teams and relations sync completed"`

---

### 2. POST `/api/sync/groups`

Synchronizes group data.

**Request Body**:
```json
[
  {
    "id": "group1",
    "name": "Admins",
    "group_type": "system"
  }
]
```

**Process Flow**:
1. Convert each JSON object to Groups ActiveModel using the `Syncable` trait
2. Upsert each group (updates: name, group_type)

**Response**: `"Groups sync completed"`

---

### 3. POST `/api/sync/contests`

Synchronizes contest data.

**Request Body**:
```json
[
  {
    "id": "contest1",
    "name": "ICPC Regional",
    "formal_name": "ACM ICPC Regional Contest 2026",
    "start_time": "2026-06-10T09:00:00Z",
    "duration": "5:00:00",
    "scoreboard_type": "icpc"
  }
]
```

**Process Flow**:
1. Convert each JSON object to Contests ActiveModel using the `Syncable` trait
2. Upsert each contest (updates: name, formal_name, start_time, duration, scoreboard_type)

**Response**: `"Contests sync completed"`

---

### 4. POST `/api/sync/organizations`

Synchronizes organization data.

**Request Body**:
```json
[
  {
    "id": "org1",
    "name": "MIT",
    "formal_name": "Massachusetts Institute of Technology"
  }
]
```

**Process Flow**:
1. Convert each JSON object to Organizations ActiveModel using the `Syncable` trait
2. Upsert each organization (updates: name, formal_name)

**Response**: `"Organizations sync completed"`

---

## Error Handling

All endpoints return `Result<String, String>`:
- **Success**: Returns a completion message
- **Failure**: Returns formatted error string with context

Common error scenarios:
- **Invalid JSON**: "Invalid JSON object" or "Data format error: {details}"
- **Database errors**: "DB Error: {details}"
- **Transaction errors**: Automatic rollback with error message

## Dependencies

- **Axum**: Web framework for handling HTTP requests
- **SeaORM**: ORM for database operations
- **Serde JSON**: JSON serialization/deserialization
- **Syncable Trait**: Custom trait from `models` module for JSON-to-ActiveModel conversion

## Design Decisions

### Why Upsert?

The upsert pattern ensures that sync operations are **idempotent**—running the same sync multiple times produces the same result. This is crucial for:
- Recovering from partial failures
- Handling duplicate sync requests
- Maintaining data consistency across systems

### Why Transactions Only for Teams?

The `sync_teams` endpoint is the only one using explicit transactions because it performs **multiple related operations**:
1. Insert/update team
2. Delete old group relationships
3. Insert new group relationships

If any step fails, the others must be rolled back to maintain referential integrity. Other sync endpoints operate on single entities without complex relationships, so transactions aren't required.

### Field Update Strategy

Each upsert explicitly lists which fields to update on conflict. This prevents:
- Overwriting fields that shouldn't change (like creation timestamps)
- Accidentally updating sensitive fields
- Introducing bugs from schema changes

## Usage Example

```bash
# Sync organizations first (teams depend on them)
curl -X POST http://localhost:3000/api/sync/organizations \
  -H "Content-Type: application/json" \
  -d '[{"id":"org1","name":"MIT","formal_name":"Massachusetts Institute of Technology"}]'

# Sync groups
curl -X POST http://localhost:3000/api/sync/groups \
  -H "Content-Type: application/json" \
  -d '[{"id":"group1","name":"Admins","group_type":"system"}]'

# Sync teams with group relationships
curl -X POST http://localhost:3000/api/sync/teams \
  -H "Content-Type: application/json" \
  -d '[{"id":"team1","name":"Alpha","label":"alpha","organization_id":"org1","resources":{},"group_ids":["group1"]}]'

# Sync contests
curl -X POST http://localhost:3000/api/sync/contests \
  -H "Content-Type: application/json" \
  -d '[{"id":"contest1","name":"ICPC","formal_name":"ICPC Regional 2026","start_time":"2026-06-10T09:00:00Z","duration":"5:00:00","scoreboard_type":"icpc"}]'
```

## Future Improvements

1. **Batch Size Limits**: Add validation for maximum array size to prevent memory issues
2. **Partial Success Reporting**: Return details about which records succeeded/failed
3. **Conflict Resolution Strategies**: Allow clients to specify merge behavior
4. **Audit Logging**: Track sync operations for debugging and compliance
5. **Async Batch Processing**: Process large sync operations in background jobs
