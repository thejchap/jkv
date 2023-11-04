# jkv

a mini-clone of a [mini-clone](https://github.com/geohot/minikeyvalue) of [s3](https://aws.amazon.com/s3/)

## dev

```bash
make setup
make dev
```

## usage

```bash
# write to k replicas
curl -i -L -X PUT -d beep localhost:8000/toot
# redirect to replica
curl -i -L localhost:8000/toot
# delete
curl -i -L -X DELETE localhost:8000/toot
```

## leader server

```bash
a mini-clone of a mini-clone of s3

Usage: jkv [OPTIONS] --volumes <VOLUMES>

Options:
  -v, --volumes <VOLUMES>
          comma separated list of volume servers

  -r, --replicas <REPLICAS>
          number of replicas to store

          [default: 3]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## volume server

```bash
PORT=3001 ./volume.sh
```
