#pragma once

// This header is force-included into benchmark executables by the validator.
// Keep Debug assertions active, but make their failure observable through the
// captured test stream instead of opening a modal CRT/Windows dialog.
#ifdef _MSC_VER
#include <crtdbg.h>
#include <cstdlib>
#include <windows.h>

namespace hasebench {
namespace detail {
struct NonInteractiveMsvcRuntime {
    NonInteractiveMsvcRuntime() noexcept {
        _CrtSetReportMode(_CRT_ASSERT, _CRTDBG_MODE_FILE);
        _CrtSetReportFile(_CRT_ASSERT, _CRTDBG_FILE_STDERR);
        _set_abort_behavior(0, _WRITE_ABORT_MSG | _CALL_REPORTFAULT);
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }
};

inline const NonInteractiveMsvcRuntime non_interactive_runtime{};
} // namespace detail
} // namespace hasebench
#endif
