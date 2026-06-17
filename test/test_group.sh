curl -v -X POST http://localhost:3000/api/sync/groups \
     -H "Content-Type: application/json" \
     -d '[{"id":"g1","name":"Super Region 1","type":"super-region","location":"Global"}]'

curl -v -X POST http://localhost:3000/api/sync/teams \
    -H "Content-Type: application/json" \
    -d '[{"id": "t1", "name": "Team", "label": "1","group_ids": ["g1"], "resources": {}}]'