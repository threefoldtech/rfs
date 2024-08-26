# fl-server frontend
## Description

`rfs frontend` is a frontend to manage rfs server to create, mount and extract FungiStore lists (FungiList)`fl` for short. An `fl` is a simple format
to keep information about an entire filesystem in a compact form. It does not hold the data itself but enough information to
retrieve this data back from a `store`.

## Prerequesites
- [node js](https://nodejs.org/en/download/package-manager)
- [rust](https://www.rust-lang.org/tools/install)
## Installation
```bash
    git clone https://github.com/threefoldtech/rfs.git
```
#### backend
 In fl-server dir:
- create flists dir containaing dirs for each user 
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

- execute the following command to run the backend:
   
    ``` bash
    cargo run --bin fl-server -- --config-path config.toml
    ```
#### frontend

- execute the following command to run the frontend:
    ```bash
        npm install
        npm run dev
    ```
## Usage
- Create Flist 
- Preview Flist
- List all Flists
- Download Flist