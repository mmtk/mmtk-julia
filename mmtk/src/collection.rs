use crate::jl_gc_log;
use crate::SINGLETON;
use crate::{
    jl_gc_get_max_memory, jl_gc_prepare_to_collect, jl_gc_update_stats, jl_get_gc_disable_counter,
    jl_hrtime, jl_throw_out_of_memory_error,
};
use crate::{JuliaVM, USER_TRIGGERED_GC};
use log::{debug, trace};
use mmtk::util::alloc::AllocationError;
use mmtk::util::heap::GCTriggerPolicy;
use mmtk::util::opaque_pointer::*;
use mmtk::vm::{Collection, GCThreadContext};
use mmtk::Mutator;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};

use crate::{BLOCK_FOR_GC, STW_COND, WORLD_HAS_STOPPED};

pub static GC_START: AtomicU64 = AtomicU64::new(0);
static CURRENT_GC_MAY_MOVE: AtomicBool = AtomicBool::new(true);

// The current GC count. Used to track the number of GCs that have occurred, and what GC it is right now.
// This count is bumped after a GC.
static GC_COUNT: AtomicU64 = AtomicU64::new(0);

use std::collections::HashSet;
use std::sync::RwLock;
use std::thread::ThreadId;

#[cfg(feature = "print_fragmentation")]
use crate::api::print_fragmentation;

lazy_static! {
    static ref GC_THREADS: RwLock<HashSet<ThreadId>> = RwLock::new(HashSet::new());
}

pub(crate) fn register_gc_thread() {
    let id = std::thread::current().id();
    GC_THREADS.write().unwrap().insert(id);
}
pub(crate) fn unregister_gc_thread() {
    let id = std::thread::current().id();
    GC_THREADS.write().unwrap().remove(&id);
}
pub(crate) fn is_gc_thread() -> bool {
    let id = std::thread::current().id();
    GC_THREADS.read().unwrap().contains(&id)
}

pub struct VMCollection {}

impl Collection<JuliaVM> for VMCollection {
    fn stop_all_mutators<F>(_tls: VMWorkerThread, mut mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<JuliaVM>),
    {
        // Wait for all mutators to stop and all finalizers to run
        while !AtomicBool::load(&WORLD_HAS_STOPPED, Ordering::SeqCst) {
            // Stay here while the world has not stopped
            // FIXME add wait var
        }

        assert!(Self::is_collection_enabled(), "Collection is disabled when threads are stopped for a GC. This is a concurrency bug, see https://github.com/mmtk/mmtk-julia/issues/278.");

        trace!("Stopped the world!");
        #[cfg(feature = "heap_dump")]
        dump_heap(GC_COUNT.load(Ordering::SeqCst), 0);

        // Store if the current GC may move objects -- we will use it when the current GC finishes.
        // We cache the value here just in case MMTk may clear it before we use the value.
        CURRENT_GC_MAY_MOVE.store(
            crate::SINGLETON.get_plan().current_gc_may_move_object(),
            Ordering::SeqCst,
        );

        // Tell MMTk the stacks are ready.
        {
            use mmtk::vm::ActivePlan;
            for mutator in crate::active_plan::VMActivePlan::mutators() {
                debug!("stop_all_mutators: visiting {:?}", mutator.mutator_tls);
                mutator_visitor(mutator);
            }
        }

        // Record the start time of the GC
        let now = unsafe { jl_hrtime() };
        trace!("gc_start = {}", now);
        GC_START.store(now, Ordering::Relaxed);
    }

    fn resume_mutators(_tls: VMWorkerThread) {
        unsafe {
            jl_gc_log(); // Remove dead objects form pinning log
        }
        // unpin conservative roots
        crate::conservative::unpin_conservative_roots();

        // Get the end time of the GC
        let end = unsafe { jl_hrtime() };
        trace!("gc_end = {}", end);
        let gc_time = end - GC_START.load(Ordering::Relaxed);
        unsafe {
            jl_gc_update_stats(
                gc_time,
                crate::api::mmtk_used_bytes(),
                is_current_gc_nursery(),
            )
        }

        #[cfg(feature = "heap_dump")]
        dump_heap(GC_COUNT.load(Ordering::SeqCst), 1);
        GC_COUNT.fetch_add(1, Ordering::SeqCst);

        #[cfg(feature = "dump_block_stats")]
        dump_immix_block_stats();
        #[cfg(feature = "print_fragmentation")]
        print_fragmentation();

        AtomicBool::store(&BLOCK_FOR_GC, false, Ordering::SeqCst);
        AtomicBool::store(&WORLD_HAS_STOPPED, false, Ordering::SeqCst);

        let (_, cvar) = &*STW_COND.clone();
        cvar.notify_all();

        debug!(
            "Live bytes = {}, total bytes = {}",
            crate::api::mmtk_used_bytes(),
            crate::api::mmtk_total_bytes()
        );

        trace!("Resuming mutators.");
    }

