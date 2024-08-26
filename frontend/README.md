# fl-server frontend

## Installation
```bash
    git clone https://github.com/threefoldtech/rfs.git
```
#### backend
 - make sure to have rust before running this command [install rust](https://www.rust-lang.org/tools/install)
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
        npm run dev
    ```
