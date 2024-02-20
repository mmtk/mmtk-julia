use mmtk::util::heap::GCTriggerPolicy;
use mmtk::util::heap::SpaceStats;
use mmtk::plan::Plan;
use mmtk::util::conversions;
use mmtk::MMTK;
use log::*;

use crate::JuliaVM;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const DEFAULT_COLLECT_INTERVAL: usize = 5600 * 1024 * std::mem::size_of::<usize>();
const MAX_COLLECT_INTERVAL: usize = 1250000000;
const GC_ALWAYS_SWEEP_FULL: bool = false;

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
        let max_total_memory = if size_hint > 0 && size_hint < (1usize << (std::mem::size_of::<usize>() * 8 - 1)) {
            size_hint as f64
        } else {
            (total_mem as f64) * percent
        };

        trace!("Julia GC Trigger: total mem = {:?}, max mem = {:?}, interval = {:?}", total_mem, max_total_memory, DEFAULT_COLLECT_INTERVAL);

        Self {
            total_mem: AtomicUsize::new(total_mem),
            max_total_memory: AtomicUsize::new(max_total_memory as usize),
            interval: AtomicUsize::new(DEFAULT_COLLECT_INTERVAL),
            interval_all_threads: AtomicUsize::new(DEFAULT_COLLECT_INTERVAL),
            actual_allocd: AtomicUsize::new(0),
            prev_sweep_full: AtomicBool::new(true),
            last_recorded_reserved_pages: AtomicUsize::new(0),
        }
    }
}

