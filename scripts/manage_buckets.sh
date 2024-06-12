#!/bin/bash

set -ex 

if [ -z ${PUBLIC_IP+x} ]
then
    echo 'Error! $PUBLIC_IP is required.'
    exit 64
fi

# Create flist bucket and blobs bucket for rfs store and allow web for both

NODE_ID=$(ssh root@$PUBLIC_IP "garage status | awk 'NR==3{print \$1}'")

ssh root@$PUBLIC_IP "
garage layout assign -z dc1 -c 1G $NODE_ID
garage layout apply --version 1
garage bucket create blobs
garage bucket create flist
garage bucket website --allow flist
garage bucket website --allow blobs
garage bucket list
"

# We need to generate a key allowing read and write permissions
# This allow us to upload different files over the s3 server bucket

WRITE_KEY_INFO=$(ssh root@$PUBLIC_IP "garage key create write-rfs-key | awk 'NR==2{print \$3}NR==3{print \$3}'")
WRITE_KEY_ID=$(echo $KEY_INFO | awk '{print $1}')
WRITE_KEY_SECRET=$(echo $KEY_INFO | awk '{print $2}')

# We need to generate a key allowing read only permission 
# This allow us to only download different files over the s3 server bucket
# After generating the flist it should be updated to include the read key only

READ_KEY_INFO=$(ssh root@$PUBLIC_IP "garage key create read-rfs-key | awk 'NR==2{print \$3}NR==3{print \$3}'")
READ_KEY_ID=$(echo $KEY_INFO | awk '{print $1}')
READ_KEY_SECRET=$(echo $KEY_INFO | awk '{print $2}')

ssh root@$PUBLIC_IP "
garage bucket allow \
  --read \
  --write \
  --owner \
  flist \
  --key write-rfs-key
garage bucket allow \
  --read \
  --write \
  --owner \
  blobs \
  --key write-rfs-key

garage bucket allow \
  --read \
  flist \
  --key read-rfs-key
garage bucket allow \
  --read \
  blobs \
  --key read-rfs-key
# "
