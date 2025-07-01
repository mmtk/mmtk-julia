use crate::api::mmtk_get_obj_size;
use crate::jl_gc_genericmemory_how;
use crate::jl_gc_update_inlined_array;
use crate::julia_scanning::{
    jl_genericmemory_typename, jl_method_type, jl_small_typeof, mmtk_jl_typeof, mmtk_jl_typetagof,
};
use crate::julia_types::*;
use crate::{JuliaVM, JULIA_BUFF_TAG, JULIA_HEADER_SIZE};
use log::*;
use mmtk::util::copy::*;
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::ObjectModel;
use mmtk::vm::*;

pub struct VMObjectModel {}

/// Global logging bit metadata spec
/// 1 bit per object
pub(crate) const LOGGING_SIDE_METADATA_SPEC: VMGlobalLogBitSpec = VMGlobalLogBitSpec::side_first();

use mmtk::util::constants::LOG_MIN_OBJECT_SIZE;
use mmtk::util::metadata::side_metadata::SideMetadataOffset;
use mmtk::util::metadata::side_metadata::SideMetadataSpec;
pub(crate) const HASH_BITS_SPEC: SideMetadataSpec = SideMetadataSpec {
    name: "julia_hash_bits",
    is_global: true,
    offset: SideMetadataOffset::layout_after(
        LOGGING_SIDE_METADATA_SPEC.as_spec().extract_side_spec(),
    ),
    log_num_of_bits: 1,
    log_bytes_in_region: LOG_MIN_OBJECT_SIZE as usize,
};

#[cfg(feature = "record_moved_objects")]
use std::{sync::Mutex, collections::HashMap};
#[cfg(feature = "record_moved_objects")]
lazy_static! {
    static ref COPIED_OBJECTS: Mutex<HashMap<usize, String>> = Mutex::new(HashMap::new());
}

pub(crate) const MARKING_METADATA_SPEC: VMLocalMarkBitSpec =
    VMLocalMarkBitSpec::side_after(LOS_METADATA_SPEC.as_spec());

pub(crate) const LOCAL_PINNING_METADATA_BITS_SPEC: VMLocalPinningBitSpec =
    VMLocalPinningBitSpec::side_after(MARKING_METADATA_SPEC.as_spec());

pub(crate) const LOCAL_FORWARDING_METADATA_BITS_SPEC: VMLocalForwardingBitsSpec =
    VMLocalForwardingBitsSpec::side_after(LOCAL_PINNING_METADATA_BITS_SPEC.as_spec());

/// PolicySpecific mark-and-nursery bits metadata spec
/// 2-bits per object
pub(crate) const LOS_METADATA_SPEC: VMLocalLOSMarkNurserySpec =
    VMLocalLOSMarkNurserySpec::side_first();

impl ObjectModel<JuliaVM> for VMObjectModel {
    const GLOBAL_LOG_BIT_SPEC: VMGlobalLogBitSpec = LOGGING_SIDE_METADATA_SPEC;
    const LOCAL_FORWARDING_POINTER_SPEC: VMLocalForwardingPointerSpec =
        VMLocalForwardingPointerSpec::in_header(-64);

    const LOCAL_PINNING_BIT_SPEC: VMLocalPinningBitSpec = LOCAL_PINNING_METADATA_BITS_SPEC;
    const LOCAL_FORWARDING_BITS_SPEC: VMLocalForwardingBitsSpec =
        LOCAL_FORWARDING_METADATA_BITS_SPEC;

    const LOCAL_MARK_BIT_SPEC: VMLocalMarkBitSpec = MARKING_METADATA_SPEC;
    const LOCAL_LOS_MARK_NURSERY_SPEC: VMLocalLOSMarkNurserySpec = LOS_METADATA_SPEC;
    const UNIFIED_OBJECT_REFERENCE_ADDRESS: bool = false;
    const OBJECT_REF_OFFSET_LOWER_BOUND: isize = 0;

