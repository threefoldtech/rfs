#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Test directory
TEST_DIR="/tmp/rfs-performance-tests"
CACHE_DIR="$TEST_DIR/cache"
SOURCE_DIR="$TEST_DIR/source"
DEST_DIR_SERIAL="$TEST_DIR/destination-serial"
DEST_DIR_PARALLEL="$TEST_DIR/destination-parallel"
FLIST_PATH="$TEST_DIR/perf-test.fl"

# Store URL - using a local directory store for testing
STORE_DIR="$TEST_DIR/store"
STORE_URL="dir://$STORE_DIR"

# Number of files and file size for testing
NUM_FILES=100
FILE_SIZE_MB=1

# Clean up function
cleanup() {
    echo "Cleaning up test directories..."
    rm -rf "$TEST_DIR"
}

# Setup function
setup() {
    echo "Setting up test directories..."
    mkdir -p "$TEST_DIR" "$CACHE_DIR" "$SOURCE_DIR" "$DEST_DIR_SERIAL" "$DEST_DIR_PARALLEL" "$STORE_DIR"
    
    echo -e "${YELLOW}Creating $NUM_FILES test files of ${FILE_SIZE_MB}MB each...${NC}"
    for i in $(seq 1 $NUM_FILES); do
        dd if=/dev/urandom of="$SOURCE_DIR/file_$i.bin" bs=1M count=$FILE_SIZE_MB status=none
        echo -ne "\rCreated $i/$NUM_FILES files"
    done
    echo -e "\nTest files created successfully"
}

# Function to measure execution time
measure_time() {
    local start_time=$(date +%s.%N)
    "$@"
    local end_time=$(date +%s.%N)
    echo "$(echo "$end_time - $start_time" | bc)"
}

# Test pack performance
test_pack_performance() {
    echo -e "\n${GREEN}Testing pack performance...${NC}"
    
    local pack_time=$(measure_time rfs pack -m "$FLIST_PATH" -s "$STORE_URL" "$SOURCE_DIR")
    
    echo -e "Pack time: ${YELLOW}$pack_time seconds${NC}"
    
    # Verify the flist was created
    if [ ! -f "$FLIST_PATH" ]; then
        echo -e "${RED}Flist file was not created${NC}"
        return 1
    fi
    
    echo "Flist created successfully at $FLIST_PATH"
    return 0
}

# Test unpack performance with and without parallel download
test_unpack_performance() {
    echo -e "\n${GREEN}Testing unpack performance...${NC}"
    
    # Clear cache directory to ensure fair comparison
    rm -rf "$CACHE_DIR"
    mkdir -p "$CACHE_DIR"
    
    # Test with parallel download (default)
    echo -e "${YELLOW}Testing with parallel download...${NC}"
    local parallel_time=$(measure_time rfs unpack -m "$FLIST_PATH" -c "$CACHE_DIR" "$DEST_DIR_PARALLEL")
    
    # Clear cache directory again
    rm -rf "$CACHE_DIR"
    mkdir -p "$CACHE_DIR"
    
    # Temporarily disable parallel download by setting PARALLEL_DOWNLOAD to 1
    echo -e "${YELLOW}Testing with serial download...${NC}"
    local serial_time=$(measure_time env RFS_PARALLEL_DOWNLOAD=1 rfs unpack -m "$FLIST_PATH" -c "$CACHE_DIR" "$DEST_DIR_SERIAL")
    
    echo -e "Serial unpack time: ${YELLOW}$serial_time seconds${NC}"
    echo -e "Parallel unpack time: ${YELLOW}$parallel_time seconds${NC}"
    
    # Calculate speedup
    local speedup=$(echo "scale=2; $serial_time / $parallel_time" | bc)
    echo -e "Speedup: ${GREEN}${speedup}x${NC}"
    
    # Verify files were unpacked correctly
    if ! diff -r "$DEST_DIR_SERIAL" "$DEST_DIR_PARALLEL" > /dev/null; then
        echo -e "${RED}Unpacked files don't match between serial and parallel methods${NC}"
        return 1
    fi
    
    echo "Files unpacked successfully and match between methods"
    return 0
}

# Main test function
main() {
    # Register cleanup on exit
    trap cleanup EXIT
    
    # Setup test environment
    setup
    
    # Run performance tests
    test_pack_performance
    test_unpack_performance
    
    echo -e "\n${GREEN}All performance tests completed!${NC}"
}

# Run the main function
main
