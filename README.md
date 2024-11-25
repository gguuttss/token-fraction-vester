# Circulating supply-based unvesting
This Scrypto blueprint provides a mechanism to vest tokens based on the percentage of circulating tokens. The design ensures that the total unvested tokens are always smaller than a chosen percentage of the circulating supply.

## Example Scenario
Let's consider a token:
- Total Token Supply: 1,000,000 tokens
- Initially Vested Tokens: 100,000 tokens
- Tokens Held By DAO: 700,000 tokens

We now want the vester to be able to have unvested at most 20% of the circulating tokens.

The blueprint calculates the unvestable tokens like so:
```
non_circulating_tokens = 700,000
initial_vest = 100,000
circulating_tokens = 1,000,000 - 700,000 - 100,000 = 200,000

max_tokens_unvested = 0.20 * ((1,000,000 - 500,000 - 300,000) / (1 - 0.20))
                    = 0.20 * (200,000 / 0.80)
                    = 0.20 * 250,000
                    = 50,000 tokens
```

We can see that the vester can at maximum have unvested 50,000 tokens, let's check whether this is correct:

There are 200,000 circulating tokens not originating from the vest. If the vester unvests 50,000 a total of 200,000 + 50,000 tokens is circulating. 50,000 / 250,000 = 0.2, so the vester has unvested 20% of the circulating supply.

## Use case
So, why would you want to vest in such a convoluted way? One reason might be that you have two parties with conflicting agendas, that still want to agree on a vest.

Imagine we have a DAO founder that wants to vest his initial allocation of governance tokens. It is important that the quantity of tokens vested is high enough to make sure the DAO founder doesn't control the entire protocol by themself. This makes a time-based vest inadequate: in a scenario where the DAO isn't bringing more tokens from its treasury into circulation, you don't want the DAO founder to be able to unvest more of his tokens!

A solution would be to let the DAO control the unvesting, but this is undesirable for the DAO founder. Why would he want the DAO to be able to control his unvesting? They might just not unvest his tokens at all...

In comes the solution: basing the amount of unvestable tokens on the circulating supply of the token! This way, we can make sure the DAO founder and DAO don't rely on each other for the unvesting process to proceed in a fair way. If the DAO starts increasing the circulating supply, the DAO founder is mostly unaffected and just receives his vested tokens. If the DAO decides to not distribute any of its locked up treasury tokens, the DAO founder's allocation just remains constant.

## How It Works
1. **Initialization**:
   - Vester initializes component by passing the tokens to vest and some parameters that specify the exact vest.
   - Vester registers components holding non-circulating tokens with method calls that fetch the amount of tokens held by them (`TokenAmountCall` struct).
2. **Unvesting**:
   - All non-circulating tokens are added up through executing `TokenAmountCall`s.
   - The maximum amount of tokens that can be unvested is calculated based on the chosen maximum unvest percentage against the circulating supply.
   - See the above example to see such a calculation.
3. **Emergency Management**:
   - In case of urgent situations, tokens can be unvested immediately using the `emergency_unvest_now` method. This should only be callable by a party that does not necessarily want the tokens to be unvested.

## Methods
The blueprint defines the following key methods:

### Public Methods
- `get_amount_unvestable`: Returns the amount of tokens that can be unvested based on the circulating supply.
- `get_amount_unvested`: Returns the current amount of unvested tokens.
- `get_token_amount`: Returns the token amount still in the vesting component.

### Restricted Methods
- `add_method_call`: Adds a new component to components that hold non-circulating tokens, accompanied with a method to fetch the amount of tokens it contains  (restricted to `vester`).
- `remove_method_call`: Removes a component and method call (restricted to `overseer`).
- `initialize_uninitialized`: To prevent adding incorrect method calls, all method calls are added in an uninitialized state, and tested to actually return a value before initializing them (restricted to `vester`).
- `remove_uninitialized_method_call`: Removes a component and method call that was contained an error and therefore can't be initialized (restricted to `overseer`).
- `put_back_tokens`: Returns tokens back to the vesting pool (restricted to `vester`).
- `unvest_tokens`: Unvests all unvestable tokens (restricted to `vester`).
- `emergency_unvest_now`: Immediately unvests all tokens (restricted to `overseer`).
- `emergency_unvest`: Begins an emergency unvest, through which all tokens can be unvested after a specific amount of time set at initialization of the component (restricted to `vester`).

## Roles
- `overseer`: A neutral party that does not really want tokens to be unvested fast. Can correct errors, or immediately unvest all tokens if necessary. Can be trusted with this responsibility, as there is no incentive to unvest faster. Could for example be a DAO overseeing the unvesting process of its founder's initial token allocation.
- `vester`: Person that wants their tokens to be unvested. Can operate all unvesting locking, and can add extra sources of non-circulating tokens if needed. An example of a vester would be a DAO's founder, vesting their initial token allocation.
