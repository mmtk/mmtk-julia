use log::*;
use mmtk::plan::Plan;
use mmtk::util::conversions;
use mmtk::util::heap::GCTriggerPolicy;
use mmtk::util::heap::SpaceStats;
use mmtk::MMTK;

use crate::JuliaVM;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const DEFAULT_COLLECT_INTERVAL: usize = 5600 * 1024 * std::mem::size_of::<usize>();
const MAX_COLLECT_INTERVAL: usize = 1250000000;
const GC_ALWAYS_SWEEP_FULL: bool = false;

/// This tries to implement Julia-style GC triggering heuristics. However, it is still siginificantly different
/// from the Julia's GC heuristics.
/// 1. Julia counts allocation per thread and compares with a per-thread interval, while this impl counts global
///    allocation and compares that with an estimated global interval. For Julia, the first thread that allocates
///    the amount of bytes that exceeds the interval will trigger a GC. For us, as MMTk does not count allocation
///    per thread, we calculate an estiamted global interval (using thread interval * n_mutator / 2), and compare
///    global allocation with it.
/// 2. Julia makes the decision of full heap GC after marking and before sweeping (they call it full sweep), while
///    MMTk makes such decisions before a GC. So for us, we use Julia's decision, but force a full heap GC in the next
///    next GC (not the current one).
/// 3. Julia counts the pointers in the remembered set, and if the remembered set is too large (large_frontier),
///    they will do full heap GC. MMTk does not collect such information about remembered set, so we do not have the heuristic
///    based on the remembered set.
pub struct JuliaGCTrigger {
    total_mem: AtomicUsize,
    max_total_memory: AtomicUsize,
    /// Julia's gc_num.interval. This is a per-thread value. The first thread that allocates this amount of bytes will trigger a GC.
    interval: AtomicUsize,
    /// Multiply interval with the estimated threads. As we only count total pages of the allocation (not allocation per thread),
    /// we multiply the interval with the estimated threads (half of the mutator threads -- Julia also uses this way to do estimation. See
    /// totalmem / nthreads / 2).
    interval_all_threads: AtomicUsize,
    actual_allocd: AtomicUsize,
    prev_sweep_full: AtomicBool,
    /// The number of pending allocation pages. The allocation requests for them have failed, and a GC is triggered.
    /// We will need to take them into consideration so that the new heap size can accomodate those allocations.
    pending_pages: AtomicUsize,
    last_recorded_reserved_pages: AtomicUsize,
}

impl JuliaGCTrigger {
    pub fn new(total_mem: usize, constrained_mem: usize, size_hint: usize) -> Self {
        // ported from jl_gc_init 64bits
        let mut total_mem = total_mem;
        if constrained_mem > 0 && constrained_mem < total_mem {
            total_mem = constrained_mem;
        }
        let percent: f64 = if (total_mem as f64) < 123e9 {
            // 60% at 0 gigs and 90% at 128 to not
            // overcommit too much on memory contrained devices
            (total_mem as f64) * 2.34375e-12 + 0.6
        } else {
            0.9f64
        };
        let max_total_memory =
            if size_hint > 0 && size_hint < (1usize << (std::mem::size_of::<usize>() * 8 - 1)) {
                size_hint as f64
            } else {
                (total_mem as f64) * percent
            };

        trace!(
            "Julia GC Trigger: total mem = {:?}, max mem = {:?}, interval = {:?}",
            total_mem,
            max_total_memory,
            DEFAULT_COLLECT_INTERVAL
        );

        Self {
            total_mem: AtomicUsize::new(total_mem),
            max_total_memory: AtomicUsize::new(max_total_memory as usize),
            interval: AtomicUsize::new(DEFAULT_COLLECT_INTERVAL),
            interval_all_threads: AtomicUsize::new(DEFAULT_COLLECT_INTERVAL),
            actual_allocd: AtomicUsize::new(0),
            prev_sweep_full: AtomicBool::new(true),
            last_recorded_reserved_pages: AtomicUsize::new(0),
            pending_pages: AtomicUsize::new(0),
        }
    }
}

