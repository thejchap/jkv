#!/bin/bash -e
export VOLUME=${1:-/tmp/volume1/}
export TYPE=volume
export PORT=${PORT:-8000}
mkdir -p $VOLUME
chmod 777 $VOLUME
CONF=$(mktemp)
echo "
daemon off;
worker_processes auto;
pcre_jit on;
error_log /dev/stderr;
pid $VOLUME/nginx.pid;
events {
  multi_accept on;
  accept_mutex off;
  worker_connections 4096;
}
http {
  sendfile on;
  sendfile_max_chunk 1024k;
  tcp_nopush on;
  tcp_nodelay on;
  open_file_cache off;
  types_hash_max_size 2048;
  server_tokens off;
  default_type application/octet-stream;
  error_log /dev/stderr error;
  access_log /dev/stdout;
  server {
    listen $PORT default_server backlog=4096;
    location / {
      root $VOLUME;
      disable_symlinks off;
      client_body_temp_path $VOLUME/body_temp;
      client_max_body_size 0;
      dav_methods PUT DELETE;
      dav_access group:rw all:r;
      create_full_put_path on;
      autoindex on;
      autoindex_format json;
    }
  }
}
" > $CONF
echo "starting nginx on $PORT"
nginx -c $CONF -p $VOLUME/tmp