    fn copy(
        from: ObjectReference,
        semantics: CopySemantics,
        copy_context: &mut GCWorkerCopyContext<JuliaVM>,
    ) -> ObjectReference {
        trace!("Attempting to copy object {}", from);

        #[cfg(debug_assertions)]
        unsafe {
            assert!(
                is_object_in_immixspace(&from),
                "We attempted to copy an object {} that is not in immix space",
                from
            );
            let obj_address = from.to_raw_address();
            let mut vtag = mmtk_jl_typetagof(obj_address);
            let mut vtag_usize = vtag.as_usize();
            assert!(
                vtag_usize != JULIA_BUFF_TAG,
                "We attempted to copy a buffer object {} that is not supported",
                from
            );
        }

        let cur_bytes = Self::get_current_size(from);
        let new_bytes = if cfg!(feature = "address_based_hashing") && test_hash_state(from, HASHED)
        {
            unsafe { get_so_object_size(from, STORED_HASH_BYTES) }
        } else {
            cur_bytes
        };
        let from_addr = from.to_raw_address();
        let from_start = Self::ref_to_object_start(from);
        let header_offset = from_addr - from_start;

        let aligned_new_bytes =
            mmtk::util::conversions::raw_align_up(new_bytes, JuliaVM::MIN_ALIGNMENT);
        let mut dst = if test_hash_state(from, UNHASHED) {
            // 8 bytes offset, for 8 bytes header
            copy_context.alloc_copy(from, aligned_new_bytes, 16, 8, semantics)
        } else {
            // 0 bytes offset, for 16 bytes header
            copy_context.alloc_copy(from, aligned_new_bytes, 16, 0, semantics)
        };

        // `alloc_copy` should never return zero.
        debug_assert!(!dst.is_zero());

        let to_obj = if test_hash_state(from, UNHASHED) {
            debug_assert_eq!(cur_bytes, new_bytes);
            let src = from_start;
            unsafe {
                std::ptr::copy_nonoverlapping::<u8>(src.to_ptr(), dst.to_mut_ptr(), cur_bytes);
            }
            let to_obj =
                unsafe { ObjectReference::from_raw_address_unchecked(dst + header_offset) };
            copy_context.post_copy(to_obj, new_bytes, semantics);
            debug_assert!(test_hash_state(to_obj, UNHASHED));
            to_obj
        } else if test_hash_state(from, HASHED) {
            info!(
                "Moving a hashed object {} with size = {}. New size = {}",
                from, cur_bytes, new_bytes
            );
            // if cur_bytes == new_bytes you end up copying the whole src
            // but before you say that dst += STORED_HASH_BYTES so you don't have space
            // in dst to copy src
            debug_assert_eq!(cur_bytes + STORED_HASH_BYTES, new_bytes);
            debug_assert_eq!(header_offset, 8);

            // Store hash
            let hash = from.to_raw_address().as_usize();
            unsafe {
                dst.store::<usize>(hash);
            }
            info!("Store hash {:x} into {}", hash, dst);
            dst += STORED_HASH_BYTES;

            // Copy the object
            let src = from_start;
            unsafe {
                std::ptr::copy_nonoverlapping::<u8>(src.to_ptr(), dst.to_mut_ptr(), cur_bytes);
            }
            let to_obj =
                unsafe { ObjectReference::from_raw_address_unchecked(dst + header_offset) };
            copy_context.post_copy(to_obj, new_bytes, semantics);

            info!("old object {}, new objectt {}", from, to_obj);

            // set_hash_state(from, UNHASHED);
            set_hash_state(to_obj, HASHED_AND_MOVED);
            to_obj
        } else if test_hash_state(from, HASHED_AND_MOVED) {
            info!("Moving a hashed+moved object {}", from);
            debug_assert_eq!(cur_bytes, new_bytes);
            debug_assert_eq!(from.to_raw_address(), from_start + 16usize);
            debug_assert_eq!(header_offset, 16);
            let hash = unsafe { Address::from_usize(get_stored_hash(from)) };
            debug_assert!(
                is_addr_in_immixspace(hash),
                "The stored hash address {} of object {} is not in immix space",
                hash,
                from
            );
            let src = from_start;
            unsafe {
                std::ptr::copy_nonoverlapping::<u8>(src.to_ptr(), dst.to_mut_ptr(), cur_bytes);
            }
            let to_obj =
                unsafe { ObjectReference::from_raw_address_unchecked(dst + header_offset) };
            copy_context.post_copy(to_obj, new_bytes, semantics);
            // set_hash_state(from, UNHASHED);
            set_hash_state(to_obj, HASHED_AND_MOVED);
            to_obj
        } else {
            panic!()
        };

        trace!("Copied object {} into {}", from, to_obj);

        unsafe {
            let vt = mmtk_jl_typeof(from.to_raw_address());

            if (*vt).name == jl_genericmemory_typename {
                jl_gc_update_inlined_array(from.to_raw_address(), to_obj.to_raw_address())
            }
        }

        #[cfg(feature = "record_moved_objects")]
        {
            let mut map = COPIED_OBJECTS.lock().unwrap();
            map.insert(from.to_raw_address().as_usize(), unsafe { crate::julia_scanning::get_julia_object_type(from.to_raw_address()) });
        }

        // zero from_obj (for debugging purposes)
        #[cfg(debug_assertions)]
        {
            use atomic::Ordering;
            unsafe {
                libc::memset(from_start.to_mut_ptr(), 0, cur_bytes);
            }

            Self::LOCAL_FORWARDING_BITS_SPEC.store_atomic::<JuliaVM, u8>(
                from,
                0b10_u8, // BEING_FORWARDED
                None,
                Ordering::SeqCst,
            );
        }

        to_obj
    }

