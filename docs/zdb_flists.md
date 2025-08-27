
# Generate an flist using ZDB

## Deploy a vm

1. Deploy a vm with a public IP
2. add docker (don't forget to add a disk for it with mountpoint = "/var/lib/docker")
3. add caddy

## Install zdb and run an instance of it

1. Execute `git clone -b development-v2 https://github.com/threefoldtech/0-db /zdb` then `cd /zdb`
2. Build

      ```bash
      cd libzdb
      make
      cd ..

      cd zdbd
      make STATIC=1
      cd ..

      make
      ```

3. Install `make install`
4. run `zdb --listen 0.0.0.0`
5. The result info you should know

      ```console
      zdbEndpoint = "<vm public IP>:<port>"
      zdbNameSpace = "default"
      zdbPassword = "default"
      ```

## Install rfs

1. Execute `git clone -b development-v2 https://github.com/threefoldtech/rfs` then `cd /rfs`
2. Execute

      ```bash
      rustup target add x86_64-unknown-linux-musl`
      cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
      mv ./target/x86_64-unknown-linux-musl/release/rfs /usr/bin/
      ```

## Convert docker image to an fl

1. Try an image for example `threefolddev/ubuntu:22.04` image
2. Executing `rfs docker -i threefolddev/ubuntu:22.04 -s "zdb://<vm public IP>:<port>/default" -d`
3. You will end up having `threefolddev-ubuntu-22.04.fl` (flist)

## Serve the flist using caddy

1. In the directory includes the output flist, you can run `caddy file-server --listen 0.0.0.0:2015 --browse`
2. The flist will be available as `http://<vm public IP>:2015/threefolddev-ubuntu-22.04.fl`
3. Use the flist to deploy any virtual machine.
