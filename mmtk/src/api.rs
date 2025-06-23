// All functions here are extern function. There is no point for marking them as unsafe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
use crate::JuliaVM;
use crate::JULIA_HEADER_SIZE;
use crate::SINGLETON;
use crate::{BUILDER, DISABLED_GC, MUTATORS, USER_TRIGGERED_GC};

use libc::c_char;
use log::*;
use mmtk::memory_manager;
use mmtk::scheduler::GCWorker;
use mmtk::util::api_util::NullableObjectReference;
use mmtk::util::opaque_pointer::*;
use mmtk::util::{Address, ObjectReference, OpaquePointer};
use mmtk::AllocationSemantics;
use mmtk::Mutator;
use std::ffi::CStr;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[no_mangle]
pub extern "C" fn mmtk_gc_init(
    min_heap_size: usize,
    max_heap_size: usize,
    n_gcthreads: usize,
    header_size: usize,
    buffer_tag: usize,
) {
    unsafe {
        crate::JULIA_HEADER_SIZE = header_size;
        crate::JULIA_BUFF_TAG = buffer_tag;
    };

    {
        let mut builder = BUILDER.lock().unwrap();

        // Set plan
        use mmtk::util::options::PlanSelector;
        let force_plan = if cfg!(feature = "nogc") {
            Some(PlanSelector::NoGC)
        } else if cfg!(feature = "marksweep") {
            Some(PlanSelector::MarkSweep)
        } else if cfg!(feature = "immix") {
            Some(PlanSelector::Immix)
        } else if cfg!(feature = "stickyimmix") {
            Some(PlanSelector::StickyImmix)
        } else {
            None
        };
        if let Some(plan) = force_plan {
            builder.options.plan.set(plan);
        }

        if cfg!(feature = "immix_max_moving") {
            builder.options.immix_defrag_headroom_percent.set(50);
        }

        // Set heap size
        let success =
            // By default min and max heap size are 0, and we use the Stock GC heuristics
            if min_heap_size == 0 && max_heap_size == 0 {
                info!(
                    "Setting mmtk heap size to use Stock GC heuristics as defined in gc_trigger.rs",
                );
                builder
                    .options
                    .gc_trigger
                    .set(mmtk::util::options::GCTriggerSelector::Delegated)
            } else if min_heap_size != 0 && min_heap_size != max_heap_size{
                info!(
                    "Setting mmtk heap size to a variable size with min-max of {}-{} (in bytes)",
                    min_heap_size, max_heap_size
                );
                builder.options.gc_trigger.set(
                    mmtk::util::options::GCTriggerSelector::DynamicHeapSize(
                        min_heap_size,
                        max_heap_size,
                    ),
                )
            } else {
                info!(
                    "Setting mmtk heap size to a fixed max of {} (in bytes)",
                    max_heap_size
                );
                builder.options.gc_trigger.set(
                    mmtk::util::options::GCTriggerSelector::FixedHeapSize(max_heap_size),
                )
            };
        assert!(
            success,
            "Failed to set heap size to {}-{}",
            min_heap_size, max_heap_size
        );

        // Set using weak references
        let success = builder.options.no_reference_types.set(false);
        assert!(success, "Failed to set no_reference_types to false");

        // Set GC threads
        if n_gcthreads > 0 {
            let success = builder.options.threads.set(n_gcthreads);
            assert!(success, "Failed to set GC threads to {}", n_gcthreads);
        }
    }

    // Make sure that we haven't initialized MMTk (by accident) yet
    assert!(!crate::MMTK_INITIALIZED.load(Ordering::SeqCst));
    // Make sure we initialize MMTk here
    lazy_static::initialize(&SINGLETON);

    // Hijack the panic hook to make sure that if we crash in the GC threads, the process aborts.
    crate::set_panic_hook();

    // Assert to make sure our fastpath allocation is correct.
    {
        // If the assertion failed, check the allocation fastpath in Julia
        // - runtime fastpath: mmtk_immix_alloc_fast and mmtk_immortal_alloc_fast in julia.h
        // - compiler inserted fastpath: llvm-final-gc-lowering.cpp
        use mmtk::util::alloc::AllocatorSelector;
        let default_allocator = memory_manager::get_allocator_mapping::<JuliaVM>(
            &SINGLETON,
            AllocationSemantics::Default,
        );
        assert_eq!(default_allocator, AllocatorSelector::Immix(0));
        let immortal_allocator = memory_manager::get_allocator_mapping::<JuliaVM>(
            &SINGLETON,
            AllocationSemantics::Immortal,
        );
        assert_eq!(immortal_allocator, AllocatorSelector::BumpPointer(0));
    }

    // Assert to make sure alignment used in C is correct
    {
        // If the assertion failed, check MMTK_MIN_ALIGNMENT in julia.h
        assert_eq!(<JuliaVM as mmtk::vm::VMBinding>::MIN_ALIGNMENT, 4);
    }
}

