#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Contract configuration
CONTRACT_ID="CD43B7CVH67FUDLGN3ID4KIH4UWXHTHOJ7QETV56XCRYLPYFARIKJU74"
NETWORK="testnet"
SOURCE_ACCOUNT="docchain"

# Function to print colored output
log() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
}

warning() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

# Generate test data
generate_test_data() {
    log "Generating test data..."
    
    # Generate document hashes
    DOCUMENT_HASH=$(openssl rand -hex 32)
    VERSION_HASH=$(openssl rand -hex 32)
    CLAIM_VALUE=$(openssl rand -hex 32)
    SIGNATURE_DATA=$(openssl rand -hex 64)
    CLAIM_REF=$(openssl rand -hex 32)
    
    # Generate timestamps
    CURRENT_TIME=$(date +%s)
    EXPIRE_TIME=$((CURRENT_TIME + 31536000)) # One year from now
    
    # Test addresses (using predefined ones for reproducibility)
    ADMIN_ADDRESS="GDQOE2XBKGQRIUCDZF6XHXBQFWEYOQVZD3GSSUG4NIXBVXDV4Y7MDRYK"
    SIGNER1_ADDRESS="GBXGQJWVLWOYHFLVTKWV5FGHA3LNYY2JQKM7OAJAUEQFU6LPCSEFVXON"
    SIGNER2_ADDRESS="GD7AHJHCDSQI6LVMEJEE2FTNCA2LJQZ4R64GUI3PWANSVEO4GEOWB636"
    
    log "Test data generated successfully:"
    echo "Document Hash: $DOCUMENT_HASH"
    echo "Version Hash: $VERSION_HASH"
    echo "Admin Address: $ADMIN_ADDRESS"
}

# Function to execute contract invocation
invoke_contract() {
    local function_name=$1
    local args=$2
    
    log "Invoking $function_name..."
    command="stellar contract invoke \
        --id $CONTRACT_ID \
        --source $SOURCE_ACCOUNT \
        --network $NETWORK \
        -- $function_name $args"
    
    echo "Executing: $command"
    eval $command
    
    if [ $? -eq 0 ]; then
        log "Successfully executed $function_name"
    else
        error "Failed to execute $function_name"
        exit 1
    fi
    
    # Add slight delay between transactions
    sleep 5
}

# Main test sequence
run_tests() {
    log "Starting contract tests..."
    
    # 1. Initialize contract
    invoke_contract "initialize" "--admin $ADMIN_ADDRESS"
    
    # 2. Create document
    SIGNERS="[\"$SIGNER1_ADDRESS\", \"$SIGNER2_ADDRESS\"]"
    METADATA="{\"department\": \"legal\", \"type\": \"contract\", \"version\": \"1.0\"}"
    invoke_contract "create_document" "--hash $DOCUMENT_HASH --title \"Test Document\" --signers '$SIGNERS' --metadata '$METADATA'"
    
    # 3. Add version
    NEW_METADATA="{\"department\": \"legal\", \"type\": \"contract\", \"version\": \"1.1\"}"
    invoke_contract "add_version" "--document_hash $DOCUMENT_HASH --version_hash $VERSION_HASH --title \"Test Document V2\" --metadata '$NEW_METADATA'"
    
    # 4. Sign document
    SIGNATURE="{\"signer\": \"$SIGNER1_ADDRESS\", \"timestamp\": $CURRENT_TIME, \"signature_data\": \"$SIGNATURE_DATA\", \"claim_reference\": \"$CLAIM_REF\"}"
    invoke_contract "sign_document" "--document_hash $DOCUMENT_HASH --signature '$SIGNATURE'"
    
    # 5. Register authority
    invoke_contract "register_authority" "--authority $SIGNER1_ADDRESS"
    
    # 6. Add claim
    CLAIM="{\"authority\": \"$ADMIN_ADDRESS\", \"claim_type\": \"ID\", \"claim_value\": \"$CLAIM_VALUE\", \"signature\": \"$SIGNATURE_DATA\", \"issued_at\": $CURRENT_TIME, \"expires_at\": $EXPIRE_TIME, \"metadata\": {\"verification_level\": \"high\"}}"
    invoke_contract "add_claim" "--user $SIGNER1_ADDRESS --claim '$CLAIM'"
    
    # 7. Get user documents
    invoke_contract "get_user_documents" "--user $SIGNER1_ADDRESS"
    
    # 8. Verify document
    invoke_contract "verify_document" "--document_hash $DOCUMENT_HASH"
    
    # 9. Update status
    invoke_contract "update_status" "--document_hash $DOCUMENT_HASH --new_status \"Active\""
    
    # 10. Update config
    invoke_contract "update_config" "--key \"MAX_SIGNERS\" --value \"5\""
    
    log "All tests completed successfully!"
}

# Cleanup function
cleanup() {
    log "Cleaning up..."
    # Add any cleanup steps here if needed
}

# Main execution
main() {
    # Check if contract ID is set
    if [ "$CONTRACT_ID" = "<YOUR_CONTRACT_ID>" ]; then
        error "Please set your CONTRACT_ID before running the script"
        exit 1
    fi
    
    # Generate test data
    generate_test_data
    
    # Run tests
    run_tests
    
    # Cleanup
    cleanup
}

# Handle script interruption
trap cleanup EXIT

# Run main function
main
