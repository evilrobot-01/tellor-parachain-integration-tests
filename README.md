# Tellor Parachain Integration Tests

### Docker
Run tests in Docker container:
- [Install Docker](https://docs.docker.com/get-docker/)
- Allocate at least 8GB of RAM to Docker, 3GB swap space, or you'll get out of memory errors
- Build and run the `tellor-parachain-integration-tests` image defined in `Dockerfile` using the command:
```shell
docker build -t tellor-parachain-integration-tests . && docker run --rm tellor-parachain-integration-tests
```

Run individual tests with output using the command:
```shell
docker run --rm tellor-parachain-integration-tests --test registers --nocapture
```
