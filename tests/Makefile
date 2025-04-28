.PHONY: all e2e performance unit integration clean

all: unit integration e2e performance

# Run unit tests
unit:
	@echo "Running unit tests..."
	cargo test --lib

# Run integration tests
integration:
	@echo "Running integration tests..."
	cargo test --test docker_test --test parallel_download_test

# Run end-to-end tests
e2e:
	@echo "Running end-to-end tests..."
	chmod +x e2e_tests.sh
	./e2e_tests.sh

# Run performance tests
performance:
	@echo "Running performance tests..."
	chmod +x performance_tests.sh
	./performance_tests.sh

# Clean test artifacts
clean:
	@echo "Cleaning test artifacts..."
	rm -rf /tmp/rfs-e2e-tests /tmp/rfs-performance-tests
