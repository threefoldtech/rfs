# Flist server

Flist server helps using rfs and docker2fl tools to generate different flists from docker images.

## Build

```bash
cargo build
```

## Run

First create `config.toml` check [configuration](#configuration)

```bash
cargo run --bin fl-server -- --config-path config.toml -d
```

### Configuration

Before building or running the server, create `config.toml` in the current directory.

example `config.toml`:

```toml
host="Your host to run the server on, required, example: 'localhost'"
port="Your port to run the server on, required, example: 3000, validation: between [0, 65535]"
store_url="List of stores to pack flists in which can be 'dir', 'zdb', 's3', required, example: ['dir:///tmp/store0']"
flist_dir="A directory to save each user flists, required, example: 'flists'"

jwt_secret="secret for jwt, required, example: 'secret'"
jwt_expire_hours="Life time for jwt token in hours, required, example: 5, validation: between [1, 24]"

[[users]] # list of authorized user in the server
username = "user1"
password = "password1"

[[users]]
username = "user2"
password = "password2"
...
```