impl GCTriggerPolicy<JuliaVM> for JuliaGCTrigger {
    fn on_gc_start(&self, mmtk: &'static MMTK<JuliaVM>) {
        let reserved_pages_in_last_gc = self.last_recorded_reserved_pages.load(Ordering::Relaxed);
        let reserved_pages_now = mmtk.get_plan().get_reserved_pages();
        self.last_recorded_reserved_pages.store(reserved_pages_now, Ordering::Relaxed);
        self.actual_allocd.store(conversions::pages_to_bytes(reserved_pages_now.saturating_sub(reserved_pages_in_last_gc)), Ordering::Relaxed);
        self.prev_sweep_full.store(if let Some(gen) = mmtk.get_plan().generational() {
            gen.last_collection_full_heap()
        } else {
            false
        }, Ordering::Relaxed);
        trace!("On GC start: alloc'd = {} = reserved now {} pages - reserved after last gc {} pages = {} pages, prev_sweep = {}", self.actual_allocd.load(Ordering::Relaxed), reserved_pages_now, reserved_pages_in_last_gc, reserved_pages_now.saturating_sub(reserved_pages_in_last_gc), self.prev_sweep_full.load(Ordering::Relaxed));
    }
    fn on_gc_end(&self, mmtk: &'static MMTK<JuliaVM>) {
        use crate::mmtk::vm::ActivePlan;
        let n_mutators = crate::active_plan::VMActivePlan::number_of_mutators();

        let reserved_pages_before_gc = self.last_recorded_reserved_pages.load(Ordering::Relaxed);
        let reserved_pages_now = mmtk.get_plan().get_reserved_pages();
        let freed = conversions::pages_to_bytes(reserved_pages_before_gc.saturating_sub(reserved_pages_now));
        self.last_recorded_reserved_pages.store(reserved_pages_now, Ordering::Relaxed);
        trace!("On GC end: freed = {} = reserved before GC {} pages - reserved now {} pages = {} pages", freed, reserved_pages_before_gc, reserved_pages_now, reserved_pages_before_gc.saturating_sub(reserved_pages_now));


        // ported from gc.c -- before sweeping in the original code.
        // ignore large frontier (large frontier means the bytes of pointers reachable from the remset is larger than the default collect interval)
        let gc_auto = !mmtk.is_user_triggered_collection();
        let not_freed_enough = gc_auto && (freed as f64) < (self.actual_allocd.load(Ordering::Relaxed) as f64 * 0.7f64);
        let mut sweep_full = false;
        if gc_auto {
            if not_freed_enough {
                trace!("Not freed enough: double the interval {:?} -> {:?}", self.interval.load(Ordering::Relaxed), self.interval.load(Ordering::Relaxed) * 2);
                self.interval.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| Some(interval * 2)).unwrap();
            }

            // on a big memory machine, increase max_collect_interval to totalmem / nthreads / 2
            let mut maxmem = self.total_mem.load(Ordering::Relaxed) / n_mutators / 2;
            if maxmem < MAX_COLLECT_INTERVAL {
                maxmem = MAX_COLLECT_INTERVAL;
            }
            if self.interval.load(Ordering::Relaxed) > maxmem {
                sweep_full = true;
                trace!("Force full heap. Clamp interval back to max mem ({} > {}).", self.interval.load(Ordering::Relaxed), maxmem);
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
                let tot = 2f64 * (live_bytes + self.actual_allocd.load(Ordering::Relaxed)) as f64 / 3f64;
                let _ = self.interval.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| if (interval as f64) > tot {
                    trace!("Not freed enough. Decrease interval to 2/3 * (live + alloc'd) = {}", tot);
                    Some(tot as usize)
                } else { None });
            } else {
                // If the current interval is larger than half the live data decrease the interval
                let half = live_bytes / 2;
                let _ = self.interval.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| if interval > half {
                    trace!("Decrease interval to half live bytes = {}", half);
                    Some(half)
                } else { None });
            }

            // But never go below default
            let _ = self.interval.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |interval| if interval < DEFAULT_COLLECT_INTERVAL {
                trace!("Don't let interval go below default = {}", DEFAULT_COLLECT_INTERVAL);
                Some(DEFAULT_COLLECT_INTERVAL)
            } else { None });
        }

        let max_total_memory = self.max_total_memory.load(Ordering::Relaxed);
        if self.interval.load(Ordering::Relaxed) + live_bytes > max_total_memory {
            if live_bytes < max_total_memory {
                trace!("About to reach max total memory. Decrease interval = {}", max_total_memory.saturating_sub(live_bytes));
                self.interval.store(max_total_memory.saturating_sub(live_bytes), Ordering::Relaxed);
            } else {
                trace!("Reached max total memory. Use default interval = {}", DEFAULT_COLLECT_INTERVAL);
                // We can't stay under our goal so let's go back to
                // the minimum interval and hope things get better
                self.interval.store(DEFAULT_COLLECT_INTERVAL, Ordering::Relaxed);
            }
        }

        // After interval is computed, estimate interval_all_threads
        self.interval_all_threads.store(self.interval.load(Ordering::Relaxed) * n_mutators / 2, Ordering::Relaxed);
    }

    /// Is a GC required now?
    fn is_gc_required(
        &self,
        space_full: bool,
        space: Option<SpaceStats<JuliaVM>>,
        plan: &dyn Plan<VM = JuliaVM>,
    ) -> bool {
        let allocd_so_far = conversions::pages_to_bytes(plan.get_reserved_pages() - self.last_recorded_reserved_pages.load(Ordering::Relaxed));
        // Check against interval_all_threads, as we count allocation from all threads.
        if allocd_so_far > self.interval_all_threads.load(Ordering::Relaxed) {
            return true;
        }

        plan.collection_required(space_full, space)
    }

    /// Is current heap full?
    fn is_heap_full(&self, plan: &dyn Plan<VM = JuliaVM>) -> bool {
        plan.get_reserved_pages() >= conversions::bytes_to_pages_up(self.max_total_memory.load(Ordering::Relaxed))
    }

    /// Return the current heap size (in pages)
    fn get_current_heap_size_in_pages(&self) -> usize {
        conversions::bytes_to_pages_up(self.max_total_memory.load(Ordering::Relaxed))
    }

    /// Return the upper bound of heap size
    fn get_max_heap_size_in_pages(&self) -> usize {
        conversions::bytes_to_pages_up(self.max_total_memory.load(Ordering::Relaxed))
    }

    /// Can the heap size grow?
    fn can_heap_size_grow(&self) -> bool {
        true
    }
}
