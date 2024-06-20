#[cfg(not(feature = "non_moving"))]
use crate::api::mmtk_is_pinned;
use crate::api::mmtk_object_is_managed_by_mmtk;
use crate::julia_types::*;
use crate::object_model::mmtk_jl_array_ndims;
use crate::slots::JuliaVMSlot;
use crate::slots::OffsetSlot;
use crate::JULIA_BUFF_TAG;
use crate::UPCALLS;
use memoffset::offset_of;
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::slot::SimpleSlot;
use mmtk::vm::SlotVisitor;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const OFFSET_OF_INLINED_SPACE_IN_MODULE: usize =
    offset_of!(mmtk_jl_module_t, usings) + offset_of!(mmtk_arraylist_t, _space);

extern "C" {
    pub static jl_simplevector_type: *const mmtk_jl_datatype_t;
    pub static jl_array_typename: *mut mmtk_jl_typename_t;
    pub static jl_module_type: *const mmtk_jl_datatype_t;
    pub static jl_task_type: *const mmtk_jl_datatype_t;
    pub static jl_string_type: *const mmtk_jl_datatype_t;
    pub static jl_weakref_type: *const mmtk_jl_datatype_t;
    pub static jl_symbol_type: *const mmtk_jl_datatype_t;
    pub static jl_method_type: *const mmtk_jl_datatype_t;
    pub static jl_uniontype_type: *const mmtk_jl_datatype_t;
}

const HT_NOTFOUND: usize = 1;

#[inline(always)]
pub unsafe fn mmtk_jl_typeof(addr: Address) -> *const mmtk_jl_datatype_t {
    let as_tagged_value =
        addr.as_usize() - std::mem::size_of::<crate::julia_scanning::mmtk_jl_taggedvalue_t>();
    let t_header = Address::from_usize(as_tagged_value).load::<Address>();
    let t = t_header.as_usize() & !15;

    Address::from_usize(t).to_ptr::<mmtk_jl_datatype_t>()
}

const PRINT_OBJ_TYPE: bool = false;

