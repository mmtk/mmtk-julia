use crate::api::MMTK_SIDE_VO_BIT_BASE_ADDRESS;
use core::sync::atomic::Ordering;
use mmtk::util::Address;
use std::sync::atomic::AtomicU8;

// This module is a duplicate of MMTK core's side metadata to allow bulk setting for VO bit.
// The problem is that VO bit is internal to MMTk core, and we cannot access VO bit.
// FIXME: We should consider refactoring MMTk core to either expose `SideMetadataSpec` for VO bit,
//        or allow the binding to construct `SideMetadataSpec` for VO bit. For either case, we can
//        remove this module and remove this code duplication.

// Functions to set the side metadata for the VO bit (copied from mmtk-core)
pub const VO_BIT_LOG_NUM_OF_BITS: i32 = 0;
pub const VO_BIT_LOG_BYTES_PER_REGION: usize = mmtk::util::constants::LOG_MIN_OBJECT_SIZE as usize;

pub fn bulk_update_vo_bit(start: Address, size: usize) {
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
        set_meta_bits(meta_start, meta_start_shift, meta_end, meta_end_shift);
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
