use scrypto::prelude::*;

#[derive(ScryptoSbor)]
pub struct TokenAmountCall {
    pub method_name: String,
    pub component_address: ComponentAddress,
    pub with_arg: bool,
    pub initialized: bool,
}

#[blueprint]
mod percentage_vester {
    enable_method_auth! {
        roles {
            overseer => updatable_by: [];
            vester => updatable_by: [];
        },
        methods {
            get_amount_unvestable => PUBLIC;
            get_amount_unvested => PUBLIC;
            get_token_amount => PUBLIC;
            add_method_call => restrict_to: [vester];
            remove_method_call => restrict_to: [overseer];
            remove_uninitialized_method_call => restrict_to: [vester];
            initialize_uninitialized => restrict_to: [vester];
            put_back_tokens => restrict_to: [vester];
            unvest_tokens => restrict_to: [vester];
            emergency_unvest_allow => restrict_to: [overseer];
            emergency_unvest => restrict_to: [vester];
        }
    }

    struct PercentageVester {
        //amount of tokens initially vested
        tokens_initially_vested: Decimal,
        //amount of tokens unvested
        tokens_unvested: Decimal,
        //percentage of circulating tokens that's allowed to be unvested
        max_percentage_unvested: Decimal,
        //method calls that get non-circulating tokens
        method_calls: Vec<TokenAmountCall>,
        //vault with tokens
        token_vault: Vault,
        //address of vested token
        vested_token_address: ResourceAddress,
        //whether emergency unvest has been turned on
        emergency_unvest: bool,
    }

    impl PercentageVester {
        pub fn instantiate_vest(tokens_to_vest: Bucket, max_percentage_unvested: Decimal, method_calls: Vec<TokenAmountCall>, overseer_address: ResourceAddress, vested_token_address: ResourceAddress) -> (Global<PercentageVester>, Bucket) {
            let tokens_initially_vested: Decimal = tokens_to_vest.amount();
            let overseer_access_rule: AccessRule = rule!(require(overseer_address));

            let vester_token: Bucket = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata(metadata! (
                    init {
                        "name" => "Vested token owner", updatable;
                        "symbol" => "VEST", updatable;
                    }
                ))
                .mint_initial_supply(1)
                .into();

            let component = Self {
                tokens_initially_vested,
                tokens_unvested: dec!(0),
                max_percentage_unvested,
                method_calls,
                token_vault: Vault::with_bucket(tokens_to_vest),
                vested_token_address,
                emergency_unvest: false,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(vester_token.resource_address()))))
            .roles(roles! {
                overseer => overseer_access_rule;
                vester => OWNER;
            })
            .globalize();

            (component, vester_token)
        }

        pub fn get_amount_unvestable(&self) -> Decimal {
            let mut non_circulating_tokens: Decimal = dec!(0);
            let token_supply: Decimal = ResourceManager::from(self.vested_token_address).total_supply().expect("Token supply not found...");

            //sorry for the non-iter loop Daan
            for call in &self.method_calls {
                let component: Global<AnyComponent> = Global::from(call.component_address);
                if call.with_arg {
                    let tokens_to_add: Decimal = component.call_raw(&call.method_name, scrypto_args!(self.vested_token_address));
                    non_circulating_tokens += tokens_to_add;
                } else {
                    let tokens_to_add: Decimal = component.call_raw(&call.method_name, scrypto_args!());
                    non_circulating_tokens += tokens_to_add;
                } 
            }

            let max_tokens_unvested: Decimal = self.max_percentage_unvested * ((token_supply - self.tokens_initially_vested - non_circulating_tokens) / (1 - self.max_percentage_unvested));

            max_tokens_unvested - self.tokens_unvested
        }

        pub fn get_token_amount(&mut self) -> Decimal {
            self.token_vault.amount()
        }

        pub fn get_amount_unvested(&mut self) -> Decimal {
            self.tokens_unvested
        }

        pub fn add_method_call(&mut self, method_name: String, with_arg: bool, component_address: ComponentAddress) {
            let call = TokenAmountCall {
                method_name,
                component_address,
                with_arg,
                initialized: false,
            };
            self.method_calls.push(call);
        }

        pub fn remove_uninitialized_method_call(&mut self, method_name: String, component_address: ComponentAddress) {
            self.method_calls.retain(|call| {
                !(call.method_name == method_name && call.component_address == component_address && !call.initialized)
            });
        }

        pub fn remove_method_call(&mut self, method_name: String, component_address: ComponentAddress) {
            self.method_calls.retain(|call| {
                !(call.method_name == method_name && call.component_address == component_address)
            });
        }

        pub fn initialize_uninitialized(&mut self) {
            //Daan don't kill me, for loops are just more readable than iter() + filter...
            for call in &self.method_calls {
                if !call.initialized {
                    let component: Global<AnyComponent> = Global::from(call.component_address);
                    if call.with_arg {
                        let _test_dec: Decimal = component.call_raw(&call.method_name, scrypto_args!(self.vested_token_address));
                    } else {
                        let _test_dec: Decimal = component.call_raw(&call.method_name, scrypto_args!());
                    }
                }
            }
        }

        pub fn unvest_tokens(&mut self) -> Option<Bucket> {
            let amount_unvestable: Decimal = self.get_amount_unvestable();

            if amount_unvestable > dec!(0) {
                
                //this could be nicer but this will definitely work always without any rounding error bullcrap
                if amount_unvestable > self.token_vault.amount() {
                    self.tokens_unvested += self.token_vault.amount();
                    Some(self.token_vault.take_all())
                } else {
                    self.tokens_unvested += amount_unvestable;
                    Some(self.token_vault.take(amount_unvestable))
                }
            } else {
                None
            }
        }

        pub fn put_back_tokens(&mut self, tokens: Bucket) {
            self.tokens_unvested -= tokens.amount();
            self.token_vault.put(tokens);
        }

        pub fn emergency_unvest_allow(&mut self) {
            self.emergency_unvest = true;
        }

        pub fn emergency_unvest(&mut self) -> Bucket {
            assert!(self.emergency_unvest, "Emergency unvest not allowed!");

            self.token_vault.take_all()
        }
    }
}
