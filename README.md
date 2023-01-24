# Chad chat

Undoubtedly the chaddest among chats.

---

## Backend

### CLI tools

#### Database

```bash
cargo install sqlx-cli --no-default-features --features native-tls, postgres
```

#### Watch

For easier development Cargo Watch watches over your project's source for changes, and runs Cargo commands when they occur.

```bash
cargo install cargo-watch
```

### Configuration

#### Directory: `backend/configuration/settings.toml`

Example settings:

```toml
[app]
host = "127.0.0.1"
port = 3000
access_jwt_secret = "ACCESS_JWT_SECRET"
refresh_jwt_secret = "REFRESH_JWT_SECRET"
origin = "127.0.0.1" 

[postgres]
database_url = "postgresql://postgres@localhost:5432/chad_chat"
# determines whether database should migrate automatically, defaults to 'true'
is_migrating = false

[postgres.fields]
username = "postgres"
password = "leave_empty_if_you_wish"
port = 5432
host = "localhost"
database_name = "chad_chat"

[redis]
database_url = "redis://127.0.0.1/0"

[redis.fields]
username = "default"
password = "leave_empty_if_you_wish"
port = 6379
host = "127.0.0.1"
database_name = "0" # redis uses indexes in range (0-15)

[smtp]
username = "bob"
password = "smtp_key"
relay = "smtp.gmail.com"
address = "bob@gmail.com" # from email field
```

> **Note**
> Most fields have corresponding uppercase environment variables names.

##### Order of database url sourcing

`database_url -> fields -> environment variable`

---

## Development

To focus on coding rather than compiling after changes use:

`/backend`

```bash
cargo watch -x run
```

`/frontend`

```bash
npm run dev
```


## Building

Regardless of whether backend is running or not, newer build will be served at `/frontend/dist`
```bash
npm run build
```
