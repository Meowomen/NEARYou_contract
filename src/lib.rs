use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey,
};

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct NearYou {
    pub nft_contract: AccountId,
    pub accounts: LookupMap<PublicKey, Balance>,
    pub nft_accounts: LookupMap<PublicKey, String>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
    NftAccounts,
}

/// Access key allowance for linkdrop keys.
pub const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000_000;

/// Gas attached to the functionCall.
pub const ON_CREATE_ACCOUNT_CALLBACK_GAS: Gas = Gas(20_000_000_000_000);
pub const NFT_TRANSFER_GAS: Gas = Gas(20_000_000_000_000);

/// Indicates there are no deposit for a callback for better readability.
pub const NO_DEPOSIT: u128 = 0;

#[ext_contract(ext_self)]
pub trait ExtLinkDrop {
    /// Callback after creating account and claiming linkdrop.
    fn on_account_created_and_claimed(&mut self, nft_id: String) -> bool;
}

fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "Contract expected a result on the callback"
    );
    match env::promise_result(0) {
        PromiseResult::Successful(_) => true,
        _ => false,
    }
}

#[near_bindgen]
impl NearYou {
    /// Initialize contract with an account that minted NFT.
    #[init]
    pub fn new(nft_contract: AccountId) -> Self {
        Self {
            nft_contract,
            accounts: LookupMap::new(StorageKey::Accounts),
            nft_accounts: LookupMap::new(StorageKey::NftAccounts),
        }
    }

    /// Map public_key with nft_id & balance to make a promise call.
    #[payable]
    pub fn send(&mut self, public_key: PublicKey, nft_id: String) -> Promise {
        assert!(
            env::attached_deposit() > ACCESS_KEY_ALLOWANCE,
            "Attached deposit must be greater than ACCESS_KEY_ALLOWANCE"
        );

        let pk = public_key.into();
        let value = self.accounts.get(&pk).unwrap_or(0);

        self.nft_accounts.insert(&pk, &nft_id);
        self.accounts.insert(
            &pk,
            &(value + env::attached_deposit() - ACCESS_KEY_ALLOWANCE),
        );

        /// Add access key to the contract.
        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            (&"claim,create_account_and_claim").to_string(),
        )
    }

    /// Claim NFT to existing account.
    pub fn claim(&mut self, account_id: AccountId) -> Promise {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Claim only can come from this account"
        );
        assert!(
            env::is_valid_account_id(account_id.as_bytes()),
            "Invalid account id"
        );

        let nft_id = self
            .nft_accounts
            .remove(&env::signer_account_pk())
            .expect("Unexpected public key");

        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());
        Promise::new(self.nft_contract.clone()).function_call(
            (&"nft_transfer").to_string(),
            format!(
                "{{\"receiver_id\": \"{}\", \"token_id\": \"{}\"}}",
                account_id, nft_id
            )
            .into_bytes(),
            1,
            NFT_TRANSFER_GAS,
        )
    }

    /// Create new account and and claim NFT to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) -> Promise {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Create account and claim only can come from this account"
        );
        assert!(
            env::is_valid_account_id(new_account_id.as_bytes()),
            "Invalid account id"
        );

        let nft_id = self
            .nft_accounts
            .remove(&env::signer_account_pk())
            .expect("Unexpected public key");
        let amount = self
            .accounts
            .remove(&env::signer_account_pk())
            .expect("Unexpected public key");

        /// Modify new_account_id from wallet to create subAccount of the sender.
        let nft_contract = format!(".{}", &env::current_account_id());
        let new_new_account_id = new_account_id
            .clone()
            .to_string()
            .replace(".testnet", &nft_contract);

        /// Create subAccount of the sender.
        Promise::new(AccountId::new_unchecked(new_new_account_id.clone()))
            .create_account()
            .add_full_access_key(new_public_key.into())
            .transfer(amount);
        Promise::new(self.nft_contract.clone())
            .function_call(
                (&"nft_transfer").to_string(),
                format!(
                    "{{\"receiver_id\": \"{}\", \"token_id\": \"{}\"}}",
                    new_new_account_id, nft_id
                )
                .into_bytes(),
                1,
                NFT_TRANSFER_GAS,
            )
            .then(ext_self::on_account_created_and_claimed(
                nft_id,
                env::current_account_id(),
                NO_DEPOSIT,
                ON_CREATE_ACCOUNT_CALLBACK_GAS,
            ))
    }

    /// Callback after execution `create_account_and_claim`.
    pub fn on_account_created_and_claimed(&mut self, nft_id: String) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Callback can only be called from the contract"
        );

        let creation_succeeded = is_promise_success();
        if creation_succeeded {
            Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());
        } else {
            // In case of failure, put the amount back.
            self.nft_accounts.insert(&env::signer_account_pk(), &nft_id);
        }
        creation_succeeded
    }

    /// Returns the nft_id associated with given key.
    pub fn get_key_balance(&self, key: PublicKey) -> String {
        self.nft_accounts
            .get(&key.into())
            .expect("Key is missing")
            .into()
    }
}
