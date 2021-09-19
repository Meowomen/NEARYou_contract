# NEARYou Contract
## About NEARYou

NEARYou allows NEAR wallet users(sender) to create a link for gifting their NFTs(Non-Fungible-Token) which follow [NEP-171](https://github.com/near/NEPs/blob/ea409f07f8/specs/Standards/NonFungibleToken/Core.md) standard. The user's friends(receiver) can claim NFT through the link. NEARYou contract stores the sender's NFT ``token_id`` and minimum amount of NEAR to activate new account.

## How NEARYou Works

Sender, who owns NFT:

- Call `send` function to create new key pair and store sender's NFT `token_id` and balance.
- `send` function adds an access key to give NEARYou contract authority for moving sender's NFT.

Receiver, who doesn't have NEAR wallet account:

- Call `create_account_and_claim` function of contract with private key.
- `create_account_and_claim` function calls `create_account` and creates the sender's subaccount as a receiver's new account.
- `create_account_and_claim` function calls `nft_transfer` function of ``NFT_MINTED_CONTRACT``(account that minted NFT) to give sender's NFT to receiver.

Receiver, who has NEAR wallet account:

- Call `claim` function of contract with private key.
- `claim` function calls `nft_transfer` function of ``NFT_MINTED_CONTRACT`` to give sender's NFT to receiver.

### Code

#### **send()**

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

- Inserts [public key, nft_id] pair and [public key, amount] pair into the ``accounts``, ``nft_accounts`` respectively.
- Make promise and add an access key to NEARYou contract.

#### **claim()**

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

- Get `nft_id` from ``nft_accounts`` map.
- Call `nft_transfer()` from ``NFT_MINTED_CONTRACT``.

#### **create_account_and_claim()**

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
- Call `nft_transfer()` from ``NFT_MINTED_CONTRACT``.

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
- Otherwise, you can skip the previous step(Compile) and deploy [``nearyou.wasm``](https://github.com/Meowomen/NEARYou_contract/blob/master/res/nearyou.wasm) directly.

Init Contract

```bash
near call YOUR_ACCOUNT new '{"nft_contract":"NFT_MINTED_CONTRACT"}' --accountId SIGNER_ACCOUNT
```
- ``NFT_MINTED_CONTRACT`` means an account that minted your NFT

After deploying, you can use NEARYou contract with your account id in the [demo page](https://github.com/Meowomen/NEARYou/blob/master/README.md#modify-configjs).

## Suggestions

- Update [``createNewAccount``](https://github.com/near/near-wallet/blob/b98294ed8125ca63b6123f56195cc6d35995df37/packages/frontend/src/utils/wallet.js#L409) function in NEAR wallet

``fundingContract`` and ``fundingAccount`` must be included in the drop-link to receive NFT at the same time as account creation through the official wallet. However, if both exist, wallet call the function ``createNewAccountLinkdrop``, which [calls the ``create_account_and_claim``](https://github.com/near/near-wallet/blob/b98294ed8125ca63b6123f56195cc6d35995df37/packages/frontend/src/utils/wallet.js#L489) in the ``fundingContract``. For NEARYou contract to work in the official wallet, both the function name and the number of factors had to be the same. However, we needed the id of the ``NFT_MINTED_CONTRACT`` in ``create_account_and_claim`` [to transfer nft](https://github.com/Meowomen/NEARYou_contract/blob/master/src/lib.rs#L149), so we declared it a global variable through the init function because it shouldn't be hard-coded for scalability. If NEAR wallet flexibly handles account creation with ``fundingAccounts`` and ``fundingContracts``, init function will not be necessary.

- Support `subaccount creation` in NEAR wallet

This proposal begins with what we did not realize about the signer of the cross-contract call. When calling ``create_account`` of the ``testnet``(official contract) within the NEARYou contract, the top-level-account will be made normally because ``testnet`` signs it, but if ``NEARYou`` signs, only subAccount can be made. We realized this late, so we made subAccount using [a little trick](https://github.com/Meowomen/NEARYou_contract/blob/master/src/lib.rs#L139) because of the naming rules. We will, of course, update the contract later, but adding ``subaccount creation`` feature in official wallet can make NEAR users easily attract others through their own contract so that expand the NEAR ecosystem.
