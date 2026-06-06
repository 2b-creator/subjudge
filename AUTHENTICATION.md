# Authentication Documentation

## Overview

The API now supports **HTTP Basic Authentication (RFC 7617)** as the primary authentication method, with **Bearer token (JWT)** as an alternative method.

## Authentication Methods

### 1. HTTP Basic Authentication (Primary Method)

HTTP Basic Authentication is the standard and recommended method for accessing the API.

#### How It Works

Send your username and password encoded in Base64 in the `Authorization` header with every request:

```
Authorization: Basic <base64(username:password)>
```

#### Example Using cURL

```bash
curl -u username:password http://localhost:3000/api/submissions
```

Or manually with Base64 encoding:

```bash
# Encode credentials
echo -n "username:password" | base64
# Result: dXNlcm5hbWU6cGFzc3dvcmQ=

# Use in request
curl -H "Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ=" http://localhost:3000/api/submissions
```

#### Example Using JavaScript

```javascript
const username = 'your_username';
const password = 'your_password';
const credentials = btoa(`${username}:${password}`);

fetch('http://localhost:3000/api/submissions', {
  headers: {
    'Authorization': `Basic ${credentials}`
  }
})
  .then(response => response.json())
  .then(data => console.log(data));
```

#### Example Using Python

```python
import requests
from requests.auth import HTTPBasicAuth

response = requests.get(
    'http://localhost:3000/api/submissions',
    auth=HTTPBasicAuth('username', 'password')
)
print(response.json())
```

### 2. Bearer Token (JWT) - Alternative Method

For applications that prefer token-based authentication, you can obtain a JWT token and use it for subsequent requests.

#### Step 1: Login to Get Token

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "your_username", "password": "your_password"}'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

#### Step 2: Use Token in Subsequent Requests

```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  http://localhost:3000/api/submissions
```

**Note:** Tokens expire after 24 hours (86400 seconds).

## API Endpoints

### Public Endpoints (No Authentication Required)

- `GET /api/version` - Get API version
- `GET /api/auth/health` - Health check for authentication service
- `POST /api/auth/login` - Login to obtain JWT token

### Protected Endpoints (Authentication Required)

All other endpoints require authentication using either HTTP Basic Auth or Bearer token:

- `GET /api/submissions` - List submissions
- `GET /api/auth/me` - Get current user information
- `GET /api/contests/{id}` - Get contest details
- `PATCH /api/contests/{id}` - Update contest
- `GET /api/contests/{id}/access` - Get access control for contest
- `GET /api/contests/{id}/teams` - Get contest teams
- `GET /api/contests/{id}/judgement-types` - Get judgement types
- `GET /api/contests/{id}/judgement-types/{judgement_type_id}` - Get specific judgement type
- `POST /api/sync/teams` - Synchronize teams
- `POST /api/sync/groups` - Synchronize groups
- `POST /api/sync/contests` - Synchronize contests
- `POST /api/sync/organizations` - Synchronize organizations

## Error Responses

### 401 Unauthorized

Returned when authentication fails or is missing:

```json
{
  "error": "Missing or invalid authorization header. Use HTTP Basic Auth or Bearer token."
}
```

Or for specific authentication failures:

```json
{
  "error": "Invalid username or password"
}
```

### 500 Internal Server Error

Returned when there's a server-side error (e.g., database connection issues).

## Security Considerations

1. **HTTPS Required**: In production, always use HTTPS to protect credentials in transit. HTTP Basic Auth sends credentials with every request, making HTTPS essential.

2. **Password Storage**: All passwords are hashed using bcrypt before storage in the database.

3. **Token Expiration**: JWT tokens expire after 24 hours. Use the `/api/auth/login` endpoint to obtain a new token.

4. **Credentials Management**: 
   - Never hardcode credentials in your application
   - Use environment variables or secure configuration management
   - Rotate passwords regularly

## Implementation Details

### How Authentication Works Internally

1. **HTTP Basic Auth**: 
   - On each request, credentials are extracted from the Authorization header
   - Username is looked up in the database
   - Password is verified against the stored bcrypt hash
   - User role and permissions are determined
   - Request proceeds if authentication succeeds

2. **Bearer Token (JWT)**:
   - Token is decoded and validated
   - Claims are extracted (user ID, username, role, team ID)
   - Token expiration is checked
   - Request proceeds if token is valid

### User Roles

The system supports the following roles:

- **Admin**: Full access to all contest operations
- **Judge**: Can evaluate submissions and view all data
- **Team**: Can submit solutions and view own data
- **Public**: Unauthenticated users (limited read-only access)

Roles are determined from the `account_type` field in the accounts table.

## Testing Authentication

### Test HTTP Basic Auth

```bash
# Should succeed with valid credentials
curl -u testuser:testpass http://localhost:3000/api/auth/me

# Should fail with invalid credentials
curl -u testuser:wrongpass http://localhost:3000/api/auth/me
```

### Test JWT Token Auth

```bash
# Get token
TOKEN=$(curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' \
  | jq -r '.token')

# Use token
curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/auth/me
```

### Test Public Endpoints

```bash
# Should work without authentication
curl http://localhost:3000/api/version
curl http://localhost:3000/api/auth/health
```

## Migration Guide

If you're migrating from a JWT-only system:

1. **Existing JWT tokens will continue to work** - no immediate changes needed
2. **Update clients gradually** to use HTTP Basic Auth for better standards compliance
3. **Monitor usage** of both authentication methods
4. **Consider deprecating** JWT tokens in the future if HTTP Basic Auth meets all needs

## Compliance

This implementation follows:

- **RFC 7617**: The 'Basic' HTTP Authentication Scheme
- **RFC 7235**: Hypertext Transfer Protocol (HTTP/1.1): Authentication
- **CLICS Specification**: Contest API authentication requirements
