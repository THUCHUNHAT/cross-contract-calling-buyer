#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

#[cfg_attr(test, allow(dead_code))]
const ON_ERC_1155_RECEIVED_SELECTOR: [u8; 4] = [0xF2, 0x3A, 0x6E, 0x61];

#[ink::contract]
pub mod incrementer {
    use crate::ON_ERC_1155_RECEIVED_SELECTOR;
    use dyn_traits::Increment;
    use ink::env::debug_println;
    use ink::prelude::vec::Vec;
    // use ink::env::Error;
    // use ink::prelude::assert::ensure;
    use ink::storage::Mapping;
    type Owner = AccountId;
    type Operator = AccountId;

    type TokenId = u64;

    /// A concrete incrementer smart contract.
    #[ink(storage)]
    pub struct Incrementer {
        value: u64,
        price_per_token: u128,
        balances: Mapping<(AccountId, TokenId), Balance>,
        token_id_nonce: TokenId,
    }

    use scale::Encode;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]

    pub enum Error {
        UnexistentToken,
        ZeroAddressTransfer,
        NotApproved,
        InsufficientBalance,
        SelfApproval,
        BatchTransferMismatch,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct TransferSingle {
        #[ink(topic)]
        operator: Option<AccountId>,
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        token_id: TokenId,
        value: Balance,
    }

    impl Incrementer {
        // use ink::prelude::AccountId
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                value: u64::default(),
                // balances:Balance::default(),
                price_per_token: Default::default(),
                balances: Mapping::new(),
                token_id_nonce: 0,
            }
        }

        // #[ink(message)]
        // pub fn inc_by(&mut self, delta: u128) {
        //     self.price_per_token+=delta;
        // }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: TokenId) -> Balance {
            self.balances.get((owner, token_id)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_ids: Vec<TokenId>,
            values: Vec<Balance>,
            data: Vec<u8>,
        ) -> Result<()> {
            let caller = self.env().caller();
            if caller != from {
                if !self.is_approved_for_all(from, caller) {
                    return Err(Error::NotApproved);
                }
            }

            if to == Self::zero_address() {
                return Err(Error::ZeroAddressTransfer);
            }

            if token_ids.is_empty() || token_ids.len() != values.len() {
                return Err(Error::BatchTransferMismatch);
            }

            let transfers = token_ids.iter().zip(values.iter());
            for (&id, &v) in transfers.clone() {
                let balance = self.balance_of(from, id);
                if balance < v {
                    return Err(Error::InsufficientBalance);
                }
            }

            for (&id, &v) in transfers {
                self.perform_transfer(from, to, id, v);
            }

            // Can use any token ID/value here, we really just care about knowing if
            // `to` is a smart contract which accepts transfers
            self.transfer_acceptance_check(caller, from, to, token_ids[0], values[0], data);

            Ok(())
        }

        #[ink(message)]
        pub fn mint(&mut self, token_id: TokenId, value: Balance) -> Result<()> {
            if !(token_id <= self.token_id_nonce) {
                return Err(Error::UnexistentToken);
            }

            let caller = self.env().caller();
            self.balances.insert((caller, token_id), &value);

            // Emit transfer event but with mint semantics
            self.env().emit_event(TransferSingle {
                operator: Some(caller),
                from: None,
                to: Some(caller),
                token_id,
                value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn sell_token(
            &mut self,
            buyer: AccountId,
            token_id: TokenId,
            amount: Balance,
            price_per_token: Balance,
            // delta: u128
        ) -> Result<()> {
            self.price_per_token = price_per_token;

            let seller: AccountId = self.env().caller();
            let seller_balance: Balance = self.balance_of(seller, token_id);
            if seller_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            if buyer == seller {
                debug_println!("Nguoi ban khong the tu mua token cua chinh minh");
                return Err(Error::SelfApproval);
            }

            let total_price: u128 = price_per_token * amount;
            debug_println!(
                "Nguoi ban {} token voi ma {} token va nhan duoc tong {} tien.",
                amount,
                token_id,
                total_price
            );

            let data: Vec<u8> = Vec::new();

            self.safe_batch_transfer_from(
                seller,
                buyer,
                ink::prelude::vec![token_id],
                ink::prelude::vec![amount],
                data,
            )?;

            self.mint(token_id, seller_balance)?;

            //Nguoi mua

            let buyer_balance: Balance = self.balance_of(buyer, token_id);

            if buyer_balance < total_price{
                debug_println!("Insufficient balance to buy tokens");
                return Err(Error::InsufficientBalance);
            }

            let data: Vec<u8> = Vec::new();

            self.safe_batch_transfer_from(
                buyer,
                seller,
                ink::prelude::vec![token_id],
                ink::prelude::vec![total_price],
                data,
            )?;

            self.mint(token_id, buyer_balance)?;

            Ok(())
        }

        // #[ink(message)]
        // pub fn sell_token(
        //     &mut self,
        //     buyer: AccountId,
        //     token_id: TokenId,
        //     amount: Balance,
        //     price_per_token: Balance,
        // ) -> Result<()> {
        //     self.price_per_token = price_per_token;
        //     let seller: AccountId = self.env().caller();
        //     let seller_balance: Balance = self.balance_of(seller, token_id);
        //     if seller_balance < amount {
        //         return Err(Error::InsufficientBalance);
        //     }
        //     if buyer == seller {
        //         debug_println!("Nguoi ban khong the tu mua token cua chinh minh");
        //         return Err(Error::SelfApproval);
        //     }

        //     let total_price: Balance = price_per_token * amount;
        //     debug_println!(
        //         "Nguoi ban {} token voi ma {} token va nhan duoc tong {} tien.",
        //         amount,
        //         token_id,
        //         total_price
        //     );

        //     // Check buyer's balance
        //     let buyer_balance: Balance = self.balance_of(buyer, token_id);
        //     if buyer_balance < total_price {
        //         debug_println!("Insufficient balance to buy tokens");
        //         return Err(Error::InsufficientBalance);
        //     }

        //     // Transfer tokens from seller to buyer
        //     let data1: Vec<u8> = Vec::new();
        //     self.safe_batch_transfer_from(
        //         seller,
        //         buyer,
        //         ink::prelude::vec![token_id],
        //         ink::prelude::vec![amount],
        //         data1,
        //     )?;

        //     // Transfer payment from buyer to seller
        //     let data2: Vec<u8> = Vec::new();
        //     self.safe_batch_transfer_from(
        //         buyer,
        //         seller,
        //         ink::prelude::vec![token_id],
        //         ink::prelude::vec![total_price],
        //         data2,
        //     )?;

        //     Ok(())
        // }

        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            true
        }

        fn zero_address() -> AccountId {
            [0u8; 32].into()
        }

        pub fn perform_transfer(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            value: Balance,
        ) {
            let mut sender_balance = self
                .balances
                .get((from, token_id))
                .expect("Caller should have ensured that `from` holds `token_id`.");
            sender_balance -= value;
            self.balances.insert((from, token_id), &sender_balance);

            let mut recipient_balance = self.balances.get((to, token_id)).unwrap_or(0);
            recipient_balance += value;
            self.balances.insert((to, token_id), &recipient_balance);

            let caller = self.env().caller();
            self.env().emit_event(TransferSingle {
                operator: Some(caller),
                from: Some(from),
                to: Some(to),
                token_id,
                value,
            });
        }

        #[cfg_attr(test, allow(unused_variables))]
        pub fn transfer_acceptance_check(
            &mut self,
            caller: AccountId,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            value: Balance,
            data: Vec<u8>,
        ) {
            // This is disabled during tests due to the use of `invoke_contract()` not
            // being supported (tests end up panicking).
            #[cfg(not(test))]
            {
                use ink::env::call::{build_call, ExecutionInput, Selector};

                // If our recipient is a smart contract we need to see if they accept or
                // reject this transfer. If they reject it we need to revert the call.
                let result = build_call::<Environment>()
                    .call(to)
                    .gas_limit(5000)
                    .exec_input(
                        ExecutionInput::new(Selector::new(ON_ERC_1155_RECEIVED_SELECTOR))
                            .push_arg(caller)
                            .push_arg(from)
                            .push_arg(token_id)
                            .push_arg(value)
                            .push_arg(data),
                    )
                    .returns::<Vec<u8>>()
                    .params()
                    .try_invoke();

                match result {
                    Ok(v) => {
                        ink::env::debug_println!(
                            "Received return value \"{:?}\" from contract {:?}",
                            v.clone()
                                .expect("Call should be valid, don't expect a `LangError`."),
                            from
                        );
                        assert_eq!(
                            v.clone()
                                .expect("Call should be valid, don't expect a `LangError`."),
                            &ON_ERC_1155_RECEIVED_SELECTOR[..],
                            "The recipient contract at {to:?} does not accept token transfers.\n
                            Expected: {ON_ERC_1155_RECEIVED_SELECTOR:?}, Got {v:?}"
                        )
                    }
                    Err(e) => {
                        match e {
                            ink::env::Error::CodeNotFound | ink::env::Error::NotCallable => {
                                // Our recipient wasn't a smart contract, so there's
                                // nothing more for
                                // us to do
                                ink::env::debug_println!(
                                    "Recipient at {:?} from is not a smart contract ({:?})",
                                    from,
                                    e
                                );
                            }
                            _ => {
                                // We got some sort of error from the call to our
                                // recipient smart
                                // contract, and as such we must revert this call
                                panic!("Got error \"{e:?}\" while trying to call {from:?}")
                            }
                        }
                    }
                }
            }
        }

        #[ink(message)]
        pub fn create(&mut self, value: Balance) -> TokenId {
            let caller = self.env().caller();

            // Given that TokenId is a `u128` the likelihood of this overflowing is pretty
            // slim.
            self.token_id_nonce += 1;
            self.balances.insert((caller, self.token_id_nonce), &value);

            // Emit transfer event but with mint semantics
            self.env().emit_event(TransferSingle {
                operator: Some(caller),
                from: None,
                to: if value == 0 { None } else { Some(caller) },
                token_id: self.token_id_nonce,
                value,
            });

            self.token_id_nonce
        }
    }

    impl Increment for Incrementer {
        // #[ink(message)]
        // fn inc(&mut self) {
        //     self.sell_token(1)
        // }

        #[ink(message)]
        fn inc(&mut self) {
            // Gọi hàm sell_token với các tham số cần thiết
            let caller = self.env().caller();
            let token_id = Default::default();
            let amount = Default::default();
            let price_per_token = Default::default();
            self.sell_token(caller, token_id, amount, price_per_token)
                .unwrap_or_else(|err| {
                    // Xử lý lỗi nếu có
                    debug_println!("Error: {:?}", err);
                });
        }

        #[ink(message)]
        fn get(&self) -> Balance {
            self.price_per_token
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn it_works() {
            let mut incrementer = Incrementer::new();
            assert_eq!(<Incrementer as Increment>::get(&incrementer), 0);
            <Incrementer as Increment>::inc(&mut incrementer);
            assert_eq!(incrementer.get(), 1);
        }
    }
}
