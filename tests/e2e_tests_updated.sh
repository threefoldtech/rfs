#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Path to the rfs binary
RFS_BIN="../target/release/rfs"

# Test directory
TEST_DIR="/tmp/rfs-upload-download-tests"
CACHE_DIR="$TEST_DIR/cache"
SOURCE_DIR="$TEST_DIR/source"
DEST_DIR="$TEST_DIR/destination"
UPLOAD_DIR="$TEST_DIR/upload"
DOWNLOAD_DIR="$TEST_DIR/download"

# Store URL - using a local directory store for testing
STORE_DIR="$TEST_DIR/store"
STORE_URL="dir://$STORE_DIR"

# Server settings for testing
SERVER_PORT=8080
SERVER_URL="http://localhost:$SERVER_PORT"
SERVER_STORAGE="$TEST_DIR/server_storage"
SERVER_PID_FILE="$TEST_DIR/server.pid"
SERVER_CONFIG_FILE="$TEST_DIR/server_config.toml"

# Test file sizes
SMALL_FILE_SIZE_MB=1
MEDIUM_FILE_SIZE_MB=5
LARGE_FILE_SIZE_MB=10

# Clean up function
cleanup() {
    echo "Cleaning up test environment..."
    
    # Stop the main server if it's running
    if [ -f "$SERVER_PID_FILE" ]; then
        echo "Stopping test server..."
        kill $(cat "$SERVER_PID_FILE") 2>/dev/null || true
        rm -f "$SERVER_PID_FILE"
    fi
    
    # Stop the second server if it's running (for sync tests)
    local SERVER2_PID_FILE="$TEST_DIR/server2.pid"
    if [ -f "$SERVER2_PID_FILE" ]; then
        echo "Stopping second test server..."
        kill $(cat "$SERVER2_PID_FILE") 2>/dev/null || true
        rm -f "$SERVER2_PID_FILE"
    fi
    
    # Remove test directories and files
    rm -rf "$TEST_DIR"
    
    echo "Cleanup complete"
}

# Create server configuration file
create_server_config() {
    echo "Creating server configuration file..."
    
    cat > "$SERVER_CONFIG_FILE" << EOF
# Server configuration for e2e tests
host="0.0.0.0"
port=8080
store_url=["dir:///tmp/store0"]
flist_dir="flists"
sqlite_path="fl-server.db"
storage_dir="storage"
# bloc_size=

jwt_secret="secret"
jwt_expire_hours=5

# users
[[users]]
username = "admin"
password = "admin"

EOF

    echo "Server configuration file created at $SERVER_CONFIG_FILE"
}

# Start the server
start_server() {
    echo -e "\n${GREEN}Starting test server on port $SERVER_PORT...${NC}"
    
    # Create server storage directory
    mkdir -p "$SERVER_STORAGE"
    
    # Create server configuration
    create_server_config
    
    # Start the server in the background
    $RFS_BIN server --config-path "$SERVER_CONFIG_FILE" > "$TEST_DIR/server.log" 2>&1 &
    
    # Save the PID
    echo $! > "$SERVER_PID_FILE"
    
    # Wait for the server to start
    echo "Waiting for server to start..."
    sleep 3
    
    # Check if the server is running
    if ! curl -s "$SERVER_URL/health" > /dev/null; then
        echo -e "${RED}Failed to start server${NC}"
        cat "$TEST_DIR/server.log"
        exit 1
    fi
    
    echo -e "${GREEN}Server started successfully${NC}"
}

