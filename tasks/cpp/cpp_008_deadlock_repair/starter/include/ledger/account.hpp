#pragma once

#include <cstdint>
#include <mutex>

namespace ledger {

enum class TransferResult {
    success,
    invalid_amount,
    self_transfer,
    insufficient_funds,
};

class TransferHook {
public:
    virtual ~TransferHook() = default;
    virtual void after_accounts_locked() = 0;
};

class Account {
public:
    Account(std::uint64_t id, std::int64_t opening_balance);
    Account(const Account&) = delete;
    Account& operator=(const Account&) = delete;
    Account(Account&&) = delete;
    Account& operator=(Account&&) = delete;

    [[nodiscard]] std::uint64_t id() const noexcept;
    [[nodiscard]] std::int64_t balance() const;

private:
    friend TransferResult transfer(Account&, Account&, std::int64_t, TransferHook*);

    std::uint64_t id_;
    mutable std::mutex mutex_;
    std::int64_t balance_;
};

[[nodiscard]] TransferResult transfer(Account& from, Account& to, std::int64_t amount,
                                       TransferHook* hook = nullptr);

} // namespace ledger
