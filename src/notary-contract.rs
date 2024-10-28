#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env,
    Symbol, Vec, vec, Map, String, panic_with_error, log, 
    xdr::{ScErrorType, ScErrorCode}, // Add this line
};

/// Error codes for the contract
#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum NotaryError {
    AlreadyExists = 1,
    NotFound = 2,
    Unauthorized = 3,
    InvalidVersion = 4,
    InvalidStatus = 5,
    InvalidSignature = 6,
    ExpiredClaim = 7,
    MissingIdentityClaim = 8,
    InvalidAuthority = 9,
    InvalidInput = 10,
    InvalidState = 11,
    OperationFailed = 12,
}

impl From<soroban_sdk::Error> for NotaryError {
    fn from(_error: soroban_sdk::Error) -> Self {
        NotaryError::OperationFailed
    }
}

impl From<NotaryError> for soroban_sdk::Error {
    fn from(error: NotaryError) -> Self {
        // Map your custom errors to Soroban error codes
        let (type_, code) = match error {
            NotaryError::AlreadyExists => (ScErrorType::Contract, ScErrorCode::ExistingValue),
            NotaryError::NotFound => (ScErrorType::Contract, ScErrorCode::MissingValue),
            NotaryError::Unauthorized => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidVersion => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidStatus => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidSignature => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::ExpiredClaim => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::MissingIdentityClaim => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidAuthority => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidInput => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidState => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::OperationFailed => (ScErrorType::Contract, ScErrorCode::InternalError),
        };

        soroban_sdk::Error::from_type_and_code(type_, code)
    }
}
impl From<&NotaryError> for soroban_sdk::Error {
    fn from(error: &NotaryError) -> Self {
        let (type_, code) = match error {
            NotaryError::AlreadyExists => (ScErrorType::Contract, ScErrorCode::ExistingValue),
            NotaryError::NotFound => (ScErrorType::Contract, ScErrorCode::MissingValue),
            NotaryError::Unauthorized => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidVersion => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidStatus => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidSignature => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::ExpiredClaim => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::MissingIdentityClaim => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidAuthority => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidInput => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::InvalidState => (ScErrorType::Contract, ScErrorCode::InternalError),
            NotaryError::OperationFailed => (ScErrorType::Contract, ScErrorCode::InternalError),
        };
        
        soroban_sdk::Error::from_type_and_code(type_, code)
    }
}
/// Storage identifiers
const ADMIN: Symbol = symbol_short!("ADMIN");
const STATE: Symbol = symbol_short!("STATE");
const DOCS: Symbol = symbol_short!("DOCS");
const AUTH: Symbol = symbol_short!("AUTH");

/// Document status
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DocumentStatus {
    Pending,
    Active,
    Revoked,
    Expired,
}

/// Version status
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VersionStatus {
    Draft,
    PendingApproval,
    Approved,
    Rejected,
    Superseded,
}

/// Identity claim structure
#[derive(Clone, Debug)]
#[contracttype]
pub struct IdentityClaim {
    authority: Address,
    claim_type: Symbol,
    claim_value: BytesN<32>,
    signature: BytesN<64>,
    issued_at: u64,
    expires_at: u64,
    metadata: Map<Symbol, String>,
}

/// Signature structure
#[derive(Clone, Debug)]
#[contracttype]
pub struct Signature {
    signer: Address,
    timestamp: u64,
    signature_data: BytesN<64>,
    claim_reference: BytesN<32>,
}

/// Document version structure
#[derive(Clone, Debug)]
#[contracttype]
pub struct DocumentVersion {
    hash: BytesN<32>,
    parent_hash: Option<BytesN<32>>,
    title: String,
    status: VersionStatus,
    creator: Address,
    created_at: u64,
    updated_at: u64,
    signatures: Vec<Signature>,
    required_signers: Vec<Address>,
    metadata: Map<Symbol, String>,
}

/// Document structure
#[derive(Clone, Debug)]
#[contracttype]
pub struct Document {
    hash: BytesN<32>,
    status: DocumentStatus,
    owner: Address,
    created_at: u64,
    updated_at: u64,
    current_version: u32,
    versions: Vec<DocumentVersion>,
    authorized_signers: Vec<Address>,
    metadata: Map<Symbol, String>,
}

/// Contract storage structure
#[derive(Clone, Debug)]
#[contracttype]
pub struct NotaryState {
    admin: Address,
    documents: Map<BytesN<32>, Document>,
    user_documents: Map<Address, Vec<BytesN<32>>>,
    authorities: Vec<Address>,
    claims: Map<Address, Vec<IdentityClaim>>,
    settings: Map<Symbol, String>,
}