// This function is a rewrite of `gc_mark_outrefs()` in `gc.c`
// INFO: *_custom() functions are acessors to bitfields that do not use bindgen generated code.
#[inline(always)]
pub unsafe fn scan_julia_object<SV: SlotVisitor<JuliaVMSlot>>(obj: Address, closure: &mut SV) {
    // get Julia object type
    let vt = mmtk_jl_typeof(obj);

    // We don't scan buffers, as they will be scanned as a part of its parent object.
    // But when a jl_binding_t buffer is inserted into remset, they have be to scanned.
    // The gc bits (tag), which is set in the write barrier, tells us if the buffer is in the remset.
    if vt as usize == JULIA_BUFF_TAG {
        let as_tagged_value =
            obj.as_usize() - std::mem::size_of::<crate::julia_scanning::mmtk_jl_taggedvalue_t>();
        let t_header = Address::from_usize(as_tagged_value).load::<Address>();
        let tag = t_header.as_usize() & 3;
        if tag == 2 {
            // buf is binding
            let b = obj.to_ptr::<mmtk_jl_binding_t>();
            let value = ::std::ptr::addr_of!((*b).value);
            let globalref = ::std::ptr::addr_of!((*b).globalref);
            let ty = ::std::ptr::addr_of!((*b).ty);

            process_slot(closure, Address::from_usize(value as usize));
            process_slot(closure, Address::from_usize(globalref as usize));
            process_slot(closure, Address::from_usize(ty as usize));
            // clearing tag bits
            Address::from_usize(as_tagged_value).store::<usize>(t_header.as_usize() & !3);
            return;
        } else {
            return; // do not scan buffers
        }
    }

    if vt == jl_symbol_type {
        return;
    }

    if vt == jl_simplevector_type {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: simple vector\n", obj);
        }

        let length = mmtk_jl_svec_len(obj);
        let mut objary_begin = mmtk_jl_svec_data(obj);
        let objary_end = objary_begin.shift::<Address>(length as isize);

        while objary_begin < objary_end {
            process_slot(closure, objary_begin);
            objary_begin = objary_begin.shift::<Address>(1);
        }
    } else if (*vt).name == jl_array_typename {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: array\n", obj);
        }

        let array = obj.to_ptr::<mmtk_jl_array_t>();
        let flags = (*array).flags;

        if flags.how_custom() == 0 {
            // data is inlined, or a foreign pointer we don't manage
            // if data is inlined (i.e. it is an internal pointer) and the array moves,
            // a->data is currently updated when copying the array since there may be other hidden
            // fields before the inlined data affecting the offset in which a->data points to
            // see jl_array_t in julia.h
        } else if flags.how_custom() == 1 {
            // julia-allocated buffer that needs to be marked
            let offset = (*array).offset as usize * (*array).elsize as usize;
            let data_addr = ::std::ptr::addr_of!((*array).data);
            process_offset_slot(closure, Address::from_ptr(data_addr), offset);
        } else if flags.how_custom() == 2 {
            // malloc-allocated pointer this array object manages
            // should be processed below if it contains pointers
        } else if flags.how_custom() == 3 {
            // has a pointer to the object that owns the data
            let owner_addr = mmtk_jl_array_data_owner_addr(array);

            // to avoid having to update a->data, which requires introspecting the owner object
            // we simply expect that both owner and buffers are pinned when in a moving GC
            #[cfg(not(feature = "non_moving"))]
            debug_assert!(
                (mmtk_object_is_managed_by_mmtk(owner_addr.load())
                    && mmtk_is_pinned(owner_addr.load())
                    || !(mmtk_object_is_managed_by_mmtk(owner_addr.load()))),
                "Owner ({:?}) may move (is_pinned = {}), a->data may become outdated!",
                owner_addr.load::<ObjectReference>(),
                mmtk_is_pinned(owner_addr.load())
            );

            process_slot(closure, owner_addr);
            return;
        }

        if (*array).data == std::ptr::null_mut() || mmtk_jl_array_len(array) == 0 {
            return;
        }

        if flags.ptrarray_custom() != 0 {
            if mmtk_jl_tparam0(vt) == jl_symbol_type {
                return;
            }

            let length = mmtk_jl_array_len(array);

            let mut objary_begin = Address::from_ptr((*array).data);
            let objary_end = objary_begin.shift::<Address>(length as isize);

            while objary_begin < objary_end {
                process_slot(closure, objary_begin);
                objary_begin = objary_begin.shift::<Address>(1);
            }
        } else if flags.hasptr_custom() != 0 {
            let et = mmtk_jl_tparam0(vt);
            let layout = (*et).layout;
            let npointers = (*layout).npointers;
            let elsize = (*array).elsize as usize / std::mem::size_of::<Address>();
            let length = mmtk_jl_array_len(array);
            let mut objary_begin = Address::from_ptr((*array).data);
            let objary_end = objary_begin.shift::<Address>((length * elsize) as isize);

            if npointers == 1 {
                objary_begin = objary_begin.shift::<Address>((*layout).first_ptr as isize);
                while objary_begin < objary_end {
                    process_slot(closure, objary_begin);
                    objary_begin = objary_begin.shift::<Address>(elsize as isize);
                }
            } else if (*layout).fielddesc_type() == 0 {
                let obj8_begin = mmtk_jl_dt_layout_ptrs(layout);
                let obj8_end = obj8_begin.shift::<u8>(npointers as isize);
                let mut elem_begin = obj8_begin;
                let elem_end = obj8_end;

                while objary_begin < objary_end {
                    while elem_begin < elem_end {
                        let elem_begin_loaded = elem_begin.load::<u8>();
                        let slot = objary_begin.shift::<Address>(elem_begin_loaded as isize);
                        process_slot(closure, slot);
                        elem_begin = elem_begin.shift::<u8>(1);
                    }
                    elem_begin = obj8_begin;
                    objary_begin = objary_begin.shift::<Address>(elsize as isize);
                }
            } else if (*layout).fielddesc_type() == 1 {
                let mut obj16_begin = mmtk_jl_dt_layout_ptrs(layout);
                let obj16_end = obj16_begin.shift::<u16>(npointers as isize);

                while objary_begin < objary_end {
                    while obj16_begin < obj16_end {
                        let elem_begin_loaded = obj16_begin.load::<u16>();
                        let slot = objary_begin.shift::<Address>(elem_begin_loaded as isize);
                        process_slot(closure, slot);
                        obj16_begin = obj16_begin.shift::<u16>(1);
                    }
                    obj16_begin = mmtk_jl_dt_layout_ptrs(layout);
                    objary_begin = objary_begin.shift::<Address>(elsize as isize);
                }
            } else {
                unimplemented!();
            }
        } else {
            return;
        }
    } else if vt == jl_module_type {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: module\n", obj);
        }

        let m = obj.to_ptr::<mmtk_jl_module_t>();
        let bsize = (*m).bindings.size;
        let mut begin =
            Address::from_mut_ptr((*m).bindings.table) + std::mem::size_of::<Address>() as usize;
        let end = Address::from_mut_ptr((*m).bindings.table)
            + bsize as usize * std::mem::size_of::<Address>();

        while begin < end {
            let b = begin.load::<*mut mmtk_jl_binding_t>();

            if b as usize == HT_NOTFOUND {
                begin = begin.shift::<Address>(2);
                continue;
            }
            if PRINT_OBJ_TYPE {
                println!(" - scan table: {}\n", obj);
            }

            if (b as usize) != 0 {
                process_slot(closure, begin);
            }

            let value = ::std::ptr::addr_of!((*b).value);
            let globalref = ::std::ptr::addr_of!((*b).globalref);
            let ty = ::std::ptr::addr_of!((*b).ty);

            process_slot(closure, Address::from_usize(value as usize));
            process_slot(closure, Address::from_usize(globalref as usize));
            process_slot(closure, Address::from_usize(ty as usize));
            begin = begin.shift::<Address>(2);
        }

        let parent_slot = ::std::ptr::addr_of!((*m).parent);
        if PRINT_OBJ_TYPE {
            println!(" - scan parent: {:?}\n", parent_slot);
        }
        process_slot(closure, Address::from_ptr(parent_slot));

        // m.usings.items may be inlined in the module when the array list size <= AL_N_INLINE (cf. arraylist_new)
        // In that case it may be an mmtk object and not a malloced address.
        // If it is an mmtk object, (*m).usings.items will then be an internal pointer to the module
        // which means we will need to trace and update it if the module moves
        if mmtk_object_is_managed_by_mmtk((*m).usings.items as usize) {
            let offset = OFFSET_OF_INLINED_SPACE_IN_MODULE;
            let slot = Address::from_ptr(::std::ptr::addr_of!((*m).usings.items));
            process_offset_slot(closure, slot, offset);
        }

        let nusings = (*m).usings.len;
        if nusings != 0 {
            let mut objary_begin = Address::from_mut_ptr((*m).usings.items);
            let objary_end = objary_begin.shift::<Address>(nusings as isize);

            while objary_begin < objary_end {
                if PRINT_OBJ_TYPE {
                    println!(" - scan usings: {:?}\n", objary_begin);
                }
                process_slot(closure, objary_begin);
                objary_begin = objary_begin.shift::<Address>(1);
            }
        }
    } else if vt == jl_task_type {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: task\n", obj);
        }

        let ta = obj.to_ptr::<mmtk_jl_task_t>();

        // transitively pinnig of stack roots happens during root
        // processing so it's fine to have only one closure here
        mmtk_scan_gcstack(ta, closure, None);

        let layout = (*jl_task_type).layout;
        debug_assert!((*layout).fielddesc_type() == 0);
        debug_assert!((*layout).nfields > 0);
        let npointers = (*layout).npointers;
        let mut obj8_begin = mmtk_jl_dt_layout_ptrs(layout);
        let obj8_end = obj8_begin.shift::<u8>(npointers as isize);

        while obj8_begin < obj8_end {
            let obj8_begin_loaded = obj8_begin.load::<u8>();
            let slot = obj.shift::<Address>(obj8_begin_loaded as isize);
            process_slot(closure, slot);
            obj8_begin = obj8_begin.shift::<u8>(1);
        }
    } else if vt == jl_string_type {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: string\n", obj);
        }
        return;
    } else {
        if PRINT_OBJ_TYPE {
            println!("scan_julia_obj {}: datatype\n", obj);
        }

        if vt == jl_weakref_type {
            return;
        }

        let layout = (*vt).layout;
        let npointers = (*layout).npointers;
        if npointers == 0 {
            return;
        } else {
            debug_assert!(
                (*layout).nfields > 0 && (*layout).fielddesc_type() != 3,
                "opaque types should have been handled specially"
            );
            if (*layout).fielddesc_type() == 0 {
                let mut obj8_begin = mmtk_jl_dt_layout_ptrs(layout);
                let obj8_end = obj8_begin.shift::<u8>(npointers as isize);

                while obj8_begin < obj8_end {
                    let obj8_begin_loaded = obj8_begin.load::<u8>();
                    let slot = obj.shift::<Address>(obj8_begin_loaded as isize);
                    process_slot(closure, slot);
                    obj8_begin = obj8_begin.shift::<u8>(1);
                }
            } else if (*layout).fielddesc_type() == 1 {
                let mut obj16_begin = mmtk_jl_dt_layout_ptrs(layout);
                let obj16_end = obj16_begin.shift::<u16>(npointers as isize);

                while obj16_begin < obj16_end {
                    let obj16_begin_loaded = obj16_begin.load::<u16>();
                    let slot = obj.shift::<Address>(obj16_begin_loaded as isize);
                    process_slot(closure, slot);
                    obj16_begin = obj16_begin.shift::<u16>(1);
                }
            } else if (*layout).fielddesc_type() == 2 {
                let mut obj32_begin = mmtk_jl_dt_layout_ptrs(layout);
                let obj32_end = obj32_begin.shift::<u32>(npointers as isize);

                while obj32_begin < obj32_end {
                    let obj32_begin_loaded = obj32_begin.load::<u32>();
                    let slot = obj.shift::<Address>(obj32_begin_loaded as isize);
                    process_slot(closure, slot);
                    obj32_begin = obj32_begin.shift::<u32>(1);
                }
            } else {
                debug_assert!((*layout).fielddesc_type() == 3);
                unimplemented!();
            }
        }
    }
}