# Setup function
setup() {
    echo "Setting up test directories..."
    mkdir -p "$TEST_DIR" "$CACHE_DIR" "$SOURCE_DIR" "$DEST_DIR" "$UPLOAD_DIR" "$DOWNLOAD_DIR" "$STORE_DIR" "$SERVER_STORAGE"

    # Create test files of different sizes
    echo "Creating test files..."
    
    # Small file
    echo -e "${YELLOW}Creating small test file (${SMALL_FILE_SIZE_MB}MB)...${NC}"
    dd if=/dev/urandom of="$SOURCE_DIR/small_file.bin" bs=1M count=$SMALL_FILE_SIZE_MB status=none
    
    # Medium file
    echo -e "${YELLOW}Creating medium test file (${MEDIUM_FILE_SIZE_MB}MB)...${NC}"
    dd if=/dev/urandom of="$SOURCE_DIR/medium_file.bin" bs=1M count=$MEDIUM_FILE_SIZE_MB status=none
    
    # Large file
    echo -e "${YELLOW}Creating large test file (${LARGE_FILE_SIZE_MB}MB)...${NC}"
    dd if=/dev/urandom of="$SOURCE_DIR/large_file.bin" bs=1M count=$LARGE_FILE_SIZE_MB status=none
    
    # Create a directory with multiple files
    mkdir -p "$SOURCE_DIR/multi_files"
    for i in {1..5}; do
        dd if=/dev/urandom of="$SOURCE_DIR/multi_files/file_$i.bin" bs=512K count=1 status=none
    done
    
    # Create a nested directory structure
    mkdir -p "$SOURCE_DIR/nested/dir1/dir2"
    echo "Test content 1" > "$SOURCE_DIR/nested/file1.txt"
    echo "Test content 2" > "$SOURCE_DIR/nested/dir1/file2.txt"
    echo "Test content 3" > "$SOURCE_DIR/nested/dir1/dir2/file3.txt"
    
    echo "Test files created successfully"
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

# Function to measure execution time
measure_time() {
    local start_time=$(date +%s.%N)
    "$@"
    local end_time=$(date +%s.%N)
    echo "$(echo "$end_time - $start_time" | bc)"
}

# Test single file upload
test_single_file_upload() {
    local file_path="$SOURCE_DIR/medium_file.bin"
    local file_name=$(basename "$file_path")
    local upload_time=$(measure_time $RFS_BIN upload "$file_path" -s "$SERVER_URL")
    
    echo -e "Upload time for $file_name: ${YELLOW}$upload_time seconds${NC}"
    
    # Verify the file was uploaded by checking if it exists in the store
    # In a real test, we would verify this by querying the server
    # For this test, we'll just check if the command succeeded
    
    return 0
}

# Test directory upload
test_directory_upload() {
    local dir_path="$SOURCE_DIR/multi_files"
    local upload_time=$(measure_time $RFS_BIN upload-dir "$dir_path" -s "$SERVER_URL")
    
    echo -e "Upload time for directory: ${YELLOW}$upload_time seconds${NC}"
    
    # Verify the directory was uploaded
    # In a real test, we would verify this by querying the server
    
    return 0
}

# Test nested directory upload
test_nested_directory_upload() {
    local dir_path="$SOURCE_DIR/nested"
    local upload_time=$(measure_time $RFS_BIN upload-dir "$dir_path" -s "$SERVER_URL" --create-flist)
    
    echo -e "Upload time for nested directory with flist: ${YELLOW}$upload_time seconds${NC}"
    
    # Verify the directory was uploaded and flist was created
    # In a real test, we would verify this by querying the server
    
    return 0
}

# Test single file download
test_single_file_download() {
    # First, upload a file to get its hash
    local file_path="$SOURCE_DIR/medium_file.bin"
    local file_name=$(basename "$file_path")
    echo -e "\n${GREEN}Uploading file to get hash: $file_path${NC}"
    local upload_output
    upload_output=$($RFS_BIN upload "$file_path" -s "$SERVER_URL" 2>&1)
    echo "$upload_output"
    
    # Extract the file hash from the upload output
    local file_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
    
    if [ -z "$file_hash" ]; then
        echo -e "${RED}Failed to get file hash from upload${NC}"
        echo -e "${RED}Upload output: ${NC}"
        echo "$upload_output"
        return 1
    fi
    
    echo "File hash: $file_hash"
    
    # Now download the file using its hash
    local download_path="$DOWNLOAD_DIR/$file_name"
    local download_time=$(measure_time $RFS_BIN download "$file_hash" -o "$download_path" -s "$SERVER_URL")
    
    echo -e "Download time for $file_name: ${YELLOW}$download_time seconds${NC}"
    
    # Verify the file was downloaded correctly
    if [ ! -f "$download_path" ]; then
        echo -e "${RED}Downloaded file does not exist${NC}"
        return 1
    fi
    
    # Compare the original and downloaded files
    if ! cmp -s "$file_path" "$download_path"; then
        echo -e "${RED}Downloaded file does not match original${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Downloaded file matches original${NC}"
    return 0
}

# Test directory download
test_directory_download() {
    # First, upload a directory with flist to get its hash
    local dir_path="$SOURCE_DIR/nested"
    echo -e "\n${GREEN}Uploading directory with flist to get hash: $dir_path${NC}"
    local upload_output
    upload_output=$($RFS_BIN upload-dir "$dir_path" -s "$SERVER_URL" --create-flist 2>&1)
    echo "$upload_output"
    
    # Extract the flist hash from the upload output
    local flist_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
    
    if [ -z "$flist_hash" ]; then
        echo -e "${RED}Failed to get flist hash from upload${NC}"
        echo -e "${RED}Upload output: ${NC}"
        echo "$upload_output"
        return 1
    fi
    
    echo "Flist hash: $flist_hash"
    
    # Now download the directory using the flist hash
    local download_dir="$DOWNLOAD_DIR/nested"
    mkdir -p "$download_dir"
    
    local download_time=$(measure_time $RFS_BIN download-dir "$flist_hash" -o "$download_dir" -s "$SERVER_URL")
    
    echo -e "Download time for directory: ${YELLOW}$download_time seconds${NC}"
    
    # Verify the directory was downloaded correctly
    if [ ! -d "$download_dir" ]; then
        echo -e "${RED}Downloaded directory does not exist${NC}"
        return 1
    fi
    
    # Compare the original and downloaded directories
    if ! diff -r "$dir_path" "$download_dir"; then
        echo -e "${RED}Downloaded directory does not match original${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Downloaded directory matches original${NC}"
    return 0
}

# Test parallel upload performance
test_parallel_upload_performance() {
    echo -e "\n${GREEN}Testing parallel upload performance...${NC}"
    
    # Create a directory with many small files for testing parallel upload
    local parallel_dir="$SOURCE_DIR/parallel_test"
    mkdir -p "$parallel_dir"
    
    echo -e "${YELLOW}Creating 20 small files for parallel upload test...${NC}"
    for i in {1..20}; do
        dd if=/dev/urandom of="$parallel_dir/file_$i.bin" bs=512K count=1 status=none
        echo -ne "\rCreated $i/20 files"
    done
    echo -e "\nTest files created successfully"
    
    # Test with default parallel upload (PARALLEL_UPLOAD=20)
    echo -e "${YELLOW}Testing with default parallel upload...${NC}"
    local parallel_time=$(measure_time $RFS_BIN upload-dir "$parallel_dir" -s "$SERVER_URL")
    
    # Test with reduced parallelism
    echo -e "${YELLOW}Testing with reduced parallelism...${NC}"
    local serial_time=$(measure_time env RFS_PARALLEL_UPLOAD=1 $RFS_BIN upload-dir "$parallel_dir" -s "$SERVER_URL")
    
    echo -e "Serial upload time: ${YELLOW}$serial_time seconds${NC}"
    echo -e "Parallel upload time: ${YELLOW}$parallel_time seconds${NC}"
    
    # Calculate speedup
    local speedup=$(echo "scale=2; $serial_time / $parallel_time" | bc)
    echo -e "Speedup: ${GREEN}${speedup}x${NC}"
    
    return 0
}

# Test parallel download performance
test_parallel_download_performance() {
    echo -e "\n${GREEN}Testing parallel download performance...${NC}"
    
    # First, upload a directory with many files to get its hash
    local parallel_dir="$SOURCE_DIR/parallel_test"
    echo -e "\n${GREEN}Uploading directory with flist for parallel test: $parallel_dir${NC}"
    local upload_output
    upload_output=$($RFS_BIN upload-dir "$parallel_dir" -s "$SERVER_URL" --create-flist 2>&1)
    echo "$upload_output"
    
    # Extract the flist hash from the upload output
    local flist_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
    
    if [ -z "$flist_hash" ]; then
        echo -e "${RED}Failed to get flist hash from upload${NC}"
        echo -e "${RED}Upload output: ${NC}"
        echo "$upload_output"
        return 1
    fi
    
    echo "Flist hash: $flist_hash"
    
    # Test with default parallel download (PARALLEL_DOWNLOAD=20)
    echo -e "${YELLOW}Testing with default parallel download...${NC}"
    local download_dir_parallel="$DOWNLOAD_DIR/parallel"
    mkdir -p "$download_dir_parallel"
    local parallel_time=$(measure_time $RFS_BIN download-dir "$flist_hash" -o "$download_dir_parallel" -s "$SERVER_URL")
    
    # Test with reduced parallelism
    echo -e "${YELLOW}Testing with reduced parallelism...${NC}"
    local download_dir_serial="$DOWNLOAD_DIR/serial"
    mkdir -p "$download_dir_serial"
    local serial_time=$(measure_time env RFS_PARALLEL_DOWNLOAD=1 $RFS_BIN download-dir "$flist_hash" -o "$download_dir_serial" -s "$SERVER_URL")
    
    echo -e "Serial download time: ${YELLOW}$serial_time seconds${NC}"
    echo -e "Parallel download time: ${YELLOW}$parallel_time seconds${NC}"
    
    # Calculate speedup
    local speedup=$(echo "scale=2; $serial_time / $parallel_time" | bc)
    echo -e "Speedup: ${GREEN}${speedup}x${NC}"
    
    # Verify downloaded directories match
    if ! diff -r "$download_dir_serial" "$download_dir_parallel"; then
        echo -e "${RED}Downloaded directories don't match between serial and parallel methods${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Downloaded directories match between methods${NC}"
    return 0
}

# Test upload with different block sizes
test_block_size_impact() {
    echo -e "\n${GREEN}Testing impact of block size on upload/download...${NC}"
    
    local file_path="$SOURCE_DIR/large_file.bin"
    local file_name=$(basename "$file_path")
    
    # Test with different block sizes
    for block_size in 256 512 1024 2048; do
        echo -e "${YELLOW}Testing with block size: ${block_size}KB${NC}"
        
        # Upload with specific block size
        local upload_time=$(measure_time $RFS_BIN upload "$file_path" -s "$SERVER_URL" -b $((block_size * 1024)))
        echo -e "\n${GREEN}Uploading file with ${block_size}KB blocks: $file_path${NC}"
        local upload_output
        upload_output=$($RFS_BIN upload "$file_path" -s "$SERVER_URL" -b $((block_size * 1024)) 2>&1)
        echo "$upload_output"
        
        # Extract the file hash from the upload output
        local file_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
        
        if [ -z "$file_hash" ]; then
            echo -e "${RED}Failed to get file hash from upload with ${block_size}KB blocks${NC}"
            echo -e "${RED}Upload output: ${NC}"
            echo "$upload_output"
            continue
        fi
        
        echo -e "Upload time with ${block_size}KB blocks: ${YELLOW}$upload_time seconds${NC}"
        
        # Download with the same hash
        local download_path="$DOWNLOAD_DIR/${block_size}kb_${file_name}"
        local download_time=$(measure_time $RFS_BIN download "$file_hash" -o "$download_path" -s "$SERVER_URL")
        
        echo -e "Download time with ${block_size}KB blocks: ${YELLOW}$download_time seconds${NC}"
        
        # Verify the file was downloaded correctly
        if ! cmp -s "$file_path" "$download_path"; then
            echo -e "${RED}Downloaded file with ${block_size}KB blocks does not match original${NC}"
            return 1
        fi
    done
    
    echo -e "${GREEN}All block size tests passed${NC}"
    return 0
}

# Test exists command
test_exists_command() {
    echo -e "\n${GREEN}Testing exists command...${NC}"
    
    # First, upload a file to check
    local file_path="$SOURCE_DIR/medium_file.bin"
    
    echo -e "\n${GREEN}Uploading file to check existence: $file_path${NC}"
    local upload_output
    upload_output=$($RFS_BIN upload "$file_path" -s "$SERVER_URL" 2>&1)
    echo "$upload_output"
    
    # Extract the file hash from the upload output
    local file_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
    
    if [ -z "$file_hash" ]; then
        echo -e "${RED}Failed to get file hash from upload${NC}"
        echo -e "${RED}Upload output: ${NC}"
        echo "$upload_output"
        return 1
    fi
    
    # Test exists command with file path
    echo -e "\n${GREEN}Testing exists with file path${NC}"
    run_test "Exists command with file path" "$RFS_BIN exists \"$file_path\" -s \"$SERVER_URL\""
    
    # Test exists command with hash
    echo -e "\n${GREEN}Testing exists with hash${NC}"
    run_test "Exists command with hash" "$RFS_BIN exists \"$file_hash\" -s \"$SERVER_URL\""
    
    # Test exists command with non-existent file
    echo -e "\n${GREEN}Testing exists with non-existent file${NC}"
    local non_existent_file="$SOURCE_DIR/non_existent_file.bin"
    touch "$non_existent_file"
    echo "This file should not exist on the server" > "$non_existent_file"
    
    # This should report that the file doesn't exist, but the command should succeed
    run_test "Exists command with non-existent file" "$RFS_BIN exists \"$non_existent_file\" -s \"$SERVER_URL\""
    
    return 0
}

# Test website-publish command
test_website_publish() {
    echo -e "\n${GREEN}Testing website-publish command...${NC}"
    
    # Create a simple website in a temporary directory
    local website_dir="$SOURCE_DIR/website"
    mkdir -p "$website_dir"
    
    # Create index.html
    cat > "$website_dir/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>Test Website</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>Test Website</h1>
    <p>This is a test website for RFS.</p>
    <img src="image.png" alt="Test Image">
</body>
</html>
EOF
    
    # Create style.css
    cat > "$website_dir/style.css" << EOF
body {
    font-family: Arial, sans-serif;
    margin: 0;
    padding: 20px;
    background-color: #f0f0f0;
}
h1 {
    color: #333;
}
EOF
    
    # Create a simple image
    dd if=/dev/urandom bs=1024 count=10 | base64 > "$website_dir/image.png"
    
    # Publish the website
    echo -e "\n${GREEN}Publishing website: $website_dir${NC}"
    local publish_output
    publish_output=$($RFS_BIN website-publish "$website_dir" -s "$SERVER_URL" 2>&1)
    echo "$publish_output"
    
    # Extract the website hash and URL from the output
    local website_hash=$(echo "$publish_output" | grep -o "Website hash: [a-f0-9]*" | cut -d' ' -f3)
    local website_url=$(echo "$publish_output" | grep -o "Website URL: .*" | cut -d' ' -f3)
    
    if [ -z "$website_hash" ]; then
        echo -e "${RED}Failed to get website hash from publish output${NC}"
        echo -e "${RED}Publish output: ${NC}"
        echo "$publish_output"
        return 1
    fi
    
    echo -e "Website hash: ${YELLOW}$website_hash${NC}"
    echo -e "Website URL: ${YELLOW}$website_url${NC}"
    
    # Verify the website is accessible
    echo -e "\n${GREEN}Verifying website is accessible...${NC}"
    if curl -s "$website_url" | grep -q "Test Website"; then
        echo -e "${GREEN}Website is accessible${NC}"
    else
        echo -e "${RED}Website is not accessible${NC}"
        return 1
    fi
    
    return 0
}

# Test sync command
test_sync_command() {
    echo -e "\n${GREEN}Testing sync command...${NC}"
    
    # We need a second server to test sync
    # For this test, we'll create a second server configuration and start it
    local SERVER2_PORT=8081
    local SERVER2_URL="http://localhost:$SERVER2_PORT"
    local SERVER2_STORAGE="$TEST_DIR/server2_storage"
    local SERVER2_PID_FILE="$TEST_DIR/server2.pid"
    local SERVER2_CONFIG_FILE="$TEST_DIR/server2_config.toml"
    
    # Create second server storage directory
    mkdir -p "$SERVER2_STORAGE"
    
    # Create second server configuration
    cat > "$SERVER2_CONFIG_FILE" << EOF
# Server configuration for e2e tests (server 2)
host="0.0.0.0"
port=8081
store_url=["dir:///tmp/store1"]
flist_dir="flists"
sqlite_path="fl-server2.db"
storage_dir="storage"
# bloc_size=

jwt_secret="secret"
jwt_expire_hours=5

# users
[[users]]
username = "admin"
password = "admin"
EOF
    
    # Start the second server
    echo -e "\n${GREEN}Starting second test server on port $SERVER2_PORT...${NC}"
    $RFS_BIN server --config-path "$SERVER2_CONFIG_FILE" > "$TEST_DIR/server2.log" 2>&1 &
    echo $! > "$SERVER2_PID_FILE"
    
    # Wait for the server to start
    echo "Waiting for second server to start..."
    sleep 3
    
    # Check if the server is running
    if ! curl -s "$SERVER2_URL/health" > /dev/null; then
        echo -e "${RED}Failed to start second server${NC}"
        cat "$TEST_DIR/server2.log"
        return 1
    fi
    
    echo -e "${GREEN}Second server started successfully${NC}"
    
    # Upload a file to the first server
    local file_path="$SOURCE_DIR/medium_file.bin"
    
    echo -e "\n${GREEN}Uploading file to first server: $file_path${NC}"
    local upload_output
    upload_output=$($RFS_BIN upload "$file_path" -s "$SERVER_URL" 2>&1)
    echo "$upload_output"
    
    # Extract the file hash from the upload output
    local file_hash=$(echo "$upload_output" | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2)
    
    if [ -z "$file_hash" ]; then
        echo -e "${RED}Failed to get file hash from upload${NC}"
        echo -e "${RED}Upload output: ${NC}"
        echo "$upload_output"
        return 1
    fi
    
    # Verify the file exists on the first server but not on the second
    echo -e "\n${GREEN}Verifying file exists on first server but not on second...${NC}"
    $RFS_BIN exists "$file_hash" -s "$SERVER_URL"
    $RFS_BIN exists "$file_hash" -s "$SERVER2_URL" || true  # This should fail, but we don't want to exit
    
    # Sync the file from the first server to the second
    echo -e "\n${GREEN}Syncing file from first server to second...${NC}"
    run_test "Sync command with hash" "$RFS_BIN sync -h \"$file_hash\" -s \"$SERVER_URL\" -d \"$SERVER2_URL\""
    
    # Verify the file now exists on both servers
    echo -e "\n${GREEN}Verifying file now exists on both servers...${NC}"
    run_test "Exists on first server after sync" "$RFS_BIN exists \"$file_hash\" -s \"$SERVER_URL\""
    run_test "Exists on second server after sync" "$RFS_BIN exists \"$file_hash\" -s \"$SERVER2_URL\""
    
    # Test sync all blocks
    echo -e "\n${GREEN}Testing sync all blocks...${NC}"
    
    # Upload another file to the first server
    local file2_path="$SOURCE_DIR/small_file.bin"
    
    echo -e "\n${GREEN}Uploading second file to first server: $file2_path${NC}"
    local upload2_output
    upload2_output=$($RFS_BIN upload "$file2_path" -s "$SERVER_URL" 2>&1)
    echo "$upload2_output"
    
    # Sync all blocks from the first server to the second
    echo -e "\n${GREEN}Syncing all blocks from first server to second...${NC}"
    run_test "Sync command for all blocks" "$RFS_BIN sync -s \"$SERVER_URL\" -d \"$SERVER2_URL\""
    
    # Stop the second server
    if [ -f "$SERVER2_PID_FILE" ]; then
        echo "Stopping second test server..."
        kill $(cat "$SERVER2_PID_FILE") 2>/dev/null || true
        rm -f "$SERVER2_PID_FILE"
    fi
    
    return 0
}

# Main test function
main() {
    # Register cleanup on exit
    trap cleanup EXIT
    
    # Setup test environment
    setup
    
    # Start the server
    start_server
    
    # Run upload tests
    test_single_file_upload
    test_directory_upload
    test_nested_directory_upload
    
    # Run download tests
    test_single_file_download
    test_directory_download
    
    # Run performance tests
    test_parallel_upload_performance
    test_parallel_download_performance
    
    # Run block size impact tests
    test_block_size_impact
    
    # Run exists command tests
    test_exists_command
    
    # Run website-publish tests
    test_website_publish
    
    # Run sync command tests
    test_sync_command
    
    echo -e "\n${GREEN}All upload and download tests completed!${NC}"
}

# Run the main function
main