/// Event types for logging
#[contracttype]
pub enum NotaryEvent {
    DocumentCreated(BytesN<32>),
    VersionAdded(BytesN<32>),
    DocumentSigned(BytesN<32>),
    StatusChanged(BytesN<32>, DocumentStatus),
    ClaimAdded(Address),
    AuthorityAdded(Address),
}

#[contract]
pub struct NotaryContract;

#[contractimpl]
impl NotaryContract {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address) -> Result<(), NotaryError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(NotaryError::AlreadyExists);
        }

        let state = NotaryState {
            admin: admin.clone(),
            documents: Map::new(&env),
            user_documents: Map::new(&env),
            authorities: Vec::new(&env),
            claims: Map::new(&env),
            settings: Map::new(&env),
        };

        env.storage().instance().set(&STATE, &state);
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }

    /// Create a new document
    pub fn create_document(
    env: Env,
    hash: BytesN<32>,
    title: String,
    signers: Vec<Address>,
    metadata: Map<Symbol, String>,
) -> Result<(), NotaryError> {
    let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

    // Clone hash upfront since we'll need it multiple times
    let hash_clone = hash.clone();

    // Check if document already exists
    if state.documents.contains_key(hash_clone.clone()) {
        return Err(NotaryError::AlreadyExists);
    }

    // Create initial version
    let version = DocumentVersion {
        hash: hash_clone.clone(),
        parent_hash: None,
        title: title.clone(),
        status: VersionStatus::PendingApproval,
        creator: env.current_contract_address(),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        signatures: Vec::new(&env),
        required_signers: signers.clone(),
        metadata: metadata.clone(),
    };

    // Create document
    let document = Document {
        hash: hash_clone.clone(),
        status: DocumentStatus::Pending,
        owner: env.current_contract_address(),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        current_version: 0,
        versions: vec![&env, version],
        authorized_signers: signers,
        metadata,
    };

    // Update state
    state.documents.set(hash, document);

    // Update user documents
    let mut user_docs = state.user_documents.get(env.current_contract_address())
        .unwrap_or(Vec::new(&env));
    user_docs.push_back(hash_clone.clone());
    state.user_documents.set(env.current_contract_address(), user_docs);

    // Save state and emit event
    env.storage().instance().set(&STATE, &state);
    env.events().publish((DOCS,), NotaryEvent::DocumentCreated(hash_clone));

    Ok(())
}

    /// Add new version to document
