## `GET /v1/sessions/{uuid}`

### Response
```json
{
    "timestamp": "2025-01-02T00:00:00Z",
    "ip": "192.168.0.1",
    "grantor": {
        "name": "My Server",
        "version": "1.0.0-alpha.1"
    },
    "player": {
        "uuid": "567458bd-d97f-4fe1-8123-6b380998acbe",
        "username": "john",
        "discord": {
            "id": "123456789012345678",
            "username": "john"
        }
    }
}
```

## `POST /v1/sessions`

### Request Body
```http
POST /v1/sessions HTTP/1.1
Accept: */*
Authorization: Bearer <jwt secret key>
Content-Type: application/json
Idempotency-Key: abc-123-def-456
User-Agent: Eden Client v2.0.0

{
    "uuid": "567458bd-d97f-4fe1-8123-6b380998acbe",
    "ip": "192.168.0.1",
    "bedrock": false
}
```

### Response Cases
---

**Case 1: Registered Players**
```http
HTTP/1.1 201 Created
Content-Type: application/json

{
    "status": "granted",
    "last_login_at": "2025-01-01T00:00:00Z",
    "perks": ["infinite-stocks", "keep-inventory"],
    "discord": {
        "id": "123456789012345678",
        "username": "john"
    }
}
```

**Case 2: Rejected (timed out, closed, suspended, etc.)**
```http
HTTP/1.1 403 Forbidden
Content-Type: application/json

{
    "status": "rejected",
    "reason": "This server is only for registered players temporarily.",
    "note": "Please come to us again later."
}
```

**Case 3: Guests**
```http
HTTP/1.1 201 Created
Content-Type: application/json

{
    "status": "granted",
    "last_login_at": null,
    "perks": [],
    "discord": null
}
```