    fn copy_to(_from: ObjectReference, _to: ObjectReference, _region: Address) -> Address {
        unimplemented!()
    }

    fn get_current_size(object: ObjectReference) -> usize {
        if is_object_in_los(&object) {
            unsafe { get_lo_object_size(object) }
        } else if is_object_in_immixspace(&object) {
            unsafe { get_so_object_size(object, get_hash_size(object)) }
        } else if is_object_in_nonmoving(&object) {
            unsafe { get_so_object_size(object, 0) }
        } else {
            // This is hacky but it should work.
            // This covers the cases for immortal space and VM space.
            // For those spaces, we only query object size when we try to find the base reference for an internal pointer.
            // For those two spaces, we bulk set VO bits so we cannot find the base reference at all.
            // We return 0 as the object size, so MMTk core won't find the base reference.
            // As we only use the base reference to pin the objects, we cannot pin the objects. But it is fine,
            // as objects in those spaces won't be moved.
            0
        }
    }

    fn get_size_when_copied(_object: ObjectReference) -> usize {
        unimplemented!()
    }

    fn get_align_when_copied(_object: ObjectReference) -> usize {
        unimplemented!()
    }

    fn get_align_offset_when_copied(_object: ObjectReference) -> usize {
        unimplemented!()
    }

    fn get_reference_when_copied_to(_from: ObjectReference, _to: Address) -> ObjectReference {
        unimplemented!()
    }

    fn get_type_descriptor(_reference: ObjectReference) -> &'static [i8] {
        unimplemented!()
    }

    #[inline(always)]
    fn ref_to_object_start(object: ObjectReference) -> Address {
        if is_object_in_los(&object) {
            object.to_raw_address() - 48
        } else {
            get_object_start_for_potentially_hashed_object(object)
        }
    }

    #[inline(always)]
    fn ref_to_header(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    fn dump_object(_object: ObjectReference) {
        unimplemented!()
    }

    fn binding_global_side_metadata_specs() -> Vec<SideMetadataSpec> {
        vec![HASH_BITS_SPEC]
    }
}

#[inline(always)]
pub fn is_object_in_los(object: &ObjectReference) -> bool {
    // FIXME: get the range from MMTk. Or at least assert at boot time to make sure those constants are correct.
    (*object).to_raw_address().as_usize() >= 0x600_0000_0000
        && (*object).to_raw_address().as_usize() < 0x800_0000_0000
}

