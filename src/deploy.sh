#!/bin/bash

soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/notary_contract.wasm \
  --network testnet \
  --source SBPW2BGMHBFKVBZUBLPVDVWI42P4CC2LHYGQSGDHLF7AUOARPNNMKU33
