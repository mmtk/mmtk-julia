use crate::api::MMTK_SIDE_VO_BIT_BASE_ADDRESS;
use crate::JuliaVM;
use core::sync::atomic::Ordering;
use enum_map::Enum;
use mmtk::util::Address;
use mmtk::util::ObjectReference;
use std::sync::atomic::AtomicU8;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Enum, PartialEq, Hash, Eq)]
pub enum RootLabel {
    MarkAndScan = 0,
    ScanOnly = 1,
    FinList = 2,
    ObjArray = 3,
    Array8 = 4,
    Obj8 = 5,
    Obj16 = 6,
    Obj32 = 7,
    Stack = 8,
    ExcStack = 9,
    ModuleBinding = 10,
}

impl RootLabel {
    pub fn from_u32(value: u32) -> RootLabel {
        match value {
            0 => RootLabel::MarkAndScan,
            1 => RootLabel::ScanOnly,
            2 => RootLabel::FinList,
            3 => RootLabel::ObjArray,
            4 => RootLabel::Array8,
            5 => RootLabel::Obj8,
            6 => RootLabel::Obj16,
            7 => RootLabel::Obj32,
            8 => RootLabel::Stack,
            9 => RootLabel::ExcStack,
            10 => RootLabel::ModuleBinding,
            _ => panic!("Unknown value: {}", value),
        }
    }
}

const PRINT_STRUCT_SIZE: bool = false;

macro_rules! print_sizeof {
    ($t: ty) => {{
        let sz = std::mem::size_of::<$t>();
        if PRINT_STRUCT_SIZE {
            println!("Rust {} = {} bytes", stringify!($t), sz);
        }
        sz
    }};
}

pub(crate) fn get_abi_structs_checksum_rust() -> usize {
    use crate::julia_types::*;
    print_sizeof!(mmtk::Mutator<crate::JuliaVM>)
        ^ print_sizeof!(mmtk__jl_taggedvalue_bits)
        ^ print_sizeof!(mmtk_jl_taggedvalue_t)
        ^ print_sizeof!(mmtk_jl_array_flags_t)
        ^ print_sizeof!(mmtk_jl_datatype_layout_t)
        ^ print_sizeof!(mmtk_jl_typename_t)
        ^ print_sizeof!(mmtk_jl_svec_t)
        ^ print_sizeof!(mmtk_jl_datatype_t)
        ^ print_sizeof!(mmtk_jl_array_t)
        ^ print_sizeof!(mmtk_jl_sym_t)
        ^ print_sizeof!(mmtk_jl_binding_t)
        ^ print_sizeof!(mmtk_htable_t)
        ^ print_sizeof!(mmtk_arraylist_t)
        ^ print_sizeof!(mmtk_jl_uuid_t)
        ^ print_sizeof!(mmtk_jl_mutex_t)
        ^ print_sizeof!(mmtk_jl_module_t)
        ^ print_sizeof!(mmtk_jl_excstack_t)
        ^ print_sizeof!(mmtk_jl_bt_element_t)
        ^ print_sizeof!(mmtk_jl_stack_context_t)
        ^ print_sizeof!(mmtk_jl_ucontext_t)
        ^ print_sizeof!(mmtk__jl_gcframe_t)
        ^ print_sizeof!(mmtk_jl_task_t)
        ^ print_sizeof!(mmtk_jl_weakref_t)
        ^ print_sizeof!(mmtk_jl_tls_states_t)
        ^ print_sizeof!(mmtk_jl_thread_heap_t)
        ^ print_sizeof!(mmtk_jl_thread_gc_num_t)
}

// The functions below allow accessing the values of bitfields without performing a for loop
use crate::julia_types::{__BindgenBitfieldUnit, mmtk__jl_task_t, mmtk_jl_array_flags_t};

// FIXME: this function needs to be updated with the 1.9.2 layout
// impl mmtk_jl_datatype_layout_t {
//     #[inline]
//     pub fn fielddesc_type_custom(&self) -> u16 {
//         let fielddesc_type_raw: u16 = unsafe {
//             ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
//         };
//         fielddesc_type_raw >> 1 & 0b11
//     }
// }