#[inline(always)]
pub fn is_object_in_immixspace(object: &ObjectReference) -> bool {
    is_addr_in_immixspace((*object).to_raw_address())
}

#[inline(always)]
pub fn is_addr_in_immixspace(addr: Address) -> bool {
    // FIXME: get the range from MMTk. Or at least assert at boot time to make sure those constants are correct.
    addr.as_usize() >= 0x200_0000_0000 && addr.as_usize() < 0x400_0000_0000
}

#[inline(always)]
pub fn is_object_in_nonmoving(object: &ObjectReference) -> bool {
    // FIXME: get the range from MMTk. Or at least assert at boot time to make sure those constants are correct.
    (*object).to_raw_address().as_usize() >= 0x800_0000_0000
        && (*object).to_raw_address().as_usize() < 0xa00_0000_0000
}

// If an object has its type tag bits set as 'smalltag', but those bits are not recognizable,
// very possibly the object is corrupted. This function asserts this case.
pub fn assert_generic_datatype(obj: Address) {
    unsafe {
        let vtag = mmtk_jl_typetagof(obj);
        let vt = vtag.to_ptr::<jl_datatype_t>();
        let type_tag = mmtk_jl_typetagof(vtag);

        if type_tag.as_usize() != ((jl_small_typeof_tags_jl_datatype_tag as usize) << 4)
            || (*vt).smalltag() != 0
        {
            #[cfg(feature = "record_moved_objects")]
            let old_type = {
                let unknown = "unknown".to_string();
                { let map = COPIED_OBJECTS.lock().unwrap(); map.get(&obj.as_usize()).unwrap_or(&unknown).to_string() }
            };
            #[cfg(not(feature = "record_moved_objects"))]
            let old_type = "not recorded (need record_moved_objects)".to_string();
            panic!(
                "GC error (probable corruption) - !jl_is_datatype(vt) = {}; vt->smalltag = {}, vt = {:?}, it was = {}",
                vt as usize != ((jl_small_typeof_tags_jl_datatype_tag as usize) << 4),
                (*(vtag.to_ptr::<jl_datatype_t>())).smalltag() != 0,
                vt,
                old_type
            );
        }
    }
}

/// This function uses mutable static variables and requires unsafe annotation