#[no_mangle]
pub extern "C" fn mmtk_bind_mutator(tls: VMMutatorThread, tid: usize) -> *mut Mutator<JuliaVM> {
    let mutator_box = memory_manager::bind_mutator(&SINGLETON, tls);

    let res = Box::into_raw(mutator_box);

    info!("Binding mutator {:?} to thread id = {}", res, tid);
    res
}

#[no_mangle]
pub extern "C" fn mmtk_post_bind_mutator(
    mutator: *mut Mutator<JuliaVM>,
    original_box_mutator: *mut Mutator<JuliaVM>,
) {
    // We have to store the original boxed mutator. Otherwise, we may have dangling pointers in mutator.
    MUTATORS.write().unwrap().insert(
        Address::from_mut_ptr(mutator),
        Address::from_mut_ptr(original_box_mutator),
    );
}

#[no_mangle]
pub extern "C" fn mmtk_destroy_mutator(mutator: *mut Mutator<JuliaVM>) {
    // destroy the mutator with MMTk.
    memory_manager::destroy_mutator(unsafe { &mut *mutator });

    let mut mutators = MUTATORS.write().unwrap();
    let key = Address::from_mut_ptr(mutator);

    // Clear the original boxed mutator
    let orig_mutator = mutators.get(&key).unwrap();
    let _ = unsafe { Box::from_raw(orig_mutator.to_mut_ptr::<Mutator<JuliaVM>>()) };

    // Remove from our hashmap
    mutators.remove(&key);
}

#[no_mangle]
pub extern "C" fn mmtk_alloc(
    mutator: *mut Mutator<JuliaVM>,
    size: usize,
    align: usize,
    offset: usize,
    semantics: AllocationSemantics,
) -> Address {
    debug_assert!(
        mmtk::util::conversions::raw_is_aligned(
            size,
            <JuliaVM as mmtk::vm::VMBinding>::MIN_ALIGNMENT
        ),
        "Alloc size {} is not aligned to min alignment",
        size
    );
    memory_manager::alloc::<JuliaVM>(unsafe { &mut *mutator }, size, align, offset, semantics)
}

#[no_mangle]
pub extern "C" fn mmtk_alloc_large(
    mutator: *mut Mutator<JuliaVM>,
    size: usize,
    align: usize,
    offset: usize,
) -> Address {
    memory_manager::alloc::<JuliaVM>(
        unsafe { &mut *mutator },
        size,
        align,
        offset,
        AllocationSemantics::Los,
    )
}

#[no_mangle]
pub extern "C" fn mmtk_post_alloc(
    mutator: *mut Mutator<JuliaVM>,
    refer: ObjectReference,
    bytes: usize,
    semantics: AllocationSemantics,
) {
    memory_manager::post_alloc::<JuliaVM>(unsafe { &mut *mutator }, refer, bytes, semantics)
}

#[no_mangle]
pub extern "C" fn mmtk_will_never_move(object: ObjectReference) -> bool {
    !object.is_movable()
}

#[no_mangle]
pub extern "C" fn mmtk_start_worker(tls: VMWorkerThread, worker: *mut GCWorker<JuliaVM>) {
    let worker = unsafe { Box::from_raw(worker) };
    memory_manager::start_worker::<JuliaVM>(&SINGLETON, tls, worker)
}

#[no_mangle]
pub extern "C" fn mmtk_initialize_collection(tls: VMThread) {
    memory_manager::initialize_collection(&SINGLETON, tls);
}

#[no_mangle]
pub extern "C" fn mmtk_used_bytes() -> usize {
    memory_manager::used_bytes(&SINGLETON)
}

#[no_mangle]
pub extern "C" fn mmtk_free_bytes() -> usize {
    memory_manager::free_bytes(&SINGLETON)
}

#[no_mangle]
pub extern "C" fn mmtk_total_bytes() -> usize {
    memory_manager::total_bytes(&SINGLETON)
}

#[no_mangle]
pub extern "C" fn mmtk_is_reachable_object(object: ObjectReference) -> bool {
    object.is_reachable()
}

