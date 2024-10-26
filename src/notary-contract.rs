#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, BytesN, Env,
    Symbol, Vec, Map, String, panic_with_error,
};

// Error codes for the contract
#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum NotaryError {
    AlreadyNotarized = 1,
    NotFound = 2,
    Unauthorized = 3,
    InvalidInput = 4,
    ExpiredTimestamp = 5,
    InvalidStatus = 6,
}

// Status enum for notarization records
#[derive(Clone, Debug)]
#[contracttype]
pub enum NotarizationStatus {
    Pending,
    Completed,
    Revoked,
    Expired,
}

// Struct to store notarization data
#[derive(Clone, Debug)]
#[contracttype]
pub struct NotarizationRecord {
    document_hash: BytesN<32>,
    notary: Address,
    timestamp: u64,
    expiration: u64,
    status: NotarizationStatus,
    metadata: Map<Symbol, String>,
}

// Struct for verification response
#[derive(Clone, Debug)]
#[contracttype]
pub struct VerificationResponse {
    is_valid: bool,
    timestamp: u64,
    notary: Address,
    status: NotarizationStatus,
    metadata: Map<Symbol, String>,
}

// Event types for logging
#[contracttype]
pub enum NotaryEvent {
    Notarized(NotarizationRecord),
    Verified(VerificationResponse),
    Revoked(BytesN<32>),
    StatusUpdated(BytesN<32>, NotarizationStatus),
}

// Contract storage keys
const ADMIN: Symbol = symbol_short!("ADMIN");
const NOTARIES: Symbol = symbol_short!("NOTARIES");
const DOCUMENTS: Symbol = symbol_short!("DOCS");
const FEE: Symbol = symbol_short!("FEE");

pub trait NotaryTrait {
    // Admin functions
    fn initialize(env: Env, admin: Address, fee: i128) -> Result<(), NotaryError>;
    fn add_notary(env: Env, notary: Address) -> Result<(), NotaryError>;
    fn remove_notary(env: Env, notary: Address) -> Result<(), NotaryError>;
    fn set_fee(env: Env, new_fee: i128) -> Result<(), NotaryError>;
    
    // Core notary functions
    fn notarize(
        env: Env,
        document_hash: BytesN<32>,
        expiration: u64,
        metadata: Map<Symbol, String>
    ) -> Result<(), NotaryError>;
    
