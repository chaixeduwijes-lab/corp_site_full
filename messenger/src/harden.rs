//! Process-level memory hardening (audit finding F1).
//!
//! The relay keeps ciphertext (and, under on-box TLS, key material) in RAM and
//! promises it never reaches disk. That promise only holds if the kernel is
//! prevented from paging the process out to swap or writing a core dump. This
//! module applies both barriers at startup:
//!
//! - `mlockall(MCL_CURRENT | MCL_FUTURE)` — lock all current and future pages
//!   into RAM so they are never swapped.
//! - `setrlimit(RLIMIT_CORE, 0)` + `prctl(PR_SET_DUMPABLE, 0)` — no core dump,
//!   and deny `ptrace`/`/proc/pid/mem` reads from non-root processes.
//!
//! Everything here is best-effort: locking memory needs `CAP_IPC_LOCK` or a
//! raised `RLIMIT_MEMLOCK` (see the systemd unit's `LimitMEMLOCK=infinity`). If
//! a step fails we log a warning rather than aborting, so the relay still runs
//! in constrained environments — but operators should treat a failed lock as a
//! broken "nothing touches disk" guarantee.

/// Apply all hardening steps. Returns the number of steps that succeeded.
#[cfg(unix)]
pub fn apply() -> HardenReport {
    let mut report = HardenReport::default();

    // Disable core dumps for this process.
    // SAFETY: setrlimit with a valid resource id and a well-formed rlimit.
    let no_core = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    report.core_dumps_disabled = unsafe { libc::setrlimit(libc::RLIMIT_CORE, &no_core) } == 0;

    // Mark the process non-dumpable: blocks core dumps that setrlimit alone
    // might not, and denies ptrace / /proc/<pid>/mem to non-root readers.
    // SAFETY: prctl PR_SET_DUMPABLE takes an int argument; extra args ignored.
    report.non_dumpable = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } == 0;

    // Lock all current and future pages so they are never paged to swap.
    // SAFETY: mlockall with valid flags; no memory is dereferenced.
    report.memory_locked = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0;

    report
}

#[cfg(not(unix))]
pub fn apply() -> HardenReport {
    HardenReport::default()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct HardenReport {
    pub memory_locked: bool,
    pub core_dumps_disabled: bool,
    pub non_dumpable: bool,
}

impl HardenReport {
    /// True only if every disk-exposure barrier is in place.
    pub fn fully_hardened(&self) -> bool {
        self.memory_locked && self.core_dumps_disabled && self.non_dumpable
    }
}