#[no_mangle]
pub extern "C" fn mmtk_is_live_object(object: ObjectReference) -> bool {
    object.is_live()
}

#[no_mangle]
pub extern "C" fn mmtk_is_mapped_address(address: Address) -> bool {
    address.is_mapped()
}

#[no_mangle]
pub extern "C" fn mmtk_handle_user_collection_request(tls: VMMutatorThread, collection: u8) {
    AtomicIsize::fetch_add(&USER_TRIGGERED_GC, 1, Ordering::SeqCst);
    if AtomicBool::load(&DISABLED_GC, Ordering::SeqCst) {
        AtomicIsize::fetch_add(&USER_TRIGGERED_GC, -1, Ordering::SeqCst);
        return;
    }
    // See jl_gc_collection_t
    match collection {
        // auto
        0 => memory_manager::handle_user_collection_request::<JuliaVM>(&SINGLETON, tls),
        // full
        1 => SINGLETON.handle_user_collection_request(tls, true, true),
        // incremental
        2 => SINGLETON.handle_user_collection_request(tls, true, false),
        _ => unreachable!(),
    };
}

#[no_mangle]
pub extern "C" fn mmtk_add_weak_candidate(reff: ObjectReference) {
    memory_manager::add_weak_candidate(&SINGLETON, reff)
}

#[no_mangle]
pub extern "C" fn mmtk_add_soft_candidate(reff: ObjectReference) {
    memory_manager::add_soft_candidate(&SINGLETON, reff)
}

#[no_mangle]
pub extern "C" fn mmtk_add_phantom_candidate(reff: ObjectReference) {
    memory_manager::add_phantom_candidate(&SINGLETON, reff)
}

#[no_mangle]
pub extern "C" fn mmtk_harness_begin(tls: VMMutatorThread) {
    memory_manager::harness_begin(&SINGLETON, tls)
}

#[no_mangle]
pub extern "C" fn mmtk_harness_end(_tls: OpaquePointer) {
    memory_manager::harness_end(&SINGLETON)
}

#[no_mangle]
pub extern "C" fn mmtk_process(name: *const c_char, value: *const c_char) -> bool {
    let name_str: &CStr = unsafe { CStr::from_ptr(name) };
    let value_str: &CStr = unsafe { CStr::from_ptr(value) };
    let mut builder = BUILDER.lock().unwrap();
    memory_manager::process(
        &mut builder,
        name_str.to_str().unwrap(),
        value_str.to_str().unwrap(),
    )
}

#[no_mangle]
pub extern "C" fn mmtk_starting_heap_address() -> Address {
    memory_manager::starting_heap_address()
}

#[no_mangle]
pub extern "C" fn mmtk_last_heap_address() -> Address {
    memory_manager::last_heap_address()
}

// Accessed from C to count the bytes we allocated with jl_gc_counted_malloc etc.
#[no_mangle]
pub static JULIA_MALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn mmtk_gc_poll(tls: VMMutatorThread) {
    memory_manager::gc_poll(&SINGLETON, tls);
}

#[no_mangle]
pub extern "C" fn mmtk_runtime_panic() {
    panic!("Panicking at runtime!")
}

#[no_mangle]
pub extern "C" fn mmtk_unreachable() {
    unreachable!()
}

#[no_mangle]
#[allow(mutable_transmutes)]
pub extern "C" fn mmtk_set_vm_space(start: Address, size: usize) {
    let mmtk: &mmtk::MMTK<JuliaVM> = &SINGLETON;
    let mmtk_mut: &mut mmtk::MMTK<JuliaVM> = unsafe { std::mem::transmute(mmtk) };
    memory_manager::set_vm_space(mmtk_mut, start, size);

    #[cfg(feature = "stickyimmix")]
    set_side_log_bit_for_region(start, size);
    set_side_vo_bit_for_region(start, size);
}

#[no_mangle]
pub extern "C" fn mmtk_memory_region_copy(
    mutator: *mut Mutator<JuliaVM>,
    src_obj: ObjectReference,
    src_addr: Address,
    dst_obj: ObjectReference,
    dst_addr: Address,
    count: usize,
) {
    use crate::slots::JuliaMemorySlice;
    let src = JuliaMemorySlice {
        owner: src_obj,
        start: src_addr,
        count,
    };
    let dst = JuliaMemorySlice {
        owner: dst_obj,
        start: dst_addr,
        count,
    };
    let mutator = unsafe { &mut *mutator };
    memory_manager::memory_region_copy(mutator, src, dst);
}