#[inline(always)]
pub unsafe fn get_so_object_size(object: ObjectReference, hash_size: usize) -> usize {
    let obj_address = object.to_raw_address();
    let mut vtag = mmtk_jl_typetagof(obj_address);
    let mut vtag_usize = vtag.as_usize();

    if vtag_usize == JULIA_BUFF_TAG {
        debug_assert_eq!(
            hash_size, 0,
            "We should not have a hash size for buffer objects. Found size {} for {}",
            hash_size, object
        );
        return mmtk_get_obj_size(object);
    }

    let with_header_size = |dtsz: usize| -> usize {
        let total_sz = dtsz + JULIA_HEADER_SIZE + hash_size;
        debug_assert!(total_sz <= 2032, "size {} greater than minimum!", total_sz);
        total_sz
    };

    if vtag_usize == ((jl_small_typeof_tags_jl_datatype_tag as usize) << 4)
        || vtag_usize == ((jl_small_typeof_tags_jl_unionall_tag as usize) << 4)
        || vtag_usize == ((jl_small_typeof_tags_jl_uniontype_tag as usize) << 4)
        || vtag_usize == ((jl_small_typeof_tags_jl_tvar_tag as usize) << 4)
        || vtag_usize == ((jl_small_typeof_tags_jl_vararg_tag as usize) << 4)
    {
        // these objects have pointers in them, but no other special handling
        // so we want these to fall through to the end
        vtag_usize = jl_small_typeof[vtag.as_usize() / std::mem::size_of::<Address>()] as usize;
        vtag = Address::from_usize(vtag_usize);
    } else if vtag_usize < ((jl_small_typeof_tags_jl_max_tags as usize) << 4) {
        if vtag_usize == ((jl_small_typeof_tags_jl_simplevector_tag as usize) << 4) {
            let length = (*obj_address.to_ptr::<jl_svec_t>()).length;
            let dtsz = length * std::mem::size_of::<Address>() + std::mem::size_of::<jl_svec_t>();
            return llt_align(with_header_size(dtsz), 16);
        } else if vtag_usize == ((jl_small_typeof_tags_jl_module_tag as usize) << 4) {
            let dtsz = std::mem::size_of::<jl_module_t>();
            return llt_align(with_header_size(dtsz), 16);
        } else if vtag_usize == ((jl_small_typeof_tags_jl_task_tag as usize) << 4) {
            let dtsz = std::mem::size_of::<jl_task_t>();
            return llt_align(with_header_size(dtsz), 16);
        } else if vtag_usize == ((jl_small_typeof_tags_jl_string_tag as usize) << 4) {
            let length = object.to_raw_address().load::<usize>();
            let dtsz = length + std::mem::size_of::<usize>() + 1;
            // NB: Strings are aligned to 8 and not to 16
            return llt_align(with_header_size(dtsz), 8);
        } else {
            let vt = jl_small_typeof[vtag_usize / std::mem::size_of::<Address>()];
            let layout = (*vt).layout;
            let dtsz = (*layout).size as usize;
            return llt_align(with_header_size(dtsz), 16);
        }
    } else {
        assert_generic_datatype(obj_address);
    }

    let obj_type = mmtk_jl_typeof(obj_address);
    let vt = vtag.to_ptr::<jl_datatype_t>();

    assert_eq!(obj_type, vt);
    if (*vt).name == jl_genericmemory_typename {
        let m = obj_address.to_ptr::<jl_genericmemory_t>();
        let how = jl_gc_genericmemory_how(obj_address);
        let res = if how == 0 {
            let layout = (*(mmtk_jl_typetagof(obj_address).to_ptr::<jl_datatype_t>())).layout;
            let mut sz = (*layout).size as usize * (*m).length;
            if (*layout).flags.arrayelem_isunion() != 0 {
                sz += (*m).length;
            }

            let dtsz = llt_align(std::mem::size_of::<jl_genericmemory_t>(), 16);
            llt_align(with_header_size(sz + dtsz), 16)
        } else {
            let dtsz = std::mem::size_of::<jl_genericmemory_t>() + std::mem::size_of::<Address>();
            llt_align(with_header_size(dtsz), 16)
        };

        debug_assert!(res <= 2032, "size {} greater than minimum!", res);

        return res;
    } else if vt == jl_method_type {
        let dtsz = std::mem::size_of::<jl_method_t>();
        debug_assert!(
            with_header_size(dtsz) <= 2032,
            "size {} greater than minimum!",
            with_header_size(dtsz)
        );

        return llt_align(with_header_size(dtsz), 16);
    }

    let layout = (*vt).layout;
    let dtsz = (*layout).size as usize;
    llt_align(with_header_size(dtsz), 16)
}

#[inline(always)]
pub unsafe fn get_lo_object_size(object: ObjectReference) -> usize {
    let obj_address = object.to_raw_address();
    let julia_big_object = (obj_address - std::mem::size_of::<_bigval_t>()).to_ptr::<_bigval_t>();
    return (*julia_big_object).sz;
}

#[inline(always)]
pub unsafe fn get_object_start_ref(object: ObjectReference) -> Address {
    let obj_address = object.to_raw_address();
    let obj_type = mmtk_jl_typeof(obj_address);

    if obj_type as usize == JULIA_BUFF_TAG {
        obj_address - 2 * JULIA_HEADER_SIZE
    } else {
        obj_address - JULIA_HEADER_SIZE
    }
}

// DONT USE THIS FUNCTION ANYWHERE OTHER THAN OBJECT SIZE QUERY, AS IT DOES NOT ALIGN UP AT ALL.
// This function is only used to align up the object size when we query object size.
// However, it seems that we don't need to align up. I am still keeping this function,
// in case that we figure out in the future that we actually need this align up.
// If we are certain that this align up is unnecessary, we can just remove this function.
#[inline(always)]
unsafe fn llt_align(size: usize, _align: usize) -> usize {
    // ((size) + (align) - 1) & !((align) - 1)
    size
}

