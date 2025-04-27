# RFS Tests

This directory contains various tests for the RFS tool.

## Test Types

1. **Unit Tests**: Standard Rust unit tests within the codebase
2. **Integration Tests**: Rust tests that verify specific functionality
3. **End-to-End Tests**: Shell scripts that test the full RFS command-line interface
4. **Performance Tests**: Shell scripts that measure and compare performance

## Running Tests

You can use the provided Makefile to run the tests:

```bash
# Run all tests
make all

# Run specific test types
make unit
make integration
make e2e
make performance

# Clean test artifacts
make clean
```

## Test Files

- `e2e_tests.sh`: End-to-end tests for all RFS commands
- `performance_tests.sh`: Performance tests focusing on parallel upload/download
- `docker_test.rs`: Integration test for the Docker functionality
- `parallel_download_test.rs`: Integration test for parallel download feature
- `Makefile`: Simplifies running the tests

## Requirements

- Rust and Cargo for unit and integration tests
- Bash for shell-based tests
- Docker for Docker-related tests
- Root/sudo access for mount tests

## Notes

- The end-to-end tests create temporary directories in `/tmp/rfs-e2e-tests`
- The performance tests create temporary directories in `/tmp/rfs-performance-tests`
- Some tests require sudo access (mount tests)
- Docker tests will be skipped if Docker is not available