#[no_mangle]
#[allow(unused_variables)] // Args are only used for sticky immix.
pub extern "C" fn mmtk_immortal_region_post_alloc(start: Address, size: usize) {
    #[cfg(feature = "stickyimmix")]
    set_side_log_bit_for_region(start, size);

    set_side_vo_bit_for_region(start, size);
}

#[cfg(feature = "stickyimmix")]
fn set_side_log_bit_for_region(start: Address, size: usize) {
    debug!("Bulk set {} to {} ({} bytes)", start, start + size, size);
    use crate::mmtk::vm::ObjectModel;
    match <JuliaVM as mmtk::vm::VMBinding>::VMObjectModel::GLOBAL_LOG_BIT_SPEC.as_spec() {
        mmtk::util::metadata::MetadataSpec::OnSide(side) => side.bset_metadata(start, size),
        _ => unimplemented!(),
    }
}

// We have to set VO bit even if this is a non_moving build. Otherwise, assertions in mmtk-core
// will complain about seeing objects without VO bit.
fn set_side_vo_bit_for_region(start: Address, size: usize) {
    debug!(
        "Bulk set VO bit {} to {} ({} bytes)",
        start,
        start + size,
        size
    );

    mmtk::util::metadata::vo_bit::VO_BIT_SIDE_METADATA_SPEC.bset_metadata(start, size);
}

#[no_mangle]
pub extern "C" fn mmtk_object_reference_write_post(
    mutator: *mut Mutator<JuliaVM>,
    src: ObjectReference,
    target: NullableObjectReference,
) {
    let mutator = unsafe { &mut *mutator };
    memory_manager::object_reference_write_post(
        mutator,
        src,
        crate::slots::JuliaVMSlot::Simple(mmtk::vm::slot::SimpleSlot::from_address(Address::ZERO)),
        target.into(),
    )
}

#[no_mangle]
pub extern "C" fn mmtk_object_reference_write_slow(
    mutator: &'static mut Mutator<JuliaVM>,
    src: ObjectReference,
    target: NullableObjectReference,
) {
    use mmtk::MutatorContext;
    mutator.barrier().object_reference_write_slow(
        src,
        crate::slots::JuliaVMSlot::Simple(mmtk::vm::slot::SimpleSlot::from_address(Address::ZERO)),
        target.into(),
    );
}

/// Side log bit is the first side metadata spec starting.
#[no_mangle]
pub static MMTK_SIDE_LOG_BIT_BASE_ADDRESS: Address =
    mmtk::util::metadata::side_metadata::GLOBAL_SIDE_METADATA_VM_BASE_ADDRESS;

/// VO bit base address
#[no_mangle]
pub static MMTK_SIDE_VO_BIT_BASE_ADDRESS: Address =
    mmtk::util::metadata::side_metadata::VO_BIT_SIDE_METADATA_ADDR;

#[no_mangle]
pub extern "C" fn mmtk_object_is_managed_by_mmtk(addr: usize) -> bool {
    crate::api::mmtk_is_mapped_address(unsafe { Address::from_usize(addr) })
}

#[no_mangle]
pub extern "C" fn mmtk_start_spawned_worker_thread(
    tls: VMWorkerThread,
    ctx: *mut GCWorker<JuliaVM>,
) {
    mmtk_start_worker(tls, ctx);
}

#[inline(always)]
pub fn store_obj_size(obj: ObjectReference, size: usize) {
    let addr_size = obj.to_raw_address() - 16;
    unsafe {
        addr_size.store::<u64>(size as u64);
    }
}

#[no_mangle]
pub extern "C" fn mmtk_store_obj_size_c(obj: ObjectReference, size: usize) {
    let addr_size = obj.to_raw_address() - 16;
    unsafe {
        addr_size.store::<u64>(size as u64);
    }
}

#[no_mangle]
pub extern "C" fn mmtk_get_obj_size(obj: ObjectReference) -> usize {
    unsafe {
        let addr_size = obj.to_raw_address() - 2 * JULIA_HEADER_SIZE;
        addr_size.load::<u64>() as usize
    }
}