#[inline(always)]
pub unsafe fn mmtk_jl_is_uniontype(t: *const jl_datatype_t) -> bool {
    mmtk_jl_typetagof(Address::from_ptr(t)).as_usize()
        == (jl_small_typeof_tags_jl_uniontype_tag << 4) as usize
}

// Address based hashing implementation

const UNHASHED: u8 = 0b00;
const HASHED: u8 = 0b01;
const HASHED_AND_MOVED: u8 = 0b10;
const STORED_HASH_BYTES: usize = std::mem::size_of::<usize>();

use mmtk::memory_manager;
use std::sync::atomic::Ordering;

pub fn test_hash_state(object: ObjectReference, state: u8) -> bool {
    let hash_bits = HASH_BITS_SPEC.load_atomic::<u8>(object.to_raw_address(), Ordering::SeqCst);
    hash_bits == state
}

pub fn set_hash_state(object: ObjectReference, state: u8) {
    debug_assert!(cfg!(feature = "address_based_hashing"));
    HASH_BITS_SPEC.store_atomic::<u8>(object.to_raw_address(), state, Ordering::SeqCst);
}

pub fn get_stored_hash(object: ObjectReference) -> usize {
    debug_assert!(test_hash_state(object, HASHED_AND_MOVED));
    unsafe { (object.to_raw_address() - 8 - STORED_HASH_BYTES).load::<usize>() }
}

pub fn get_hash_size(object: ObjectReference) -> usize {
    if cfg!(feature = "address_based_hashing") && test_hash_state(object, HASHED_AND_MOVED) {
        STORED_HASH_BYTES
    } else {
        0
    }
}

pub fn get_object_start_for_potentially_hashed_object(object: ObjectReference) -> Address {
    let obj_start = unsafe { get_object_start_ref(object) };
    if cfg!(feature = "address_based_hashing") {
        if test_hash_state(object, HASHED_AND_MOVED) {
            return obj_start - STORED_HASH_BYTES;
        }
    }
    obj_start
}

#[no_mangle]
pub fn mmtk_get_object_hash(object: ObjectReference) -> usize {
    let obj_addr = object.to_raw_address();
    crate::early_return_for_non_moving_build!(obj_addr.as_usize());

    if !cfg!(feature = "address_based_hashing") {
        crate::api::mmtk_pin_object(object);
        return obj_addr.as_usize();
    }

    if !crate::object_model::is_addr_in_immixspace(obj_addr) {
        return obj_addr.as_usize();
    }

    if test_hash_state(object, HASHED_AND_MOVED) {
        get_stored_hash(object)
    } else {
        set_hash_state(object, HASHED);
        obj_addr.as_usize()
    }
}

#[no_mangle]
pub fn mmtk_get_ptr_hash(ptr: Address) -> usize {
    crate::early_return_for_non_moving_build!(ptr.as_usize());
    if !cfg!(feature = "address_based_hashing") {
        crate::api::mmtk_pin_pointer(ptr);
        return ptr.as_usize();
    }

    if !crate::object_model::is_addr_in_immixspace(ptr) {
        return ptr.as_usize();
    }

    let maybe_objref = memory_manager::find_object_from_internal_pointer(ptr, 32 * 1024); // FIXME: Immix space 32K block. Find out the constant!
    if let Some(objref) = maybe_objref {
        let obj_addr = objref.to_raw_address();
        if test_hash_state(objref, HASHED_AND_MOVED) {
            // info!("Get hash from stored hash for object {}", objref);
            let offset = ptr - obj_addr;
            get_stored_hash(objref) + offset
        } else {
            // info!("Get hash from object {}", objref);
            set_hash_state(objref, HASHED);
            ptr.as_usize()
        }
    } else {
        panic!(
            "Invalid pointer: {}, we cannot find the base object for it",
            ptr
        )
    }
}
