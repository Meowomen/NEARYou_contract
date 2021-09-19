# NEARYou Contract
## About NEARYou

NEARYou allows NEAR wallet user(sender) to create link for giving their NFT(Non-Fungible-Token). Their friends(receiver) can claim NFT through the link. NEARYou contract stores sender's NFT's token_id(NFT id) and NEAR for activaing new account to send NFT when receiver requests claim.

## How NEARYou Works

Sender, who has NFT:

- Call `send` function to create new key pair and store sender's NFT `token_id` and balance.
- `send` function add access key to give contract authority for moving sender's NFT.

Receiver, who doesn't have NEAR wallet account:

- Call `create_account_and_claim` function of contract with private key.
- `create_account_and_claim` function calls `create_account` and create sender's subaccount as a receiver's new account.
- `create_account_and_claim` function calls `nft_transfer` function of NFT-minting contract to give sender's NFT to receiver.

Receiver, who has NEAR wallet account:

- Call `claim` function of contract with private key.
- `claim` function calls `nft_transfer` function of NFT-minting contract to give sender's NFT to receiver.

### Code

**send()**

```rust
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

        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            (&"claim,create_account_and_claim").to_string(),
        )
    }
```

- Inserts public key-NFT_id pair and public key-balance in the accounts, nft_accounts.
- Make promise and add access key to NEARYou contract.

**claim()**

```rust
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
```

- Get `nft_id` from nft_accounts map.
- Call `nft_transfer()` from nft contract.

**create_account_and_claim()**

```rust
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

        let nft_contract = format!(".{}", &env::current_account_id());
        let new_new_account_id = new_account_id
            .clone()
            .to_string()
            .replace(".testnet", &nft_contract);

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
```

- Get `amount` and `nft_id` from map.
- Create sub account(`new_new_account`) of sender's account.
- Call `nft_transfer()` from nft contract.

## Getting Started

Clone this repository

```bash
git clone https://github.com/Meowomen/NEARYou_contract
cd NEARYou_contract
```

Compile Contract code

```bash
cargo build --target wasm32-unknown-unknown --release
```

Deploy Contract

```jsx
near deploy --wasmFile target/wasm32-unknown-unknown/release/nearyou.wasm --accountId YOUR_ACCOUNT_HERE
```

Init Contract

```bash
near call YOUR_ACCOUNT new '{"nft_contract":"NFT_MINTiNG_CONTRACT"}' --accountId SIGNER_ACCOUNT
```

After deploy NEARYou contract, you can use NEARYou contract with your account id in the [demo page](https://github.com/HeesungB/near-drop-demo)

## Suggestion

We suggest adding `making subaccount` menu in the NEAR web wallet. In NEAR protocol, newly created account must be under a namespace of the creator account however, NEAR official wallet has not the create subaccount menu. Adding create subaccount feature can make NEAR users easily attract others through their own contract so that expand the NEAR ecosystem.
