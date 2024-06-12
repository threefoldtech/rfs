#!/bin/bash

set -ex

if [ -z ${DOMAIN+x} ]
then
    echo 'Error! $DOMAIN is required.'
    exit 64
fi

# Deploy a vm for garage server with mycelium and public IP for s3 server TODO: mycelium and remove public IP

tfcmd deploy vm --name s3_server --ssh ~/.ssh/id_rsa.pub --cpu 8 --memory 16 --disk 50 --ipv4 --rootfs 10
sleep 6 # wait deployment
OUTPUT=$(tfcmd get vm s3_server 2>&1 | tail -n +3 | tr { '\n' | tr , '\n' | tr } '\n')
MYCELIUM_IP=$(echo "$OUTPUT" | grep -Eo '"mycelium_ip"[^,]*' | awk  -F'"' '{print $4}')
PUBLIC_IP=$(echo "$OUTPUT" | grep -Eo '"computedip"[^,]*' | awk  -F'"' '{print $4}' | cut -d/ -f1-1)

# Deploy a name gateway to expose a domain for garage web

tfcmd deploy gateway name -n flist.$DOMAIN --backends http://$PUBLIC_IP:3902
tfcmd deploy gateway name -n blobs.$DOMAIN --backends http://$PUBLIC_IP:3902
sleep 6 # wait deployment
OUTPUT=$(tfcmd get gateway name $DOMAIN 2>&1 | tail -n +3 | tr { '\n' | tr , '\n' | tr } '\n')
FQDN=$(echo "$OUTPUT" | grep -Eo '"FQDN"[^,]*' | awk  -F'"' '{print $4}')

# Expose S3 server over public IP (mycelium not suppoerted yet) (garage is used) TODO: mycelium and remove public IP

ssh root@$PUBLIC_IP "
wget https://garagehq.deuxfleurs.fr/_releases/v1.0.0/x86_64-unknown-linux-musl/garage
chmod +x garage
mv garage /usr/local/bin

cat > /etc/garage.toml <<EOF
metadata_dir = '/home/meta'
data_dir = '/home/data'
db_engine = 'sqlite'

replication_factor = 1

rpc_bind_addr = '[::]:3901'
rpc_public_addr = '0.0.0.0:3901'
rpc_secret = '$(openssl rand -hex 32)'

[s3_api]
s3_region = 'garage'
api_bind_addr = '[::]:3900'
root_domain = '.s3.garage.localhost'

[s3_web]
bind_addr = '[::]:3902'
root_domain = '.$FQDN'
index = 'index.html'

[k2v_api]
api_bind_addr = '[::]:3904'

[admin]
api_bind_addr = '[::]:3903'
admin_token = '$(openssl rand -base64 32)'
metrics_token = '$(openssl rand -base64 32)'
EOF

garage server > output.log 2>&1 &
"