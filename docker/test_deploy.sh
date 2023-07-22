#!/usr/bin/env bash
set -e

export COMPOSE_DOCKER_CLI_BUILD=1
export DOCKER_BUILDKIT=1
export CARGO_BUILD_FEATURES=prometheus-metrics # Enable prometheus

# Rebuilding dev docker
pushd ..
docker build . -f docker/Dockerfile.scheduler --build-arg RUST_RELEASE_MODE=release -t "alexandrupero/lemmy-scheduler:$(git describe --tag --always)-$(git rev-parse --short HEAD)-arm64" --push
docker build . -f docker/Dockerfile.api --build-arg RUST_RELEASE_MODE=release -t "alexandrupero/lemmy-api:$(git describe --tag --always)-$(git rev-parse --short HEAD)-arm64" --push
docker build . -f docker/Dockerfile.federation --build-arg RUST_RELEASE_MODE=release -t "alexandrupero/lemmy-federation:$(git describe --tag --always)-$(git rev-parse --short HEAD)-arm64" --push

# Run the playbook
# pushd ../../../lemmy-ansible
# ansible-playbook -i test playbooks/site.yml
# popd