pub unsafe fn mmtk_scan_gcpreserve_stack<'a, EV: SlotVisitor<JuliaVMSlot>>(
    ta: *const mmtk_jl_task_t,
    closure: &'a mut EV,
) {
    // process transitively pinning stack
    let mut s = (*ta).gcpreserve_stack;
    let (offset, lb, ub) = (0 as isize, 0 as u64, u64::MAX);

    if s != std::ptr::null_mut() {
        let s_nroots_addr = ::std::ptr::addr_of!((*s).nroots);
        let mut nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
        debug_assert!(nroots.as_usize() as u32 <= UINT32_MAX);
        let mut nr = nroots >> 3;

        loop {
            let rts = Address::from_mut_ptr(s).shift::<Address>(2);
            let mut i = 0;

            while i < nr {
                let real_addr = get_stack_addr(rts.shift::<Address>(i as isize), offset, lb, ub);

                let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                use crate::julia_finalizer::gc_ptr_tag;
                // malloced pointer tagged in jl_gc_add_quiescent
                // skip both the next element (native function), and the object
                if slot & 3usize == 3 {
                    i += 2;
                    continue;
                }

                // pointer is not malloced but function is native, so skip it
                if gc_ptr_tag(slot, 1) {
                    i += 2;
                    continue;
                }

                process_slot(closure, real_addr);
                i += 1;
            }

            let s_prev_address = ::std::ptr::addr_of!((*s).prev);
            let sprev = read_stack(Address::from_ptr(s_prev_address), offset, lb, ub);
            if sprev.is_zero() {
                break;
            }

            s = sprev.to_mut_ptr::<mmtk_jl_gcframe_t>();
            let s_nroots_addr = ::std::ptr::addr_of!((*s).nroots);
            let new_nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
            nroots = new_nroots;
            nr = nroots >> 3;
            continue;
        }
    }
}

