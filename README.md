# Payment Toy
A toy commandline application for processing transaction records.

## Quick Example
```
cargo run -- tx_records.csv
```
The provided records db contains a very large deposit of just over one
septillion moneys to client `612` followed by a series of relatively
small thousand money withdrawals, to demonstrate the fixed point
arithmatic's correct behavior when compared to `f64`.

## Withdrawal Disputes
I'm unsure how a system might handle disputes of withdrawal
transactions, and so such disputes are ignored.


## Error handling / UX
`Account::process_transaction` swallows many errors, returning and
silently aborting the transaction; the comments above each return
statement  indicate the nature of each error.

`main` panics whenever the arguments are invalid, unable to access the
input file fails, and parsing fails.

Using some error logging/tracing would make this more realistic, but is 
left undone.

The fixed point arithmatic will panic on overflow, even in release
mode. Checking for overflow in the payment toy and cancelling the
transaction may be better, but is a touch of code bloat too much for a
presumably unrealistic situation.

Since only withdrawal and deposit transactions use the `amount` column,
constructing a `Transaction` with or without an amount of moneys
inappropriately is impossible, and the deserializer for `Transaction`
ensures records 


### Notably Absent Optimizations
The fixed point moneys arithmatic likely doesn't need 96 bits of
precision, and the crate is arguably wasting 31 bits for our needs; if
[the rust_decimal crate](https://crates.io/crates/rust_decimal) allowed
the exponent and size to be specified like the
[`fixed`](https://crates.io/crates/fixed) binary fixed arithmatic crate
does, we could likely get away with 64 bit fixed point maths.

[`fxhash`](https://crates.io/crates/fxhash) is likely not faster for
the small 16-bit `Client` identifiers, and so is not even worth
considering given the denial of service risk. `std`'s default hasher is
used instead, well known for its DOS resistance.