    fn block_for_gc(_tls: VMMutatorThread) {
        debug!("Triggered GC!");

        unsafe { jl_gc_prepare_to_collect() };

        debug!("Finished blocking mutator for GC!");
    }

    fn spawn_gc_thread(_tls: VMThread, ctx: GCThreadContext<JuliaVM>) {
        // Just drop the join handle. The thread will run until the process quits.
        let _ = std::thread::Builder::new()
            .name("MMTk Worker".to_string())
            .spawn(move || {
                use mmtk::util::opaque_pointer::*;
                use mmtk::util::Address;

                // Remember this GC thread
                register_gc_thread();

                // Start the worker loop
                let worker_tls = VMWorkerThread(VMThread(OpaquePointer::from_address(unsafe {
                    Address::from_usize(thread_id::get())
                })));
                match ctx {
                    GCThreadContext::Worker(w) => {
                        mmtk::memory_manager::start_worker(&SINGLETON, worker_tls, w)
                    }
                }

                // The GC thread quits somehow. Unresgister this GC thread
                unregister_gc_thread();
            });
    }

    fn schedule_finalization(_tls: VMWorkerThread) {}

    fn out_of_memory(_tls: VMThread, _err_kind: AllocationError) {
        println!("Out of Memory!");
        unsafe { jl_throw_out_of_memory_error() };
    }

    fn vm_live_bytes() -> usize {
        crate::api::JULIA_MALLOC_BYTES.load(Ordering::SeqCst)
    }

    fn is_collection_enabled() -> bool {
        unsafe { jl_get_gc_disable_counter() == 0 }
    }

    fn create_gc_trigger() -> Box<dyn GCTriggerPolicy<JuliaVM>> {
        use crate::gc_trigger::*;
        let max_memory = unsafe { jl_gc_get_max_memory() };
        Box::new(JuliaGCTrigger::new(max_memory))
    }
}

pub fn is_current_gc_nursery() -> bool {
    match crate::SINGLETON.get_plan().generational() {
        Some(gen) => gen.is_current_gc_nursery(),
        None => false,
    }
}