pub unsafe fn mmtk_scan_gcstack<'a, EV: SlotVisitor<JuliaVMSlot>>(
    ta: *const mmtk_jl_task_t,
    closure: &'a mut EV,
    pclosure: Option<&'a mut EV>,
) {
    // process Julia's standard shadow (GC) stack
    let stkbuf = (*ta).stkbuf;
    let copy_stack = (*ta).copy_stack_custom();

    #[cfg(feature = "julia_copy_stack")]
    if stkbuf != std::ptr::null_mut() && copy_stack != 0 {
        let stkbuf_slot = Address::from_ptr(::std::ptr::addr_of!((*ta).stkbuf));
        process_slot(closure, stkbuf_slot);
    }

    let (mut offset, mut lb, mut ub) = (0 as isize, 0 as u64, u64::MAX);

    #[cfg(feature = "julia_copy_stack")]
    if stkbuf != std::ptr::null_mut() && copy_stack != 0 && (*ta).ptls == std::ptr::null_mut() {
        if ((*ta).tid as i16) < 0 {
            panic!("tid must be positive.")
        }
        let stackbase = ((*UPCALLS).get_stackbase)((*ta).tid);
        ub = stackbase as u64;
        lb = ub - ((*ta).copy_stack() as u64);
        offset = (*ta).stkbuf as isize - lb as isize;
    }

    // process Julia's gc shadow stack
    scan_stack((*ta).gcstack, lb, ub, offset, closure, pclosure);

    // just call into C, since the code is cold
    if (*ta).excstack != std::ptr::null_mut() {
        ((*UPCALLS).scan_julia_exc_obj)(
            Address::from_ptr(ta),
            Address::from_mut_ptr(closure),
            process_slot::<EV> as _,
        );
    }
}

