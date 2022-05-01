# Payment Toy
A toy commandline application for processing transaction records.

## Quick Example
```
cargo run -- tx_records.csv
```
The provided records db contains a very large deposit of just over one
quintillion moneys to client `612` followed by a series of relatively
small thousand money withdrawals, to demonstrate the fixed point
arithmatic's correct behavior when compared to `f64`.

## Withdrawal Disputes
I'm unsure how a system might handle disputes of withdrawal
transactions, and so such disputes are reported as errorneous.


## Error handling / UX
`main` panics whenever the arguments are invalid, the input file is
inaccessable, or the parsing fails; all errors from process_transaction
are ignored, `main` simply moves on to the next record.

Using some error logging/tracing would make this more realistic, but is
left undone.

The fixed point arithmatic will panic on overflow, even in release
mode. Checking for overflow in the payment toy and cancelling the
transaction may be better, but is a touch of code bloat too much for a
presumably unrealistic situation.

Since only withdrawal and deposit transactions use the `amount` column,
constructing a `Transaction` with or without an amount of moneys
inappropriately is impossible, and the deserializer and constructors
for `Transaction` prevents negative amounts.


### Notably Absent Optimizations
[`fxhash`](https://crates.io/crates/fxhash) is likely not faster for
the small 16-bit `Client` identifiers, and so is not even worth
considering given the denial of service risk. `std`'s default hasher is
used instead, well known for its DOS resistance.
