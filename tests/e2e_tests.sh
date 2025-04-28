#!/bin/bash
set -ex

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Path to the rfs binary
RFS_BIN="../target/release/rfs"

# Test directory
TEST_DIR="/tmp/rfs-e2e-tests"
CACHE_DIR="$TEST_DIR/cache"
SOURCE_DIR="$TEST_DIR/source"
DEST_DIR="$TEST_DIR/destination"
MOUNT_DIR="$TEST_DIR/mount"
FLIST_PATH="$TEST_DIR/test.fl"
DOCKER_FLIST_PATH="$TEST_DIR/docker-test.fl"

# Store URL - using a local directory store for testing
STORE_DIR="$TEST_DIR/store"
STORE_URL="dir://$STORE_DIR"

# Clean up function
cleanup() {
    echo "Cleaning up test directories..."
    # Unmount if mounted
    if mountpoint -q "$MOUNT_DIR"; then
        sudo umount "$MOUNT_DIR"
    fi
    rm -rf "$TEST_DIR"
}

# Setup function
setup() {
    echo "Setting up test directories..."
    mkdir -p "$TEST_DIR" "$CACHE_DIR" "$SOURCE_DIR" "$DEST_DIR" "$MOUNT_DIR" "$STORE_DIR"

    # Create some test files
    echo "Creating test files..."
    echo "This is a test file 1" > "$SOURCE_DIR/file1.txt"
    echo "This is a test file 2" > "$SOURCE_DIR/file2.txt"
    mkdir -p "$SOURCE_DIR/subdir"
    echo "This is a test file in a subdirectory" > "$SOURCE_DIR/subdir/file3.txt"

    # Create a symlink
    ln -s "file1.txt" "$SOURCE_DIR/link_to_file1.txt"

    # Create a smaller file for testing
    dd if=/dev/urandom of="$SOURCE_DIR/random.bin" bs=1M count=1
}

# Function to run a test and report result
run_test() {
    local test_name="$1"
    local test_cmd="$2"

    echo -e "\n${GREEN}Running test: $test_name${NC}"
    echo "Command: $test_cmd"

    if eval "$test_cmd"; then
        echo -e "${GREEN}✓ Test passed: $test_name${NC}"
        return 0
    else
        echo -e "${RED}✗ Test failed: $test_name${NC}"
        return 1
    fi
}

# Test the pack command
test_pack() {
    run_test "Pack command" "$RFS_BIN pack -m $FLIST_PATH -s $STORE_URL $SOURCE_DIR"

    # Verify the flist was created
    if [ ! -f "$FLIST_PATH" ]; then
        echo -e "${RED}Flist file was not created${NC}"
        return 1
    fi

    echo "Flist created successfully at $FLIST_PATH"
    return 0
}

# Test the unpack command
test_unpack() {
    run_test "Unpack command" "$RFS_BIN unpack -m $FLIST_PATH -c $CACHE_DIR $DEST_DIR"

    # Verify files were unpacked correctly
    if ! diff -r "$SOURCE_DIR" "$DEST_DIR"; then
        echo -e "${RED}Unpacked files don't match source files${NC}"
        return 1
    fi

    echo "Files unpacked successfully to $DEST_DIR"
    return 0
}

# Test the mount command (requires sudo)
test_mount() {
    echo -e "\n${GREEN}Running test: Mount command${NC}"
    echo "Command: sudo $RFS_BIN mount -m $FLIST_PATH -c $CACHE_DIR $MOUNT_DIR"

    # Run the mount command in the background
    sudo $RFS_BIN mount -m $FLIST_PATH -c $CACHE_DIR $MOUNT_DIR &
    MOUNT_PID=$!

    # Wait a moment for the mount to complete
    sleep 3

    # Verify the mount point is working
    if ! mountpoint -q "$MOUNT_DIR"; then
        echo -e "${RED}Mount failed${NC}"
        kill $MOUNT_PID 2>/dev/null
        return 1
    fi

    # Check if files are accessible
    if ! ls -la "$MOUNT_DIR"; then
        echo -e "${RED}Cannot list files in mount directory${NC}"
        sudo umount "$MOUNT_DIR" 2>/dev/null
        kill $MOUNT_PID 2>/dev/null
        return 1
    fi

    # Read a file from the mount
    if ! cat "$MOUNT_DIR/file1.txt"; then
        echo -e "${RED}Cannot read file from mount${NC}"
        sudo umount "$MOUNT_DIR" 2>/dev/null
        kill $MOUNT_PID 2>/dev/null
        return 1
    fi

    # Unmount
    sudo umount "$MOUNT_DIR" 2>/dev/null
    kill $MOUNT_PID 2>/dev/null

    echo -e "${GREEN}✓ Test passed: Mount command${NC}"
    echo "Mount test completed successfully"
    return 0
}

# Test the docker command (requires docker)
test_docker() {
    # Check if docker is available
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Docker is not installed, skipping docker test${NC}"
        return 0
    fi

    echo -e "\n${GREEN}Running test: Docker command${NC}"
    echo "Command: $RFS_BIN docker -i alpine:latest -s $STORE_URL"

    # Pull a small test image
    docker pull alpine:latest

    # Convert docker image to flist with a timeout
    timeout 60 $RFS_BIN docker -i alpine:latest -s $STORE_URL &
    DOCKER_PID=$!

    # Wait for the command to complete or timeout
    wait $DOCKER_PID
    RESULT=$?

    if [ $RESULT -eq 124 ]; then
        echo -e "${RED}Docker command timed out${NC}"
        return 1
    elif [ $RESULT -ne 0 ]; then
        echo -e "${RED}Docker command failed with exit code $RESULT${NC}"
        return 1
    fi

    # Verify the flist was created
    if [ ! -f "alpine-latest.fl" ]; then
        echo -e "${RED}Docker flist file was not created${NC}"
        return 1
    fi

    echo -e "${GREEN}✓ Test passed: Docker command${NC}"
    echo "Docker image converted to flist successfully"
    return 0
}

# Test the config command
test_config() {
    # Add a tag
    run_test "Config tag add" "$RFS_BIN config -m $FLIST_PATH tag add -t test=value"

    # List tags
    run_test "Config tag list" "$RFS_BIN config -m $FLIST_PATH tag list"

    # Add a store
    run_test "Config store add" "$RFS_BIN config -m $FLIST_PATH store add -s $STORE_URL"

    # List stores
    run_test "Config store list" "$RFS_BIN config -m $FLIST_PATH store list"

    return 0
}

# Main test function
main() {
    # Register cleanup on exit
    trap cleanup EXIT

    # Setup test environment
    setup

    # Run tests
    test_pack
    test_unpack
    test_config

    # These tests may require sudo
    echo -e "\n${GREEN}The following tests may require sudo:${NC}"
    test_mount
    test_docker

    echo -e "\n${GREEN}All tests completed!${NC}"
}

# Run the main function
main
