# CPP-008 — Deadlock-free account transfers

This small ledger component has a reproducible deadlock under opposing
concurrent transfers. Repair it without changing the public header, public
function signatures, or CMake target names.

`transfer(from, to, amount, hook)` moves a positive integral amount from
`from` to `to` when sufficient funds are available. It returns:

* `success` after exactly that movement;
* `invalid_amount` when `amount <= 0`;
* `self_transfer` when `from` and `to` name the same account and the amount is
  positive;
* `insufficient_funds` when the source balance is smaller than the amount.

The validation order is the order above. Failed transfers must leave both
balances unchanged. `Account` construction rejects a negative opening balance
with `std::invalid_argument`.

The sum of balances across a set of accounts is an invariant: every completed
or failed transfer, including a transfer whose hook throws, must preserve it.
Transfers may overlap with each other, including a throwing transfer and a
successful transfer sharing one account; neither may lose or overwrite the
other transfer's completed balance change.
`TransferHook::after_accounts_locked` is called only for a positive,
non-self transfer after both participating accounts have been locked and
before its balance check or mutation. Exceptions from that callback propagate;
the transfer must nevertheless leave balances unchanged and release locks.

The implementation must permit independent transfers on disjoint account
pairs to make progress while another pair is paused in a hook. Do not solve
the bug by serialising every transfer through a global mutex, by retry/sleep
loops, or by changing the public API. Establish a local multi-account locking
strategy that cannot deadlock under arbitrarily many opposing transfers.

`Account::balance()` may be called concurrently with transfers and returns a
consistent value. Accounts are identity-bearing, non-copyable objects.

Build and run the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