impl mmtk_jl_array_flags_t {
    #[inline]
    pub fn how_custom(&self) -> u16 {
        let how_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        how_raw & 0b11
    }
    #[inline]
    pub fn ndims_custom(&self) -> u16 {
        let ndims_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        ndims_raw >> 2 & 0b111_111_111
    }
    #[inline]
    pub fn ptrarray_custom(&self) -> u16 {
        let ptrarray_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        ptrarray_raw >> 12 & 0b1
    }
    #[inline]
    pub fn hasptr_custom(&self) -> u16 {
        let hasptr_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        hasptr_raw >> 13 & 0b1
    }
}

impl mmtk__jl_task_t {
    #[inline]
    pub fn copy_stack_custom(&self) -> u32 {
        let copy_stack_raw: u32 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 4usize]>, u32>(self._bitfield_1)
        };
        copy_stack_raw & 2147483647u32
    }
}

#[no_mangle]
pub extern "C" fn mmtk_julia_copy_stack_check(c_flag_is_defined: bool) {
    if c_flag_is_defined {
        #[cfg(not(feature = "julia_copy_stack"))]
        panic!("COPY_STACK flag has been defined in C, but `julia_copy_stack` feature has not been set.")
    } else {
        #[cfg(feature = "julia_copy_stack")]
        panic!("COPY_STACK flag has not been defined in C, but `julia_copy_stack` feature has been set.")
    }
}

#[no_mangle]
pub extern "C" fn mmtk_get_possibly_forwared(object: ObjectReference) -> ObjectReference {
    match object.get_forwarded_object::<JuliaVM>() {
        Some(forwarded) => forwarded,
        None => object,
    }
}

// Functions to set the side metadata for the VO bit (copied from mmtk-core)
pub const VO_BIT_LOG_NUM_OF_BITS: i32 = 0;
pub const VO_BIT_LOG_BYTES_PER_REGION: usize = mmtk::util::constants::LOG_MIN_OBJECT_SIZE as usize;

pub fn bulk_update_vo_bit(
    start: Address,
    size: usize,
    update_meta_bits: &impl Fn(Address, u8, Address, u8),
) {
    // Update bits for a contiguous side metadata spec. We can simply calculate the data end address, and
    // calculate the metadata address for the data end.
    let update_contiguous = |data_start: Address, data_bytes: usize| {
        if data_bytes == 0 {
            return;
        }
        let meta_start = address_to_meta_address(data_start);
        let meta_start_shift = meta_byte_lshift(data_start);
        let meta_end = address_to_meta_address(data_start + data_bytes);
        let meta_end_shift = meta_byte_lshift(data_start + data_bytes);
        update_meta_bits(meta_start, meta_start_shift, meta_end, meta_end_shift);
    };

    // VO bit is global
    update_contiguous(start, size);
}

/// Performs the translation of data address (`data_addr`) to metadata address for the specified metadata (`metadata_spec`).
pub fn address_to_meta_address(data_addr: Address) -> Address {
    #[cfg(target_pointer_width = "32")]
    let res = {
        if metadata_spec.is_global {
            address_to_contiguous_meta_address(metadata_spec, data_addr)
        } else {
            address_to_chunked_meta_address(metadata_spec, data_addr)
        }
    };
    #[cfg(target_pointer_width = "64")]
    let res = { address_to_contiguous_meta_address(data_addr) };

    res
}

/// Performs address translation in contiguous metadata spaces (e.g. global and policy-specific in 64-bits, and global in 32-bits)
pub fn address_to_contiguous_meta_address(data_addr: Address) -> Address {
    let rshift = (mmtk::util::constants::LOG_BITS_IN_BYTE as i32) - VO_BIT_LOG_NUM_OF_BITS;

    if rshift >= 0 {
        MMTK_SIDE_VO_BIT_BASE_ADDRESS + ((data_addr >> VO_BIT_LOG_BYTES_PER_REGION) >> rshift)
    } else {
        MMTK_SIDE_VO_BIT_BASE_ADDRESS + ((data_addr >> VO_BIT_LOG_BYTES_PER_REGION) << (-rshift))
    }
}

