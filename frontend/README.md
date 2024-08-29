# Threefold RFS

## Description

`Threefold RFS` is a frontend that helps manage the RFS server for creating, mounting, and extracting FungiStore lists, or fl for short. An fl is a simple format that stores information about a whole filesystem in a compact way. It doesn't hold the actual data but includes enough details to retrieve the data from a store.

## Prerequesites

- build essentials

  ```bash
  sudo apt-get install build-essential
  ```

- [node js](https://nodejs.org/en/download/package-manager)
- [rust](https://www.rust-lang.org/tools/install)
- Cargo, to be configured to run in the shell
- musl tool

  ```bash
      sudo apt install musl-tools
  ```

## Installation

```bash
    git clone https://github.com/threefoldtech/rfs.git
```

### backend

In fl-server dir:

- create flists dir containaing dirs for each user
  ex:
  - fl-server
    - flists
      - user1
      - user2
- include config file
  ex:

  ```yml
      host='localhost'
      port=4000
      store_url=['dir:///tmp/store0']
      flist_dir='flists'

      jwt_secret='secret'
      jwt_expire_hours=5

      [[users]] # list of authorized user in the server
      username = "user1"
      password = "password1"

      [[users]]
      username = "user2"
      password = "password2"
  ```

- Move to `fl-server` directory and execute the following command to run the backend:

  ```bash
  cargo run --bin fl-server -- --config-path config.toml
  ```

### frontend

- Move to `frontend` directory, open new terminal and execute the following commands to run the frontend:

  ```bash
      npm install
      npm run dev
  ```

## Usage

- Login with users listed in config.toml with their username and password
- Create Flist
- Preview Flist
- List all Flists
- Download Flist