impl GCTriggerPolicy<JuliaVM> for JuliaGCTrigger {
    fn on_gc_start(&self, mmtk: &'static MMTK<JuliaVM>) {
        let reserved_pages_in_last_gc = self.last_recorded_reserved_pages.load(Ordering::Relaxed);
        // reserved pages now should include pending allocations
        let reserved_pages_now =
            mmtk.get_plan().get_reserved_pages() + self.pending_pages.load(Ordering::SeqCst);

        self.last_recorded_reserved_pages
            .store(reserved_pages_now, Ordering::Relaxed);
        self.actual_allocd.store(
            conversions::pages_to_bytes(
                reserved_pages_now.saturating_sub(reserved_pages_in_last_gc),
            ),
            Ordering::Relaxed,
        );
        self.prev_sweep_full.store(
            if let Some(gen) = mmtk.get_plan().generational() {
                gen.last_collection_full_heap()
            } else {
                false
            },
            Ordering::Relaxed,
        );
        trace!("On GC start: alloc'd = {} = reserved now {} pages - reserved after last gc {} pages = {} pages, prev_sweep = {}", self.actual_allocd.load(Ordering::Relaxed), reserved_pages_now, reserved_pages_in_last_gc, reserved_pages_now.saturating_sub(reserved_pages_in_last_gc), self.prev_sweep_full.load(Ordering::Relaxed));
    }
    fn on_gc_end(&self, mmtk: &'static MMTK<JuliaVM>) {
        use crate::mmtk::vm::ActivePlan;
        let n_mutators = crate::active_plan::VMActivePlan::number_of_mutators();

        let reserved_pages_before_gc = self.last_recorded_reserved_pages.load(Ordering::Relaxed);
        let reserved_pages_now =
            mmtk.get_plan().get_reserved_pages() + self.pending_pages.load(Ordering::SeqCst);
        let freed = conversions::pages_to_bytes(
            reserved_pages_before_gc.saturating_sub(reserved_pages_now),
        );
        self.last_recorded_reserved_pages
            .store(reserved_pages_now, Ordering::Relaxed);
        trace!("On GC end: freed = {} = reserved before GC {} pages - reserved now {} pages = {} pages", freed, reserved_pages_before_gc, reserved_pages_now, reserved_pages_before_gc.saturating_sub(reserved_pages_now));

        // ported from gc.c -- before sweeping in the original code.
        // ignore large frontier (large frontier means the bytes of pointers reachable from the remset is larger than the default collect interval)
        let gc_auto = !mmtk.is_user_triggered_collection();
        let not_freed_enough = gc_auto
            && ((freed as f64) < (self.actual_allocd.load(Ordering::Relaxed) as f64 * 0.7f64));
        let mut sweep_full = false;
        if gc_auto {
            if not_freed_enough {
                trace!(
                    "Not freed enough: double the interval {:?} -> {:?}",
                    self.interval.load(Ordering::Relaxed),
                    self.interval.load(Ordering::Relaxed) * 2
                );
                self.interval
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| {
                        Some(interval * 2)
                    })
                    .unwrap();
            }

            // on a big memory machine, increase max_collect_interval to totalmem / nthreads / 2
            let mut maxmem = self.total_mem.load(Ordering::Relaxed) / n_mutators / 2;
            if maxmem < MAX_COLLECT_INTERVAL {
                maxmem = MAX_COLLECT_INTERVAL;
            }
            if self.interval.load(Ordering::Relaxed) > maxmem {
                sweep_full = true;
                trace!(
                    "Force full heap. Clamp interval back to max mem ({} > {}).",
                    self.interval.load(Ordering::Relaxed),
                    maxmem
                );
                self.interval.store(maxmem, Ordering::Relaxed);
            }
        }

        let live_bytes = conversions::pages_to_bytes(reserved_pages_now);
        if live_bytes > self.max_total_memory.load(Ordering::Relaxed) {
            trace!("Force full heap. Live > max total memory");
            sweep_full = true;
        }
        if GC_ALWAYS_SWEEP_FULL {
            trace!("Force full heap. Always");
            sweep_full = true;
        }

        if sweep_full {
            if let Some(gen) = mmtk.get_plan().generational() {
                // Force full heap in the next GC
                gen.force_full_heap_collection();
            }
        }