pub fn meta_byte_lshift(data_addr: Address) -> u8 {
    if VO_BIT_LOG_NUM_OF_BITS >= 3 {
        return 0;
    }
    let rem_shift = mmtk::util::constants::BITS_IN_WORD as i32
        - ((mmtk::util::constants::LOG_BITS_IN_BYTE as i32) - VO_BIT_LOG_NUM_OF_BITS);
    ((((data_addr >> VO_BIT_LOG_BYTES_PER_REGION) << rem_shift) >> rem_shift)
        << VO_BIT_LOG_NUM_OF_BITS) as u8
}

/// This method is used for bulk updating side metadata for a data address range. As we cannot guarantee
/// that the data address range can be mapped to whole metadata bytes, we have to deal with cases that
/// we need to mask and zero certain bits in a metadata byte. The end address and the end bit are exclusive.
/// The end bit for update_bits could be 8, so overflowing needs to be taken care of.
pub fn update_meta_bits(
    meta_start_addr: Address,
    meta_start_bit: u8,
    meta_end_addr: Address,
    meta_end_bit: u8,
    update_bytes: &impl Fn(Address, Address),
    update_bits: &impl Fn(Address, u8, u8),
) {
    // Start/end is the same, we don't need to do anything.
    if meta_start_addr == meta_end_addr && meta_start_bit == meta_end_bit {
        return;
    }

    // zeroing bytes
    if meta_start_bit == 0 && meta_end_bit == 0 {
        update_bytes(meta_start_addr, meta_end_addr);
        return;
    }

    if meta_start_addr == meta_end_addr {
        // Update bits in the same byte between start and end bit
        update_bits(meta_start_addr, meta_start_bit, meta_end_bit);
    } else if meta_start_addr + 1usize == meta_end_addr && meta_end_bit == 0 {
        // Update bits in the same byte after the start bit (between start bit and 8)
        update_bits(meta_start_addr, meta_start_bit, 8);
    } else {
        // update bits in the first byte
        update_meta_bits(
            meta_start_addr,
            meta_start_bit,
            meta_start_addr + 1usize,
            0,
            update_bytes,
            update_bits,
        );
        // update bytes in the middle
        update_meta_bits(
            meta_start_addr + 1usize,
            0,
            meta_end_addr,
            0,
            update_bytes,
            update_bits,
        );
        // update bits in the last byte
        update_meta_bits(
            meta_end_addr,
            0,
            meta_end_addr,
            meta_end_bit,
            update_bytes,
            update_bits,
        );
    }
}

/// This method is used for bulk setting side metadata for a data address range.
pub fn set_meta_bits(
    meta_start_addr: Address,
    meta_start_bit: u8,
    meta_end_addr: Address,
    meta_end_bit: u8,
) {
    let set_bytes = |start: Address, end: Address| {
        set(start, 0xff, end - start);
    };
    let set_bits = |addr: Address, start_bit: u8, end_bit: u8| {
        // we are setting selected bits in one byte
        let mask: u8 = !(u8::MAX.checked_shl(end_bit.into()).unwrap_or(0)) & (u8::MAX << start_bit); // Get a mask that the bits we need to set are 1, and the other bits are 0.
        unsafe { addr.as_ref::<AtomicU8>() }.fetch_or(mask, Ordering::SeqCst);
    };
    update_meta_bits(
        meta_start_addr,
        meta_start_bit,
        meta_end_addr,
        meta_end_bit,
        &set_bytes,
        &set_bits,
    );
}

/// Set a range of memory to the given value. Similar to memset.
pub fn set(start: Address, val: u8, len: usize) {
    unsafe {
        std::ptr::write_bytes::<u8>(start.to_mut_ptr(), val, len);
    }
}
