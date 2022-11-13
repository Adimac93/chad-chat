# Configuration

## Backend
Directory: `./backend/config/settings.toml`

### Example
```toml
[database]
host = "localhost"
database_name = "chad_chat"
username = "postgres"
password = ""
port = 5432

[addr]
ip = [127, 0, 0, 1]
port = 3000

[jwt]
secret = "very_very_secret_token"
```

## Frontend
Directory: `./frontend/config/settings.json`

### Example
```json
{
  "origin": {
    "ip": "localhost",
    "port": 5173,
    "secure": false
  }
}
```
