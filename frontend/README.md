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

Please check [rfs server](../rfs/README.md#server-command)

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
