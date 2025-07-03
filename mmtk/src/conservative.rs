use crate::jl_log_pinning_event;
use crate::julia_types::*;
use mmtk::memory_manager;
use mmtk::util::constants::BYTES_IN_ADDRESS;
use mmtk::util::{Address, ObjectReference};
use std::collections::HashSet;
use std::sync::Mutex;
lazy_static! {
    pub static ref CONSERVATIVE_ROOTS: Mutex<HashSet<ObjectReference>> = Mutex::new(HashSet::new());
}
pub fn pin_conservative_roots() {
    crate::early_return_for_non_moving_build!(());
    crate::early_return_for_current_gc!();

    let mut roots = CONSERVATIVE_ROOTS.lock().unwrap();
    let n_roots = roots.len();
    roots.retain(|obj| {
        unsafe {
            let obj = (*obj).to_raw_address();
            jl_log_pinning_event(obj, std::ptr::null_mut(), 0);
        }
        mmtk::memory_manager::pin_object(*obj)
    });
    let n_pinned = roots.len();
    log::debug!("Conservative roots: {}, pinned: {}", n_roots, n_pinned);
}
pub fn unpin_conservative_roots() {
    crate::early_return_for_non_moving_build!(());
    crate::early_return_for_current_gc!();

    let mut roots = CONSERVATIVE_ROOTS.lock().unwrap();
    let n_pinned = roots.len();
    let mut n_live = 0;
    roots.drain().for_each(|obj| {
        if mmtk::memory_manager::is_live_object(obj) {
            n_live += 1;
            mmtk::memory_manager::unpin_object(obj);
        }
    });
    log::debug!(
        "Conservative roots: pinned: {}, unpinned/live {}",
        n_pinned,
        n_live
    );
}
pub fn mmtk_conservative_scan_task_stack(ta: *const jl_task_t) {
    crate::early_return_for_non_moving_build!(());
    crate::early_return_for_current_gc!();

    log::debug!("mmtk_conservative_scan_native_stack begin ta = {:?}", ta);
    let mut active_start = Address::ZERO;
    let mut active_end = Address::ZERO;
    let mut total_start = Address::ZERO;
    let mut total_end = Address::ZERO;
    unsafe {
        crate::jl_active_task_stack(
            ta,
            &mut active_start as _,
            &mut active_end as _,
            &mut total_start as _,
            &mut total_end as _,
        )
    };
    log::debug!(
        "mmtk_conservative_scan_native_stack continue, active = {},{}, total = {},{}",
        active_start,
        active_end,
        total_start,
        total_end,
    );

    let size = active_end - active_start;

    if !active_start.is_zero() {
        log::debug!("Conservatively scan the stack");
        // See jl_guard_size
        // TODO: Are we sure there are always guard pages we need to skip?
        const JL_GUARD_PAGE: usize = 4096 * 8;
        let guard_page_start = active_start + JL_GUARD_PAGE;
        log::debug!("Skip guard page: {}, {}", active_start, guard_page_start);
        conservative_scan_range(guard_page_start, active_start + size);
    } else {
        log::warn!("Skip stack for {:?}", ta);
    }
}
pub fn mmtk_conservative_scan_task_registers(ta: *const jl_task_t) {
    crate::early_return_for_non_moving_build!(());
    crate::early_return_for_current_gc!();

    let (lo, hi) = get_range(&unsafe { &*ta }.ctx);
    conservative_scan_range(lo, hi);
}
pub fn mmtk_conservative_scan_ptls_registers(ptls: &mut _jl_tls_states_t) {
    crate::early_return_for_non_moving_build!(());
    crate::early_return_for_current_gc!();

    let (lo, hi) = get_range(&((*ptls).gc_tls.ctx_at_the_time_gc_started));
    conservative_scan_range(lo, hi);
}
// TODO: This scans the entire context type, which is slower.
// We actually only need to scan registers.
fn get_range<T>(ctx: &T) -> (Address, Address) {
    let start = Address::from_ptr(ctx);
    let ty_size = std::mem::size_of::<T>();
    (start, start + ty_size)
}
fn conservative_scan_range(lo: Address, hi: Address) {
    // The high address is exclusive
    let hi = if hi.is_aligned_to(BYTES_IN_ADDRESS) {
        hi - BYTES_IN_ADDRESS
    } else {
        hi.align_down(BYTES_IN_ADDRESS)
    };
    let lo = lo.align_up(BYTES_IN_ADDRESS);
    log::trace!("Scan {} (lo) {} (hi)", lo, hi);
    let mut cursor = hi;
    while cursor >= lo {
        let addr = unsafe { cursor.load::<Address>() };
        if let Some(obj) = is_potential_mmtk_object(addr) {
            CONSERVATIVE_ROOTS.lock().unwrap().insert(obj);
        }
        cursor -= BYTES_IN_ADDRESS;
    }
}
fn is_potential_mmtk_object(addr: Address) -> Option<ObjectReference> {
    if crate::object_model::is_addr_in_immixspace(addr) {
        // We only care about immix space. If the object is in other spaces, we won't move them, and we don't need to pin them.
        memory_manager::find_object_from_internal_pointer(addr, usize::MAX)
    } else {
        None
    }
}