pub fn add_version(
    env: Env,
    document_hash: BytesN<32>,
    version_hash: BytesN<32>,
    title: String,
    metadata: Map<Symbol, String>,
) -> Result<(), NotaryError> {
    let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

    // Clone both hashes upfront since we'll need them multiple times
    let doc_hash_clone = document_hash.clone();
    let ver_hash_clone = version_hash.clone();

    // Get existing document, using clone for get operation
    let mut document = state.documents.get(document_hash)
        .ok_or(NotaryError::NotFound)?;

    // Verify caller is owner or authorized signer
    if !Self::is_authorized(&document, env.current_contract_address()) {
        return Err(NotaryError::Unauthorized);
    }

    // Create new version
    let version = DocumentVersion {
        hash: ver_hash_clone.clone(),
        parent_hash: Some(doc_hash_clone.clone()),
        title,
        status: VersionStatus::Draft,
        creator: env.current_contract_address(),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        signatures: Vec::new(&env),
        required_signers: document.authorized_signers.clone(),
        metadata,
    };

    // Add version and update document
    document.versions.push_back(version);
    document.current_version = (document.versions.len() - 1) as u32;
    document.updated_at = env.ledger().timestamp();

    // Update state with original document_hash
    state.documents.set(doc_hash_clone, document);
    env.storage().instance().set(&STATE, &state);

    // Emit event using cloned version hash
    env.events().publish((DOCS,), NotaryEvent::VersionAdded(ver_hash_clone));

    Ok(())
}

    /// Sign a document version
    pub fn sign_document(
        env: Env,
        document_hash: BytesN<32>,
        signature: Signature,
    ) -> Result<(), NotaryError> {
        let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

        // Get document
        let mut document = state.documents.get(document_hash.clone())
            .ok_or(NotaryError::NotFound)?;

        // Verify signer is authorized
        if !document.authorized_signers.contains(&env.current_contract_address()) {
            return Err(NotaryError::Unauthorized);
        }

        // Get current version
        let current_version_idx = document.current_version as usize;
        let mut current_version = document.versions.get(current_version_idx as u32).unwrap();
        
        // Verify not already signed by this signer
        if current_version.signatures.iter().any(|s| s.signer == signature.signer) {
            return Err(NotaryError::AlreadyExists);
        }

        // Add signature
        current_version.signatures.push_back(signature.clone());
        current_version.updated_at = env.ledger().timestamp();

        // Check if all required signatures are present
        if current_version.signatures.len() == current_version.required_signers.len() {
            current_version.status = VersionStatus::Approved;
            document.status = DocumentStatus::Active;
        }

        // Update document
        document.versions.set(current_version_idx as u32, current_version);
        document.updated_at = env.ledger().timestamp();

        // Update state
        state.documents.set(document_hash.clone(), document);
        env.storage().instance().set(&STATE, &state);

        // Emit event
        env.events().publish((DOCS,), NotaryEvent::DocumentSigned(document_hash));

        Ok(())
    }

    /// Register a certification authority
    pub fn register_authority(env: Env, authority: Address) -> Result<(), NotaryError> {
        let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

        // Verify caller is admin
        if env.current_contract_address() != state.admin {
            return Err(NotaryError::Unauthorized);
        }

        // Add authority if not already registered
        if !state.authorities.contains(&authority) {
            state.authorities.push_back(authority.clone());
            
            // Update state and emit event
            env.storage().instance().set(&STATE, &state);
            env.events().publish((AUTH,), NotaryEvent::AuthorityAdded(authority));
        }

        Ok(())
    }

    /// Add identity claim
    pub fn add_claim(
        env: Env,
        user: Address,
        claim: IdentityClaim,
    ) -> Result<(), NotaryError> {
        let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

        // Verify caller is authorized authority
        if !state.authorities.contains(&env.current_contract_address()) {
            return Err(NotaryError::InvalidAuthority);
        }

        // Verify claim expiration
        if claim.expires_at <= env.ledger().timestamp() {
            return Err(NotaryError::ExpiredClaim);
        }

        // Add claim to user's claims
        let mut user_claims = state.claims.get(user.clone())
            .unwrap_or(Vec::new(&env));
        user_claims.push_back(claim.clone());
        state.claims.set(user.clone(), user_claims);

        // Update state and emit event
        env.storage().instance().set(&STATE, &state);
        env.events().publish((AUTH,), NotaryEvent::ClaimAdded(user));

        Ok(())
    }

    /// Verify document
    pub fn verify_document(env: Env, document_hash: BytesN<32>) -> Result<Document, NotaryError> {
        let state: NotaryState = env.storage().instance().get(&STATE).unwrap();
        
        state.documents.get(document_hash)
            .ok_or(NotaryError::NotFound)
    }

    /// Helper: Check if address is authorized for document
    fn is_authorized(document: &Document, address: Address) -> bool {
        address == document.owner || document.authorized_signers.contains(address)
    }

    /// Get user's documents
    pub fn get_user_documents(env: Env, user: Address) -> Result<Vec<BytesN<32>>, NotaryError> {
        let state: NotaryState = env.storage().instance().get(&STATE).unwrap();
        
        Ok(state.user_documents.get(user)
            .unwrap_or(Vec::new(&env)).clone())
    }

    /// Update document status
    pub fn update_status(
        env: Env,
        document_hash: BytesN<32>,
        new_status: DocumentStatus,
    ) -> Result<(), NotaryError> {
        let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

        // Get document
        let mut document = state.documents.get(document_hash.clone())
            .ok_or(NotaryError::NotFound)?;

        // Verify caller is owner
        if env.current_contract_address() != document.owner {
            return Err(NotaryError::Unauthorized);
        }

        // Update status
        document.status = new_status.clone();
        document.updated_at = env.ledger().timestamp();

        // Update state
        state.documents.set(document_hash.clone(), document);
        env.storage().instance().set(&STATE, &state);

        // Emit event
        env.events().publish((DOCS,), NotaryEvent::StatusChanged(document_hash, new_status));

        Ok(())
    }

    /// Get contract configuration
    pub fn get_config(env: Env, key: Symbol) -> Result<String, NotaryError> {
        let state: NotaryState = env.storage().instance().get(&STATE).unwrap();
        
        state.settings.get(key)
            .ok_or(NotaryError::NotFound)
    }

    /// Update contract configuration
    pub fn update_config(
        env: Env,
        key: Symbol,
        value: String,
    ) -> Result<(), NotaryError> {
        let mut state: NotaryState = env.storage().instance().get(&STATE).unwrap();

        // Verify caller is admin
        if env.current_contract_address() != state.admin {
            return Err(NotaryError::Unauthorized);
        }

        // Update setting
        state.settings.set(key, value);
        env.storage().instance().set(&STATE, &state);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction},
        vec, map, Vec,
    };

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        let admin = Address::random(&env);
        assert!(client.initialize(&admin).is_ok());
    }