pub fn is_current_gc_moving() -> bool {
    CURRENT_GC_MAY_MOVE.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn mmtk_block_thread_for_gc() {
    AtomicBool::store(&BLOCK_FOR_GC, true, Ordering::SeqCst);

    let (lock, cvar) = &*STW_COND.clone();
    let mut count = lock.lock().unwrap();

    debug!("Blocking for GC!");

    AtomicBool::store(&WORLD_HAS_STOPPED, true, Ordering::SeqCst);

    while AtomicBool::load(&BLOCK_FOR_GC, Ordering::SeqCst) {
        count = cvar.wait(count).unwrap();
    }

    AtomicIsize::store(&USER_TRIGGERED_GC, 0, Ordering::SeqCst);
}

#[cfg(feature = "dump_block_stats")]
pub fn dump_immix_block_stats() {
    use mmtk::util::Address;
    use mmtk::util::ObjectReference;
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("output-block-stats.log") // ← Replace with your desired file path
        .expect("Unable to open log file");

    SINGLETON.enumerate_objects(
        |space_name: &str, block_start: Address, block_size: usize, object: ObjectReference| {
            if space_name == "immix" {
                writeln!(
                    file,
                    "Block: {}, object: {} ({}), size: {}, pinned: {}",
                    block_start,
                    object,
                    unsafe {
                        crate::julia_scanning::get_julia_object_type(object.to_raw_address())
                    },
                    unsafe {
                        crate::object_model::get_so_object_size(
                            object,
                            crate::object_model::get_hash_size(object),
                        )
                    },
                    mmtk::memory_manager::is_pinned(object),
                )
                .expect("Unable to write to log file");
            } else if space_name == "nonmoving" {
                writeln!(
                    file,
                    "Nonmoving: {}, object: {} ({}), size: {}, reachable: {}",
                    block_start,
                    object,
                    unsafe {
                        crate::julia_scanning::get_julia_object_type(object.to_raw_address())
                    },
                    unsafe { crate::object_model::get_so_object_size(object, 0) },
                    object.is_reachable(),
                )
                .expect("Unable to write to log file");
            }
        },
    );
}

#[cfg(feature = "heap_dump")]
pub fn dump_heap(gc_count: u64, phase: u64) {
    // phase: 0 for pre-gc, 1 for post-gc
    println!("Dumping heap for GC {} phase {}", gc_count, phase);
    use json::JsonValue;
    use mmtk::util::constants::LOG_BYTES_IN_PAGE;
    use mmtk::util::heap::inspection::*;
    use mmtk::vm::ObjectModel;
    use std::fs::File;
    use std::io::Write;

    fn space_into_json(space: &dyn SpaceInspector) -> JsonValue {
        let mut json = JsonValue::new_object();
        json["name"] = JsonValue::String(space.space_name().to_string());
        json["policy"] = JsonValue::String(space.policy_name().to_string());
        json["used_bytes"] = JsonValue::Number((space.used_pages() << LOG_BYTES_IN_PAGE).into());
        json
    }

    fn region_into_json(space: &dyn SpaceInspector, region: &dyn RegionInspector) -> JsonValue {
        // println!("Dumping region: {} {}", region.region_type(), region.start());
        let mut json = JsonValue::new_object();
        json["type"] = JsonValue::String(region.region_type().to_string());
        json["start"] = JsonValue::String(format!("{}", region.start()));
        json["size"] = JsonValue::Number(region.size().into());

        let sub_regions = space.list_sub_regions(region);
        if sub_regions.is_empty() {
            let objects = region.list_objects();
            json["objects"] = objects
                .into_iter()
                .map(|object| {
                    let mut obj_json = JsonValue::new_object();
                    obj_json["address"] = JsonValue::String(format!("{}", object));
                    obj_json["type"] = JsonValue::String(unsafe {
                        crate::julia_scanning::get_julia_object_type(object.to_raw_address())
                    });
                    obj_json["size"] = JsonValue::Number(
                        unsafe { crate::object_model::VMObjectModel::get_current_size(object) }
                            .into(),
                    );
                    obj_json["pinned"] =
                        JsonValue::Boolean(mmtk::memory_manager::is_pinned(object));
                    obj_json
                })
                .collect::<Vec<_>>()
                .into();
        } else {
            json["regions"] = sub_regions
                .into_iter()
                .map(|sub_region| region_into_json(space, &*sub_region))
                .collect::<Vec<_>>()
                .into();
        }

        json
    }

    // Dump high-levvel space information
    {
        let mut file =
            File::create(format!("spaces_gc_{}_phase_{}.json", gc_count, phase)).unwrap();
        let mut json = JsonValue::new_array();
        SINGLETON
            .inspect_spaces()
            .iter()
            .for_each(|space| json.push(space_into_json(*space)).unwrap());
        file.write_all(json::stringify_pretty(json, 2).as_bytes())
            .unwrap();
    }

    // Dump Immix heap -- be careful we only dump one chunk at a time to avoid using too much Rust memory and get OOM
    {
        let mut file =
            File::create(format!("immix_heap_gc_{}_phase_{}.json", gc_count, phase)).unwrap();

        file.write_all(b"[\n").unwrap();

        let immix_space = SINGLETON
            .inspect_spaces()
            .into_iter()
            .find(|s| s.space_name() == "immix")
            .expect("Immix space not found");
        let chunks = immix_space.list_top_regions();
        let n_chunks = chunks.len();
        let mut i = 0;
        chunks
            .into_iter()
            .for_each(|chunk: Box<dyn RegionInspector>| {
                let json = region_into_json(immix_space, &*chunk);
                file.write_all(json::stringify_pretty(json, 2).as_bytes())
                    .unwrap();
                if i != n_chunks - 1 {
                    file.write_all(b",\n").unwrap();
                }
                i += 1;
            });
        assert!(i == n_chunks);

        file.write_all(b"]\n").unwrap();
    }
}
