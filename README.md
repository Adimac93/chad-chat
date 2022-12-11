# Chad chat

Undoubtedly the chaddest among chats.

---

## Backend

### CLI tools

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
# allows using API from remote origin if remote (non local) ip is provided
origin = "127.0.0.1" 

[database]
database_url = "postgresql://postgres@localhost:5432/chad_chat"
# determines whether database should migrate automatically, defaults to 'true'
is_migrating = false
[database.fields]
username = "postgres"
password = "leave_empty_if_you_wish"
port = 5432
host = "localhost"
database_name = "chad_chat"
```

> **Note**
> Most fields have corresponding uppercase environment variables names.

##### Order of database url sourcing

`database_url -> fields -> environment variable`

---

## Development

To focus on coding rather than compiling after changes during development you can use:

`/backend`

```bash
cargo watch -x run
```

`/frontend`

```bash
npm run dev
```

## Frontend

Nothing to see here now.
Run `npm run dev` and GLHF!
