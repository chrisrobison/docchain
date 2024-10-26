from fastapi import FastAPI, HTTPException, File, UploadFile, Depends
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm
from pydantic import BaseModel
from typing import Optional, Dict
import hashlib
import jwt
from datetime import datetime, timedelta
from stellar_sdk import Server, Keypair, TransactionBuilder, Network
import asyncio
from motor.motor_asyncio import AsyncIOMotorClient
import uvicorn

# Configuration
SECRET_KEY = "SBPW2BGMHBFKVBZUBLPVDVWI42P4CC2LHYGQSGDHLF7AUOARPNNMKU33"
ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_MINUTES = 30
STELLAR_NETWORK_URL = "https://horizon-testnet.stellar.org"
MONGO_URL = "mongodb://localhost:27017"

# Initialize FastAPI app
app = FastAPI(title="Digital Notary Service API")

# Initialize database
client = AsyncIOMotorClient(MONGO_URL)
db = client.notary_db

# Initialize Stellar SDK
server = Server(horizon_url=STELLAR_NETWORK_URL)

# OAuth2 scheme for token authentication
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="token")

# Pydantic models for request/response validation
class User(BaseModel):
    username: str
    stellar_address: Optional[str] = None
    disabled: Optional[bool] = None

class UserInDB(User):
    hashed_password: str

class Token(BaseModel):
    access_token: str
    token_type: str

class TokenData(BaseModel):
    username: Optional[str] = None

class NotarizationRequest(BaseModel):
    document_hash: str
    metadata: Optional[Dict] = None

class NotarizationResponse(BaseModel):
    transaction_id: str
    timestamp: datetime
    document_hash: str
    status: str

class VerificationResponse(BaseModel):
    verified: bool
    timestamp: Optional[datetime]
    transaction_id: Optional[str]

# Helper functions
def get_document_hash(file_content: bytes) -> str:
    """Generate SHA-256 hash of document content"""
    return hashlib.sha256(file_content).hexdigest()

async def get_user(username: str):
    """Retrieve user from database"""
    user_dict = await db.users.find_one({"username": username})
    if user_dict:
        return UserInDB(**user_dict)
    return None

async def authenticate_user(username: str, password: str):
    """Authenticate user credentials"""
    user = await get_user(username)
    if not user:
        return False
    if not verify_password(password, user.hashed_password):
        return False
    return user

def create_access_token(data: dict, expires_delta: Optional[timedelta] = None):
    """Create JWT access token"""
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.utcnow() + expires_delta
    else:
        expire = datetime.utcnow() + timedelta(minutes=15)
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt

async def get_current_user(token: str = Depends(oauth2_scheme)):
    """Get current user from JWT token"""
    credentials_exception = HTTPException(
        status_code=401,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        username: str = payload.get("sub")
        if username is None:
            raise credentials_exception
        token_data = TokenData(username=username)
    except jwt.PyJWTError:
        raise credentials_exception
    user = await get_user(username=token_data.username)
    if user is None:
        raise credentials_exception
    return user

# API endpoints
@app.post("/token", response_model=Token)
async def login_for_access_token(form_data: OAuth2PasswordRequestForm = Depends()):
    """Endpoint for user authentication and token generation"""
    user = await authenticate_user(form_data.username, form_data.password)
    if not user:
        raise HTTPException(
            status_code=401,
            detail="Incorrect username or password",
            headers={"WWW-Authenticate": "Bearer"},
        )
    access_token_expires = timedelta(minutes=ACCESS_TOKEN_EXPIRE_MINUTES)
    access_token = create_access_token(
        data={"sub": user.username}, expires_delta=access_token_expires
    )
    return {"access_token": access_token, "token_type": "bearer"}

@app.post("/notarize", response_model=NotarizationResponse)
async def notarize_document(
    file: UploadFile = File(...),
    current_user: User = Depends(get_current_user)
):
    """
    Endpoint to notarize a document
    1. Reads and hashes the uploaded file
    2. Submits the hash to the Stellar blockchain
    3. Returns the transaction details
    """
    try:
        # Read and hash file content
        content = await file.read()
        document_hash = get_document_hash(content)

        # Create Stellar transaction
        source_keypair = Keypair.from_secret(current_user.stellar_address)
        source_account = await server.load_account(source_keypair.public_key)

        # Build transaction
        transaction = (
            TransactionBuilder(
                source_account=source_account,
                network_passphrase=Network.TESTNET_NETWORK_PASSPHRASE,
                base_fee=100,
            )
            .append_manage_data_op(
                data_name=document_hash,
                data_value=str(datetime.utcnow().timestamp()),
                source=source_keypair.public_key,
            )
            .set_timeout(30)
            .build()
        )

        # Sign and submit transaction
        transaction.sign(source_keypair)
        response = await server.submit_transaction(transaction)

        # Store notarization record in database
        notarization_record = {
            "document_hash": document_hash,
            "transaction_id": response["id"],
            "timestamp": datetime.utcnow(),
            "user_id": current_user.username,
            "filename": file.filename,
            "status": "completed"
        }
        await db.notarizations.insert_one(notarization_record)

        return NotarizationResponse(
            transaction_id=response["id"],
            timestamp=notarization_record["timestamp"],
            document_hash=document_hash,
            status="completed"
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/verify", response_model=VerificationResponse)
async def verify_document(
    file: UploadFile = File(...),
    current_user: User = Depends(get_current_user)
):
    """
    Endpoint to verify a document's notarization status
    1. Hashes the uploaded file
    2. Checks the Stellar blockchain for the hash
    3. Returns verification details
    """
    try:
        # Read and hash file content
        content = await file.read()
        document_hash = get_document_hash(content)

        # Check database for notarization record
        record = await db.notarizations.find_one({"document_hash": document_hash})

        if not record:
            return VerificationResponse(
                verified=False,
                timestamp=None,
                transaction_id=None
            )

        # Verify on blockchain
        transaction = await server.transactions().transaction(record["transaction_id"]).call()

        return VerificationResponse(
            verified=True,
            timestamp=record["timestamp"],
            transaction_id=record["transaction_id"]
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/notarizations", response_model=list[NotarizationResponse])
async def get_user_notarizations(current_user: User = Depends(get_current_user)):
    """Retrieve all notarizations for the current user"""
    try:
        cursor = db.notarizations.find({"user_id": current_user.username})
        notarizations = await cursor.to_list(length=None)
        return [NotarizationResponse(**notarization) for notarization in notarizations]
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    uvicorn.run("main:app", host="0.0.0.0", port=8000, reload=True)
