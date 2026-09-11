#include "ledger/account.hpp"

#include <atomic>
#include <barrier>
#include <cassert>
#include <chrono>
#include <future>
#include <stdexcept>
#include <thread>

using namespace ledger;
using namespace std::chrono_literals;

namespace {
class BlockingHook final : public TransferHook {
public:
    void after_accounts_locked() override {
        entered.set_value();
        release.wait();
    }

    std::promise<void> entered;
    std::promise<void> released;
    std::shared_future<void> release = released.get_future().share();
};

class ThrowingHook final : public TransferHook {
public:
    void after_accounts_locked() override { throw std::runtime_error("audit failure"); }
};

void opposite_transfers_make_progress() {
    for (int round = 0; round != 12; ++round) {
        Account left(10, 100);
        Account right(20, 100);
        std::barrier start(3);
        std::atomic<int> successes{0};

        std::thread forward([&] {
            start.arrive_and_wait();
            if (transfer(left, right, 1) == TransferResult::success) {
                ++successes;
            }
        });
        std::thread backward([&] {
            start.arrive_and_wait();
            if (transfer(right, left, 1) == TransferResult::success) {
                ++successes;
            }
        });
        start.arrive_and_wait();
        forward.join();
        backward.join();
        assert(successes == 2);
        assert(left.balance() + right.balance() == 200);
    }
}

void independent_pairs_are_not_globally_serialized() {
    Account first_from(1, 20);
    Account first_to(2, 0);
    Account second_from(3, 20);
    Account second_to(4, 0);
    BlockingHook hook;

    auto blocked = std::async(std::launch::async, [&] {
        return transfer(first_from, first_to, 5, &hook);
    });
    assert(hook.entered.get_future().wait_for(2s) == std::future_status::ready);

    auto independent = std::async(std::launch::async, [&] {
        return transfer(second_from, second_to, 7);
    });
    assert(independent.wait_for(2s) == std::future_status::ready);
    assert(independent.get() == TransferResult::success);

    hook.released.set_value();
    assert(blocked.wait_for(2s) == std::future_status::ready);
    assert(blocked.get() == TransferResult::success);
    assert(first_from.balance() + first_to.balance() + second_from.balance() + second_to.balance() == 40);
}

void concurrency_and_failure_invariants() {
    Account a(101, 600);
    Account b(102, 600);
    Account c(103, 600);
    std::barrier start(4);
    auto worker = [&](Account& from, Account& to) {
        start.arrive_and_wait();
        for (int i = 0; i != 400; ++i) {
            assert(transfer(from, to, 1) == TransferResult::success);
        }
    };
    std::thread one(worker, std::ref(a), std::ref(b));
    std::thread two(worker, std::ref(b), std::ref(c));
    std::thread three(worker, std::ref(c), std::ref(a));
    start.arrive_and_wait();
    one.join();
    two.join();
    three.join();
    assert(a.balance() + b.balance() + c.balance() == 1800);
    assert(transfer(a, b, 10'000) == TransferResult::insufficient_funds);
    assert(a.balance() + b.balance() + c.balance() == 1800);
}

void throwing_transfer_cannot_restore_an_unlocked_snapshot() {
    // Hold a shared account long enough that an implementation which captures
    // balances before locking can race a completed overlapping transfer. The
    // public contract permits both operations to overlap; only the first is
    // paused in its after-lock hook.
    for (int round = 0; round != 24; ++round) {
        Account a(201, 100);
        Account b(202, 100);
        Account c(203, 100);
        BlockingHook pause;
        ThrowingHook throwing_hook;

        auto successful = std::async(std::launch::async, [&] {
            return transfer(c, a, 1, &pause);
        });
        assert(pause.entered.get_future().wait_for(2s) == std::future_status::ready);

        auto throwing = std::async(std::launch::async, [&] {
            try {
                (void)transfer(a, b, 1, &throwing_hook);
                return false;
            } catch (const std::runtime_error&) {
                return true;
            }
        });
        // Give the blocked operation a chance to reach its lock acquisition.
        // Correct implementations need no timing assumption: their balances
        // are read and changed only while both account locks are held.
        std::this_thread::sleep_for(5ms);
        pause.released.set_value();

        assert(successful.get() == TransferResult::success);
        assert(throwing.get());
        assert(a.balance() + b.balance() + c.balance() == 300);
    }
}
} // namespace

int main() {
    opposite_transfers_make_progress();
    independent_pairs_are_not_globally_serialized();
    concurrency_and_failure_invariants();
    throwing_transfer_cannot_restore_an_unlocked_snapshot();
}
