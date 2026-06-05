#!/bin/bash

# Test script for contest endpoints
BASE_URL="http://localhost:3000/api"

echo "=== Testing Contest Endpoints ==="
echo

# Test 1: GET contest by ID
echo "1. GET /api/contests/wf2014"
curl -X GET "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -w "\nStatus: %{http_code}\n\n"

# Test 2: PATCH contest start time (setting a future start time)
echo "2. PATCH /api/contests/wf2014 - Set start time"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wf2014",
    "start_time": "2026-12-25T10:00:00",
    "countdown_pause_time": null
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 3: PATCH contest start time (clear start time, set countdown pause)
echo "3. PATCH /api/contests/wf2014 - Clear start time, set countdown pause"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wf2014",
    "start_time": null,
    "countdown_pause_time": "1:23:45"
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 4: PATCH contest start time (invalid - both set, should fail with 400)
echo "4. PATCH /api/contests/wf2014 - Invalid: both start_time and countdown_pause_time set (should fail)"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wf2014",
    "start_time": "2026-12-25T10:00:00",
    "countdown_pause_time": "1:23:45"
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 5: PATCH scoreboard thaw time (future time)
echo "5. PATCH /api/contests/wf2014 - Set scoreboard thaw time (future)"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wf2014",
    "scoreboard_thaw_time": "2026-12-25T15:00:00"
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 6: PATCH scoreboard thaw time (past time - should return 200 with modified contest)
echo "6. PATCH /api/contests/wf2014 - Set scoreboard thaw time (past - should adjust)"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wf2014",
    "scoreboard_thaw_time": "2020-01-01T00:00:00"
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 7: PATCH with wrong contest ID (should fail with 400)
echo "7. PATCH /api/contests/wf2014 - Wrong ID in payload (should fail)"
curl -X PATCH "${BASE_URL}/contests/wf2014" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "wrong_id",
    "start_time": "2026-12-25T10:00:00"
  }' \
  -w "\nStatus: %{http_code}\n\n"

# Test 8: GET non-existent contest (should return 404)
echo "8. GET /api/contests/nonexistent (should return 404)"
curl -X GET "${BASE_URL}/contests/nonexistent" \
  -H "Content-Type: application/json" \
  -w "\nStatus: %{http_code}\n\n"

echo "=== Test completed ==="