#[inline(always)]
unsafe fn scan_stack<'a, EV: SlotVisitor<JuliaVMSlot>>(
    mut stack: *mut mmtk__jl_gcframe_t,
    lb: u64,
    ub: u64,
    offset: isize,
    mut closure: &'a mut EV,
    mut pclosure: Option<&'a mut EV>,
) {
    if stack != std::ptr::null_mut() {
        let s_nroots_addr = ::std::ptr::addr_of!((*stack).nroots);
        let mut nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
        debug_assert!(nroots.as_usize() as u32 <= UINT32_MAX);
        let mut nr = nroots >> 3;

        loop {
            // if the 'pin' bit on the root type is not set, must transitively pin
            // and therefore use transitive pinning closure
            let closure_to_use: &mut &mut EV = if (nroots.as_usize() & 4) == 0 {
                &mut closure
            } else {
                // otherwise, use the pinning closure (if available)
                match &mut pclosure {
                    Some(c) => c,
                    None => &mut closure,
                }
            };

            let rts = Address::from_mut_ptr(stack).shift::<Address>(2);
            let mut i = 0;
            while i < nr {
                if (nroots.as_usize() & 1) != 0 {
                    let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                    let real_addr = get_stack_addr(slot, offset, lb, ub);
                    process_slot(*closure_to_use, real_addr);
                } else {
                    let real_addr =
                        get_stack_addr(rts.shift::<Address>(i as isize), offset, lb, ub);

                    let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                    use crate::julia_finalizer::gc_ptr_tag;
                    // malloced pointer tagged in jl_gc_add_quiescent
                    // skip both the next element (native function), and the object
                    if slot & 3usize == 3 {
                        i += 2;
                        continue;
                    }

                    // pointer is not malloced but function is native, so skip it
                    if gc_ptr_tag(slot, 1) {
                        process_offset_slot(*closure_to_use, real_addr, 1);
                        i += 2;
                        continue;
                    }

                    process_slot(*closure_to_use, real_addr);
                }

                i += 1;
            }

            let s_prev_address = ::std::ptr::addr_of!((*stack).prev);
            let sprev = read_stack(Address::from_ptr(s_prev_address), offset, lb, ub);
            if sprev.is_zero() {
                break;
            }

            stack = sprev.to_mut_ptr::<mmtk_jl_gcframe_t>();
            let s_nroots_addr = ::std::ptr::addr_of!((*stack).nroots);
            let new_nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
            nroots = new_nroots;
            nr = nroots >> 3;
            continue;
        }
    }
}

#[inline(always)]
pub unsafe fn read_stack(addr: Address, offset: isize, lb: u64, ub: u64) -> Address {
    let real_addr = get_stack_addr(addr, offset, lb, ub);

    real_addr.load::<Address>()
}