#[allow(unused_variables)]
fn assert_is_object(object: ObjectReference) {
    // The checks are quite expensive. Dont run it in normal builds.
    const ASSERT_OBJECT: bool = false;
    if ASSERT_OBJECT {
        #[cfg(debug_assertions)]
        {
            use crate::object_model::{is_object_in_immixspace, is_object_in_los};
            if !mmtk_object_is_managed_by_mmtk(object.to_raw_address().as_usize()) {
                panic!("{} is not managed by MMTk", object);
            }
            if !is_object_in_immixspace(&object) && !is_object_in_los(&object) {
                // We will use VO bit in the following check. But if the object is not in immix space or LOS, we cannot do the check.
                return;
            }
            if !object
                .to_raw_address()
                .is_aligned_to(ObjectReference::ALIGNMENT)
            {
                panic!(
                    "{} is not aligned, it cannot be an object reference",
                    object
                )
            }
            if memory_manager::is_mmtk_object(object.to_raw_address()).is_none() {
                error!("{} is not an object", object);
                if let Some(base_ref) = memory_manager::find_object_from_internal_pointer(
                    object.to_raw_address(),
                    usize::MAX,
                ) {
                    panic!("{} is an internal pointer of {}", object, base_ref);
                } else {
                    panic!(
                        "{} is not recognised as an object reference, or an internal reference",
                        object
                    );
                }
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn mmtk_pin_object(object: ObjectReference) -> bool {
    assert_is_object(object);
    crate::early_return_for_non_moving_build!(false);
    memory_manager::pin_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_unpin_object(object: ObjectReference) -> bool {
    assert_is_object(object);
    crate::early_return_for_non_moving_build!(false);
    memory_manager::unpin_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_is_object_pinned(object: ObjectReference) -> bool {
    assert_is_object(object);
    crate::early_return_for_non_moving_build!(false);

    memory_manager::is_pinned(object)
}

macro_rules! handle_potential_internal_pointer {
    ($func: path, $addr: expr) => {{
        if $addr.is_aligned_to(ObjectReference::ALIGNMENT) {
            if let Some(obj) = memory_manager::is_mmtk_object($addr) {
                return $func(obj);
            }
        }
        let maybe_objref = memory_manager::find_object_from_internal_pointer($addr, usize::MAX);
        if let Some(obj) = maybe_objref {
            trace!(
                "Attempt to pin {:?}, but it is an internal reference of {:?}",
                $addr,
                obj
            );
            $func(obj)
        } else {
            warn!(
                "Attempt to pin {:?}, but it is not recognised as a object",
                $addr
            );
            false
        }
    }};
}

#[no_mangle]
pub extern "C" fn mmtk_pin_pointer(addr: Address) -> bool {
    crate::early_return_for_non_moving_build!(false);

    if crate::object_model::is_addr_in_immixspace(addr) {
        handle_potential_internal_pointer!(memory_manager::pin_object, addr)
    } else {
        debug!("Object is not in Immix space. MMTk will not move the object. No need to pin the object.");
        false
    }
}

#[no_mangle]
pub extern "C" fn mmtk_unpin_pointer(addr: Address) -> bool {
    crate::early_return_for_non_moving_build!(false);

    if crate::object_model::is_addr_in_immixspace(addr) {
        handle_potential_internal_pointer!(memory_manager::unpin_object, addr)
    } else {
        debug!("Object is not in Immix space. MMTk will not move the object. No need to unpin the object.");
        false
    }
}

#[no_mangle]
pub extern "C" fn mmtk_is_pointer_pinned(addr: Address) -> bool {
    crate::early_return_for_non_moving_build!(false);

    if crate::object_model::is_addr_in_immixspace(addr) {
        handle_potential_internal_pointer!(memory_manager::is_pinned, addr)
    } else if mmtk_object_is_managed_by_mmtk(addr.as_usize()) {
        debug!(
            "Object is not in Immix space. MMTk will not move the object. We assume it is pinned."
        );
        true
    } else {
        debug!("Object is not managed by mmtk - checking pinning state via this function isn't supported.");
        false
    }
}

#[no_mangle]
pub extern "C" fn get_mmtk_version() -> *const c_char {
    crate::build_info::MMTK_JULIA_FULL_VERSION_STRING
        .as_c_str()
        .as_ptr() as _
}

// #[cfg(feature = "dump_memory_stats")]
#[no_mangle]
pub extern "C" fn print_fragmentation() {
    let map = memory_manager::live_bytes_in_last_gc(&SINGLETON);
    for (space, stats) in map {
        println!(
            "Utilization in space {:?}: {} live bytes, {} total bytes, {:.2} %",
            space,
            stats.live_bytes,
            stats.used_bytes,
            (stats.live_bytes as f64 / stats.used_bytes as f64) * 100.0
        );
    }

    // SINGLETON.get_plan().dump_memory_stats();
}