#[test]
    fn test_document_lifecycle() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Create document
        let hash = BytesN::random(&env);
        let title = String::from_slice(&env, "Test Document");
        let signers = vec![&env, Address::random(&env)];
        let metadata = Map::new(&env);
        
        assert!(client.create_document(&hash, &title, &signers, &metadata).is_ok());

        // Test version creation
        let version_hash = BytesN::random(&env);
        let version_title = String::from_slice(&env, "Version 2");
        assert!(client.add_version(&hash, &version_hash, &version_title, &metadata).is_ok());

        // Test document signing
        let signature = Signature {
            signer: signers.get(0).unwrap(),
            timestamp: env.ledger().timestamp(),
            signature_data: BytesN::random(&env),
            claim_reference: BytesN::random(&env),
        };
        assert!(client.sign_document(&hash, &signature).is_ok());

        // Verify document
        let document = client.verify_document(&hash).unwrap();
        assert_eq!(document.status, DocumentStatus::Active);
        assert_eq!(document.current_version, 1);
    }

    #[test]
    fn test_authority_management() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Register authority
        let authority = Address::random(&env);
        env.set_source_account(&admin);
        assert!(client.register_authority(&authority).is_ok());

        // Add claim
        let user = Address::random(&env);
        let claim = IdentityClaim {
            authority: authority.clone(),
            claim_type: symbol_short!("ID"),
            claim_value: BytesN::random(&env),
            signature: BytesN::random(&env),
            issued_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
            metadata: Map::new(&env),
        };
        
        env.set_source_account(&authority);
        assert!(client.add_claim(&user, &claim).is_ok());
    }

    #[test]
    fn test_document_status_update() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Create document
        let hash = BytesN::random(&env);
        let title = String::from_slice(&env, "Test Document");
        let signers = vec![&env, Address::random(&env)];
        let metadata = Map::new(&env);
        
        client.create_document(&hash, &title, &signers, &metadata).unwrap();

        // Update status
        assert!(client.update_status(&hash, &DocumentStatus::Revoked).is_ok());

        // Verify status
        let document = client.verify_document(&hash).unwrap();
        assert_eq!(document.status, DocumentStatus::Revoked);
    }

    #[test]
    fn test_configuration_management() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Update config
        let key = symbol_short!("MAX_SIGNERS");
        let value = String::from_slice(&env, "10");
        env.set_source_account(&admin);
        assert!(client.update_config(&key, &value).is_ok());

        // Verify config
        let result = client.get_config(&key).unwrap();
        assert_eq!(result, value);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_actions() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Try to register authority from non-admin account
        let unauthorized = Address::random(&env);
        env.set_source_account(&unauthorized);
        
        let authority = Address::random(&env);
        client.register_authority(&authority).unwrap();
    }

    #[test]
    fn test_multiple_signatures() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Create document with multiple signers
        let hash = BytesN::random(&env);
        let title = String::from_slice(&env, "Multi-Sig Document");
        let signers = vec![
            &env,
            Address::random(&env),
            Address::random(&env),
            Address::random(&env)
        ];
        let metadata = Map::new(&env);
        
        client.create_document(&hash, &title, &signers, &metadata).unwrap();

        // Add signatures
        for signer in signers.iter() {
            let signature = Signature {
                signer: signer.clone(),
                timestamp: env.ledger().timestamp(),
                signature_data: BytesN::random(&env),
                claim_reference: BytesN::random(&env),
            };
            
            env.set_source_account(signer);
            assert!(client.sign_document(&hash, &signature).is_ok());
        }

        // Verify all signatures are present
        let document = client.verify_document(&hash).unwrap();
        let current_version = document.versions.get(document.current_version as usize).unwrap();
        assert_eq!(current_version.signatures.len(), signers.len());
        assert_eq!(document.status, DocumentStatus::Active);
    }

    #[test]
    fn test_expired_claims() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize
        let admin = Address::random(&env);
        client.initialize(&admin).unwrap();

        // Register authority
        let authority = Address::random(&env);
        env.set_source_account(&admin);
        client.register_authority(&authority).unwrap();

        // Add expired claim
        let user = Address::random(&env);
        let claim = IdentityClaim {
            authority: authority.clone(),
            claim_type: symbol_short!("ID"),
            claim_value: BytesN::random(&env),
            signature: BytesN::random(&env),
            issued_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() - 1, // Expired
            metadata: Map::new(&env),
        };
        
        env.set_source_account(&authority);
        assert!(client.add_claim(&user, &claim).is_err());
    }
}