#[inline(always)]
pub fn get_stack_addr(addr: Address, offset: isize, lb: u64, ub: u64) -> Address {
    if addr.as_usize() >= lb as usize && addr.as_usize() < ub as usize {
        return addr + offset;
    } else {
        return addr;
    }
}

#[inline(always)]
pub fn process_slot<EV: SlotVisitor<JuliaVMSlot>>(closure: &mut EV, slot: Address) {
    let simple_slot = SimpleSlot::from_address(slot);

    #[cfg(debug_assertions)]
    {
        use crate::JuliaVM;
        use mmtk::vm::slot::Slot;

        if let Some(objref) = simple_slot.load() {
            debug_assert!(
                mmtk::memory_manager::is_in_mmtk_spaces::<JuliaVM>(objref),
                "Object {:?} in slot {:?} is not mapped address",
                objref,
                simple_slot
            );

            let raw_addr_usize = objref.to_raw_address().as_usize();

            // captures wrong slots before creating the work
            debug_assert!(
                raw_addr_usize % 16 == 0 || raw_addr_usize % 8 == 0,
                "Object {:?} in slot {:?} is not aligned to 8 or 16",
                objref,
                simple_slot
            );
        }
    }

    closure.visit_slot(JuliaVMSlot::Simple(simple_slot));
}

// #[inline(always)]
// pub unsafe fn boot_image_object_has_been_scanned(obj: Address) -> u8 {
//     let obj_type_addr = mmtk_jl_typeof(obj);
//     let obj_type = obj_type_addr.to_ptr::<mmtk_jl_datatype_t>();

//     if obj_type == jl_symbol_type {
//         return 1;
//     }

//     if BI_METADATA_START_ALIGNED_DOWN == 0 {
//         return 0;
//     }

//     if obj.as_usize() < BI_METADATA_START_ALIGNED_DOWN
//         || obj.as_usize() >= BI_METADATA_END_ALIGNED_UP
//     {
//         return 0;
//     }

//     return check_metadata_scanned(obj);
// }

// #[inline(always)]
// pub unsafe fn boot_image_mark_object_as_scanned(obj: Address) {
//     if BI_METADATA_START_ALIGNED_DOWN == 0 {
//         return;
//     }

//     if obj.as_usize() < BI_METADATA_START_ALIGNED_DOWN
//         || obj.as_usize() >= BI_METADATA_END_ALIGNED_UP
//     {
//         return;
//     }

//     mark_metadata_scanned(obj);
// }

#[inline(always)]
pub fn process_offset_slot<EV: SlotVisitor<JuliaVMSlot>>(
    closure: &mut EV,
    slot: Address,
    offset: usize,
) {
    let offset_slot = OffsetSlot::new_with_offset(slot, offset);
    #[cfg(debug_assertions)]
    {
        use crate::JuliaVM;
        use mmtk::vm::slot::Slot;

        if let Some(objref) = offset_slot.load() {
            debug_assert!(
                mmtk::memory_manager::is_in_mmtk_spaces::<JuliaVM>(objref),
                "Object {:?} in slot {:?} is not mapped address",
                objref,
                offset_slot
            );
        }
    }

    closure.visit_slot(JuliaVMSlot::Offset(offset_slot));
}

#[inline(always)]
pub fn mmtk_jl_array_ndimwords(ndims: u32) -> usize {
    if ndims < 3 {
        return 0;
    }

    return (ndims - 2) as usize;
}

#[inline(always)]
pub unsafe fn mmtk_jl_svec_len(obj: Address) -> usize {
    (*obj.to_ptr::<mmtk_jl_svec_t>()).length
}

#[inline(always)]
pub unsafe fn mmtk_jl_svec_data(obj: Address) -> Address {
    obj + std::mem::size_of::<crate::julia_scanning::mmtk_jl_svec_t>()
}

#[inline(always)]
pub unsafe fn mmtk_jl_array_len(a: *const mmtk_jl_array_t) -> usize {
    (*a).length
}

#[inline(always)]
pub unsafe fn mmtk_jl_array_data_owner_addr(array: *const mmtk_jl_array_t) -> Address {
    Address::from_ptr(array) + mmtk_jl_array_data_owner_offset(mmtk_jl_array_ndims(array))
}