        // ported from gc.c -- after sweeping in the original code
        if gc_auto {
            // If we aren't freeing enough or are seeing lots and lots of pointers (large_frontier, ignored) let it increase faster
            if not_freed_enough {
                let tot =
                    2f64 * (live_bytes + self.actual_allocd.load(Ordering::Relaxed)) as f64 / 3f64;
                let _ = self.interval.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| if (interval as f64) > tot {
                    trace!("Not freed enough. Decrease interval to 2/3 * (live + alloc'd) = {}", tot);
                    Some(tot as usize)
                } else { None });
            } else {
                // If the current interval is larger than half the live data decrease the interval
                let half = live_bytes / 2;
                let _ =
                    self.interval
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| {
                            if interval > half {
                                trace!("Decrease interval to half live bytes = {}", half);
                                Some(half)
                            } else {
                                None
                            }
                        });
            }

            // But never go below default
            let _ = self
                .interval
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| {
                    if interval < DEFAULT_COLLECT_INTERVAL {
                        trace!(
                            "Don't let interval go below default = {}",
                            DEFAULT_COLLECT_INTERVAL
                        );
                        Some(DEFAULT_COLLECT_INTERVAL)
                    } else {
                        None
                    }
                });
        }

        let max_total_memory = self.max_total_memory.load(Ordering::Relaxed);
        if self.interval.load(Ordering::Relaxed) + live_bytes > max_total_memory {
            if live_bytes < max_total_memory {
                trace!(
                    "About to reach max total memory. Decrease interval = {}",
                    max_total_memory.saturating_sub(live_bytes)
                );
                self.interval.store(
                    max_total_memory.saturating_sub(live_bytes),
                    Ordering::Relaxed,
                );
            } else {
                trace!(
                    "Reached max total memory. Use default interval = {}",
                    DEFAULT_COLLECT_INTERVAL
                );
                // We can't stay under our goal so let's go back to
                // the minimum interval and hope things get better
                self.interval
                    .store(DEFAULT_COLLECT_INTERVAL, Ordering::Relaxed);
            }
        }

        // After interval is computed, estimate interval_all_threads
        self.interval_all_threads.store(
            self.interval.load(Ordering::Relaxed) * n_mutators / 2,
            Ordering::Relaxed,
        );

        // Clear pending allocation pages at the end of GC, no matter we used it or not.
        self.pending_pages.store(0, Ordering::SeqCst);
    }

    fn on_pending_allocation(&self, pages: usize) {
        self.pending_pages.fetch_add(pages, Ordering::SeqCst);
    }

    /// Is a GC required now?
    fn is_gc_required(
        &self,
        space_full: bool,
        space: Option<SpaceStats<JuliaVM>>,
        plan: &dyn Plan<VM = JuliaVM>,
    ) -> bool {
        let reserved_pages_now = plan.get_reserved_pages();
        let reserved_pages_before_gc = self.last_recorded_reserved_pages.load(Ordering::Relaxed);

        let allocd_so_far = conversions::pages_to_bytes(
            reserved_pages_now.saturating_sub(reserved_pages_before_gc),
        );

        trace!(
            "Reserved now = {}, last recorded reserved = {}, Allocd so far: {}. interval_all_threads = {}",
            plan.get_reserved_pages(), self.last_recorded_reserved_pages.load(Ordering::Relaxed), allocd_so_far,
            self.interval_all_threads.load(Ordering::Relaxed)
        );

        // Check against interval_all_threads, as we count allocation from all threads.
        if allocd_so_far > self.interval_all_threads.load(Ordering::Relaxed) {
            return true;
        }

        plan.collection_required(space_full, space)
    }

    // Basically there is no limit for the heap size.

    /// Is current heap full?
    fn is_heap_full(&self, _plan: &dyn Plan<VM = JuliaVM>) -> bool {
        false
    }

    /// Return the current heap size (in pages)
    fn get_current_heap_size_in_pages(&self) -> usize {
        usize::MAX
    }

    /// Return the upper bound of heap size
    fn get_max_heap_size_in_pages(&self) -> usize {
        usize::MAX
    }

    /// Can the heap size grow?
    fn can_heap_size_grow(&self) -> bool {
        true
    }
}
