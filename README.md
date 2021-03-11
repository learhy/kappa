# kappa - Kentik host probe built with eBPF
Kappa is Kentik's next-gen host and sensor network probe.

## Arguments
kappa has many command line options defined at https://github.com/kentik/kappa/blob/master/src/args.yml

## Build
Building kappa is easiest via docker-compose:

```
docker-compose up
```

This will create a local docker image `kappa:latest` using the `Dockerfile.compose` file. 

## Run

Once you have built the kappa image, run it exporting into a local Grafana/Prometheus instance with the repo https://github.com/kentik/kentik-lite.

```
git clone git@github.com:kentik/kentik-lite.git
cd kentik-lite
docker-compose run --entrypoint fetch ktranslate
chmod a+w grafana/data
chmod a+w prometheus/data
docker-compose -f docker-compose-kappa.yaml
```

Log into grafana at http://localhost:3000 with `admin/therealdeal`

