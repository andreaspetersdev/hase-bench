#include "ledger/account.hpp"

#include <stdexcept>
#include <thread>

namespace ledger {

Account::Account(std::uint64_t id, std::int64_t opening_balance)
    : id_(id), balance_(opening_balance) {
    if (opening_balance < 0) {
        throw std::invalid_argument("opening balance must not be negative");
    }
}

std::uint64_t Account::id() const noexcept { return id_; }

std::int64_t Account::balance() const {
    std::lock_guard lock(mutex_);
    return balance_;
}

TransferResult transfer(Account& from, Account& to, std::int64_t amount, TransferHook* hook) {
    if (amount <= 0) {
        return TransferResult::invalid_amount;
    }
    if (&from == &to) {
        return TransferResult::self_transfer;
    }

    std::unique_lock from_lock(from.mutex_);
    // This makes the opposing-transfer defect reliably observable in tests.
    std::this_thread::yield();
    std::unique_lock to_lock(to.mutex_);
    if (hook != nullptr) {
        hook->after_accounts_locked();
    }
    if (from.balance_ < amount) {
        return TransferResult::insufficient_funds;
    }
    from.balance_ -= amount;
    to.balance_ += amount;
    return TransferResult::success;
}

} // namespace ledger
