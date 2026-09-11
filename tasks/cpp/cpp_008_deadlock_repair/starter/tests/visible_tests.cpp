#include "ledger/account.hpp"

#include <cassert>
#include <stdexcept>

using namespace ledger;

namespace {
class ThrowingHook final : public TransferHook {
public:
    void after_accounts_locked() override { throw std::runtime_error("audit unavailable"); }
};
}

int main() {
    Account checking(100, 90);
    Account savings(200, 10);
    assert(transfer(checking, savings, 25) == TransferResult::success);
    assert(checking.balance() == 65);
    assert(savings.balance() == 35);

    const auto total = checking.balance() + savings.balance();
    assert(transfer(checking, savings, 0) == TransferResult::invalid_amount);
    assert(transfer(checking, savings, -2) == TransferResult::invalid_amount);
    assert(transfer(checking, checking, 1) == TransferResult::self_transfer);
    assert(transfer(checking, savings, 1000) == TransferResult::insufficient_funds);
    assert(checking.balance() + savings.balance() == total);

    ThrowingHook hook;
    try {
        (void)transfer(checking, savings, 1, &hook);
        assert(false);
    } catch (const std::runtime_error&) {
    }
    assert(checking.balance() + savings.balance() == total);

    try {
        Account invalid(300, -1);
        (void)invalid;
        assert(false);
    } catch (const std::invalid_argument&) {
    }
}