    fn verify(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<VerificationResponse, NotaryError>;
    
    fn revoke(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<(), NotaryError>;
    
    fn update_status(
        env: Env,
        document_hash: BytesN<32>,
        new_status: NotarizationStatus
    ) -> Result<(), NotaryError>;
    
    // Query functions
    fn get_notarization(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<NotarizationRecord, NotaryError>;
    
    fn get_notary_documents(
        env: Env,
        notary: Address
    ) -> Result<Vec<BytesN<32>>, NotaryError>;
    
    fn is_notary(env: Env, address: Address) -> bool;
}

#[contract]
pub struct NotaryContract;

#[contractimpl]
impl NotaryTrait for NotaryContract {
    fn initialize(env: Env, admin: Address, fee: i128) -> Result<(), NotaryError> {
        if env.storage().has(&ADMIN) {
            panic_with_error!("Contract already initialized");
        }
        
        env.storage().set(&ADMIN, &admin);
        env.storage().set(&FEE, &fee);
        
        // Initialize empty notaries list
        let notaries: Vec<Address> = Vec::new(&env);
        env.storage().set(&NOTARIES, &notaries);
        
        Ok(())
    }

    fn add_notary(env: Env, notary: Address) -> Result<(), NotaryError> {
        let admin: Address = env.storage().get(&ADMIN).unwrap();
        if env.invoker() != admin {
            return Err(NotaryError::Unauthorized);
        }
        
        let mut notaries: Vec<Address> = env.storage().get(&NOTARIES).unwrap();
        if !notaries.contains(&notary) {
            notaries.push_back(notary.clone());
            env.storage().set(&NOTARIES, &notaries);
        }
        
        Ok(())
    }

    fn remove_notary(env: Env, notary: Address) -> Result<(), NotaryError> {
        let admin: Address = env.storage().get(&ADMIN).unwrap();
        if env.invoker() != admin {
            return Err(NotaryError::Unauthorized);
        }
        
        let mut notaries: Vec<Address> = env.storage().get(&NOTARIES).unwrap();
        let index = notaries.first_index_of(notary.clone());
        if let Some(i) = index {
            notaries.remove(i);
            env.storage().set(&NOTARIES, &notaries);
        }
        
        Ok(())
    }

    fn set_fee(env: Env, new_fee: i128) -> Result<(), NotaryError> {
        let admin: Address = env.storage().get(&ADMIN).unwrap();
        if env.invoker() != admin {
            return Err(NotaryError::Unauthorized);
        }
        
        env.storage().set(&FEE, &new_fee);
        Ok(())
    }

    fn notarize(
        env: Env,
        document_hash: BytesN<32>,
        expiration: u64,
        metadata: Map<Symbol, String>
    ) -> Result<(), NotaryError> {
        // Check if caller is an authorized notary
        if !Self::is_notary(env.clone(), env.invoker()) {
            return Err(NotaryError::Unauthorized);
        }
        
        // Check if document is already notarized
        if env.storage().has(&document_hash) {
            return Err(NotaryError::AlreadyNotarized);
        }
        
        // Validate expiration timestamp
        if expiration <= env.ledger().timestamp() {
            return Err(NotaryError::ExpiredTimestamp);
        }
        
        // Create notarization record
        let record = NotarizationRecord {
            document_hash: document_hash.clone(),
            notary: env.invoker(),
            timestamp: env.ledger().timestamp(),
            expiration,
            status: NotarizationStatus::Completed,
            metadata,
        };
        
        // Store the record
        env.storage().set(&document_hash, &record);
        
        // Emit notarization event
        env.events().publish((Symbol::new(&env, "notarize"), record.clone()));
        
        Ok(())
    }

    fn verify(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<VerificationResponse, NotaryError> {
        // Get notarization record
        let record: NotarizationRecord = env.storage().get(&document_hash)
            .ok_or(NotaryError::NotFound)?;
        
        // Check if notarization is expired
        let is_expired = env.ledger().timestamp() > record.expiration;
        let status = if is_expired {
            NotarizationStatus::Expired
        } else {
            record.status
        };
        
        let response = VerificationResponse {
            is_valid: matches!(status, NotarizationStatus::Completed),
            timestamp: record.timestamp,
            notary: record.notary,
            status,
            metadata: record.metadata,
        };
        
        // Emit verification event
        env.events().publish((Symbol::new(&env, "verify"), response.clone()));
        
        Ok(response)
    }

    fn revoke(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<(), NotaryError> {
        // Get notarization record
        let mut record: NotarizationRecord = env.storage().get(&document_hash)
            .ok_or(NotaryError::NotFound)?;
        
        // Check if caller is the original notary
        if env.invoker() != record.notary {
            return Err(NotaryError::Unauthorized);
        }
        
        // Update status to revoked
        record.status = NotarizationStatus::Revoked;
        env.storage().set(&document_hash, &record);
        
        // Emit revocation event
        env.events().publish((Symbol::new(&env, "revoke"), document_hash));
        
        Ok(())
    }

    fn update_status(
        env: Env,
        document_hash: BytesN<32>,
        new_status: NotarizationStatus
    ) -> Result<(), NotaryError> {
        // Get notarization record
        let mut record: NotarizationRecord = env.storage().get(&document_hash)
            .ok_or(NotaryError::NotFound)?;
        
        // Check if caller is the original notary
        if env.invoker() != record.notary {
            return Err(NotaryError::Unauthorized);
        }
        
        // Update status
        record.status = new_status;
        env.storage().set(&document_hash, &record);
        
        // Emit status update event
        env.events().publish((Symbol::new(&env, "status_update"), (document_hash, new_status)));
        
        Ok(())
    }

    fn get_notarization(
        env: Env,
        document_hash: BytesN<32>
    ) -> Result<NotarizationRecord, NotaryError> {
        env.storage().get(&document_hash)
            .ok_or(NotaryError::NotFound)
    }

    fn get_notary_documents(
        env: Env,
        notary: Address
    ) -> Result<Vec<BytesN<32>>, NotaryError> {
        // Create vector to store document hashes
        let mut documents = Vec::new(&env);
        
        // Iterate through all documents and filter by notary
        // Note: This is a simplified implementation. In production,
        // you'd want to implement pagination and more efficient queries
        let all_docs: Map<BytesN<32>, NotarizationRecord> = env.storage().get(&DOCUMENTS)
            .unwrap_or(Map::new(&env));
            
        for record in all_docs.values() {
            if record.notary == notary {
                documents.push_back(record.document_hash);
            }
        }
        
        Ok(documents)
    }

    fn is_notary(env: Env, address: Address) -> bool {
        let notaries: Vec<Address> = env.storage().get(&NOTARIES).unwrap();
        notaries.contains(&address)
    }
}

// Tests module
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, AuthorizedFunction}, vec, map};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        let admin = Address::random(&env);
        let fee = 100;
        
        assert!(client.initialize(&admin, &fee).is_ok());
    }

    #[test]
    fn test_notarization_flow() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NotaryContract);
        let client = NotaryContractClient::new(&env, &contract_id);
        
        // Initialize contract
        let admin = Address::random(&env);
        let notary = Address::random(&env);
        let fee = 100;
        
        client.initialize(&admin, &fee).unwrap();
        client.add_notary(&notary).unwrap();
        
        // Create test document hash
        let document_hash = BytesN::random(&env);
        let expiration = env.ledger().timestamp() + 86400; // 24 hours
        let metadata = map![&env];
        
        // Test notarization
        env.set_authorized_function(AuthorizedFunction::Contract(contract_id.clone()));
        client.notarize(&document_hash, &expiration, &metadata).unwrap();
        
        // Test verification
        let verification = client.verify(&document_hash).unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.notary, notary);
    }
}