#[inline(always)]
pub unsafe fn mmtk_jl_array_data_owner_offset(ndims: u32) -> usize {
    // (offsetof(jl_array_t,ncols)
    #[allow(deref_nullptr)]
    let offset_ncols =
        &(*(::std::ptr::null::<mmtk_jl_array_t>())).__bindgen_anon_1 as *const _ as usize;

    // (offsetof(jl_array_t,ncols) + sizeof(size_t)*(1+jl_array_ndimwords(ndims))) in bytes
    let res = offset_ncols
        + std::mem::size_of::<::std::os::raw::c_ulong>() * (1 + mmtk_jl_array_ndimwords(ndims));
    res
}

#[inline(always)]
pub unsafe fn mmtk_jl_tparam0(vt: *const mmtk_jl_datatype_t) -> *const mmtk_jl_datatype_t {
    mmtk_jl_svecref((*vt).parameters, 0)
}

#[inline(always)]
pub unsafe fn mmtk_jl_svecref(vt: *mut mmtk_jl_svec_t, i: usize) -> *const mmtk_jl_datatype_t {
    debug_assert!(
        mmtk_jl_typeof(Address::from_mut_ptr(vt)) as usize == jl_simplevector_type as usize
    );
    debug_assert!(i < mmtk_jl_svec_len(Address::from_mut_ptr(vt)));

    let svec_data = mmtk_jl_svec_data(Address::from_mut_ptr(vt));
    let result_ptr = svec_data + i;
    let result = result_ptr.atomic_load::<AtomicUsize>(Ordering::Relaxed);
    ::std::mem::transmute::<usize, *const mmtk_jl_datatype_t>(result)
}

#[inline(always)]
pub unsafe fn mmtk_jl_dt_layout_ptrs(l: *const mmtk_jl_datatype_layout_t) -> Address {
    mmtk_jl_dt_layout_fields(l)
        + (mmtk_jl_fielddesc_size((*l).fielddesc_type()) * (*l).nfields) as usize
}

#[inline(always)]
pub unsafe fn mmtk_jl_dt_layout_fields(l: *const mmtk_jl_datatype_layout_t) -> Address {
    Address::from_ptr(l) + std::mem::size_of::<mmtk_jl_datatype_layout_t>()
}

#[inline(always)]
pub unsafe fn mmtk_jl_fielddesc_size(fielddesc_type: u16) -> u32 {
    debug_assert!(fielddesc_type <= 2);
    2 << fielddesc_type
}

const JL_BT_NON_PTR_ENTRY: usize = usize::MAX;

pub fn mmtk_jl_bt_is_native(bt_entry: *mut mmtk_jl_bt_element_t) -> bool {
    let entry = unsafe { (*bt_entry).__bindgen_anon_1.uintptr };
    entry != JL_BT_NON_PTR_ENTRY
}

pub fn mmtk_jl_bt_entry_size(bt_entry: *mut mmtk_jl_bt_element_t) -> usize {
    if mmtk_jl_bt_is_native(bt_entry) {
        1
    } else {
        2 + mmtk_jl_bt_num_jlvals(bt_entry) + mmtk_jl_bt_num_uintvals(bt_entry)
    }
}

pub fn mmtk_jl_bt_num_jlvals(bt_entry: *mut mmtk_jl_bt_element_t) -> usize {
    debug_assert!(!mmtk_jl_bt_is_native(bt_entry));
    let entry = unsafe { (*bt_entry.add(1)).__bindgen_anon_1.uintptr };
    entry & 0x7
}

pub fn mmtk_jl_bt_num_uintvals(bt_entry: *mut mmtk_jl_bt_element_t) -> usize {
    debug_assert!(!mmtk_jl_bt_is_native(bt_entry));
    let entry = unsafe { (*bt_entry.add(1)).__bindgen_anon_1.uintptr };
    (entry >> 3) & 0x7
}

pub fn mmtk_jl_bt_entry_jlvalue(bt_entry: *mut mmtk_jl_bt_element_t, i: usize) -> ObjectReference {
    let entry = unsafe { (*bt_entry.add(2 + i)).__bindgen_anon_1.jlvalue };
    debug_assert!(!entry.is_null());
    unsafe { ObjectReference::from_raw_address_unchecked(Address::from_mut_ptr(entry)) }
}
